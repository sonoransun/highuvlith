---
description: >-
  193 nm water immersion (NA up to 1.35), then double and quadruple patterning (LELE, SADP,
  SAQP) and computational lithography (OPC, SMO, ILT): how optical lithography kept scaling
  after the wavelength stopped at 193 nm.
---

# Immersion and multiple patterning

After 157 nm was dropped, lithography stayed at 193 nm for more than fifteen years of
leading-edge manufacturing. Two ideas carried it through. **Water immersion** raised the
numerical aperture above 1, the limit for a dry lens. Once even that ran out, **multiple
patterning** split each dense layer into several exposures, each individually printable.
Around both grew **computational lithography**: masks and illumination designed by
simulation rather than by hand.

## An old idea: put liquid under the lens

Microscopists had done it for more than a century. ZEISS credits Giovanni Battista Amici with
the modern immersion objective, "most probably" in 1847, and Ernst Abbe with homogeneous oil
immersion at Jena in the 1870s [1]. The physics is the same for a lithography lens.
Numerical aperture is NA = *n* sin θ, where *n* is the refractive index of the medium the light
ends in. In air *n* = 1, so NA < 1 however steep the rays. Worse, a ray that is too steep for
air cannot leave the flat last surface of the lens at all: it is totally internally
reflected. Fill the gap with a liquid of index *n* and those rays reach the wafer, and NA can
approach *n*.

<figure markdown="span">
  ![Two panels, each showing the flat-bottomed last lens element above a resist-coated wafer.
  Left, dry: a moderate ray leaves the lens and reaches the focus, but a steeper ray is totally
  internally reflected at the glass-air surface because n sin theta would exceed one. Right,
  immersion: with water between lens and wafer both rays reach the focus, so NA equals n sin
  theta up to about 1.35, and the wavelength inside the water is about 134
  nm.](../assets/images/history/immersion-principle-light.svg#only-light){ width="820" }
  ![Two panels, each showing the flat-bottomed last lens element above a resist-coated wafer.
  Left, dry: a moderate ray leaves the lens and reaches the focus, but a steeper ray is totally
  internally reflected at the glass-air surface because n sin theta would exceed one. Right,
  immersion: with water between lens and wafer both rays reach the focus, so NA equals n sin
  theta up to about 1.35, and the wavelength inside the water is about 134
  nm.](../assets/images/history/immersion-principle-dark.svg#only-dark){ width="820" }
  <figcaption markdown="span">The immersion principle. Ray angles follow Snell's law with n ≈ 1.56 for the
  fused-silica lens and 1.44 for water at 193 nm. Schematic, not to scale; script:
  [make_history_schematics.py](../figures/history/make_history_schematics.py).</figcaption>
</figure>

The idea was patented for lithography early. Werner Tabarelli and Ernst Löbach filed a US
patent on immersion photolithography in October 1980 (granted 1982) [2]. A Hitachi patent by
Akihiro Takanashi and co-workers, with Japanese priority from 1981 and granted in 1984,
reported 0.95 µm lines without liquid and 0.69 or 0.62 µm with liquids of index 1.36 or 1.53
in the gap [3]. In 1989 Kawata, Carter, Yen and Smith at MIT described
"optical projection lithography using lenses with numerical apertures greater than unity"
[4]. In 2001 Switkes and Rothschild at MIT Lincoln Laboratory demonstrated immersion
lithography at 157 nm [5].

## 193 nm and water

The decisive argument was for 193 nm and ordinary water. Water is transparent at 193 nm and has
an index of about 1.44 there, so the wavelength inside it is 193/1.44 ≈ 134 nm, shorter than
157 nm, while the 193 nm lasers, fused-silica lenses, masks and much of the resist knowledge
stay in use [6, 7]. **Burn J. Lin** of TSMC made the case in 2002, working out the resolution
and depth-of-focus scaling for immersion at high NA [6]. On 3 December 2003 TSMC became the first chipmaker to commit to
193 nm immersion, ordering an ASML XT:1250i [8].

The tools arrived fast:

| Year | Tool | NA | Milestone |
|---|---|---|---|
| 2003 | ASML AT:1150i | 0.75 | ASML's first immersion system (prototype) [9] |
| 2004 | ASML XT:1250i | 0.85 | first commercial immersion scanner, first shipped late 2004 [10] |
| 2005–2006 | Nikon NSR-S609B | 1.07 | first scanner with NA above 1; first immersion tool in mass production [10, 11] |
| 2006 | ASML XT:1700i | 1.20 | ASML's first immersion system for volume production [9, 10] |
| 2006–2007 | Nikon NSR-S610C | 1.30 | ≤ 45 nm [10, 11] |
| 2007 | ASML XT:1900i | 1.35 | NA 1.35 [9]; 36.5 nm half-pitch demonstrated, k₁ = 0.255 [12] |
| 2008 | Nikon NSR-S620D, ASML NXT:1950i | 1.35 | ≤ 38 nm single exposure [11, 13] |

NA has stayed at 1.35 ever since: every immersion scanner in the history data set from 2007 to
2023 has it. Lin put the realistic ceiling with water near 1.37, at sin θ ≈ 0.95 [7].
Hyper-NA lenses (NA > 1) also forced new lens forms. Bruning notes that catadioptric designs,
with a few mirrors among the lenses, became essential to keep them to a manageable size [14].

Immersion brought its own problems. A film of water moving with a wafer stage at high speed can
leave bubbles, water stains and particles behind. Lin's 2006 review describes the fixes:
post-exposure soaks, hydrophobic surfaces, careful stage routing and a seal ring at the wafer
edge [7]. In the mid-2000s higher-index fluids and lens materials were studied as a way past
NA 1.5 [14]. Ronse wrote in 2006 that their "economic feasibility is not proven" [15]. They
never reached production.

At these angles polarization matters. For light polarized in the plane of incidence (TM), the
fields of two steeply crossing beams no longer line up, and image contrast drops. Nikon
announced its POLANO polarized illumination in November 2004 and offered it on its dry
NSR-S308F from spring 2005 [16]; hyper-NA immersion
tools such as ASML's XT:1700i then made it standard [27].

Adoption was quick. IBM described a 45 nm process with immersion lithography at IEDM 2006
[17]. Intel's 45 nm process of 2007 was still patterned with dry 193 nm tools, as the title of
its IEDM paper announced [18].

!!! info "Immersion in highuvlith"
    Refractive optics can carry an immersion medium (✅ Implemented).
    `OpticsConfig.immersion(numerical_aperture, immersion_index)` accepts NA up to 0.95 *n*, and
    `OpticsConfig.immersion_193i()` is the ArF water preset: NA 1.35 with *n* = 1.437, close to
    the measured index of water at 193 nm (about 1.44). The defocus phase is evaluated in the
    medium (see [optics](../optics.md)). The
    [vector imaging model](../vector-imaging.md) uses the same image-medium index for the
    direction of every diffraction order, the obliquity factor and the film interface, and
    computes the TE/TM polarization effects that made polarized illumination standard. The
    scalar engine alone is flagged 🔶 above NA ≈ 0.8 in the
    [capability matrix](../capability-matrix.md), because it ignores polarization.

<figure markdown="span">

![Contrast versus pitch for 193 nm water immersion at NA 1.35 with TE, TM and unpolarized light: TE keeps the most contrast, TM the least, unpolarized lies between; the scalar model overestimates all but TE.](../assets/images/sim/site/site-193i-polarization-light.png#gh-light-mode-only)
![Contrast versus pitch for 193 nm water immersion at NA 1.35 with TE, TM and unpolarized light: TE keeps the most contrast, TM the least, unpolarized lies between; the scalar model overestimates all but TE.](../assets/images/sim/site/site-193i-polarization-dark.png#gh-dark-mode-only)

<figcaption>Polarization at NA 1.35 (water): dense 1:1 lines imaged with the vector model for TE, TM and unpolarized illumination, against the scalar model. Near the resolution limit the two first orders meet at steep angles, so TM light (field in the plane of incidence) interferes poorly; production 193i tools use polarized illumination. Thin mask, ideal lens, image in water (no resist film). Models: vector imaging ✅, immersion optics ✅.</figcaption>
</figure>

## Multiple patterning: beyond k₁ = 0.25

Immersion stopped at NA 1.35, and a single exposure cannot print dense lines below
k₁ = 0.25. At 193 nm that means a smallest pitch of about 72 nm, a half-pitch of 36 nm [19].
The workaround was to **split the pattern**:

- **LELE (litho–etch–litho–etch).** Print every other line, etch it into a hard mask, then
  print the lines in between and etch again. Mack's 2008 article sets out the costs: two
  complete pattern transfers "essentially double your lithographic cost", and the result is
  extremely sensitive to overlay. For a 64 nm pitch, pattern placement had to be accurate to
  2–3 nm over a field more than 2 cm across [19]. Litho–freeze variants (LFLE) save the
  intermediate etch [19].
- **SADP (self-aligned, or spacer, double patterning).** Print *mandrels* at twice the target
  pitch, coat them with a conformal film, etch the film back so that only spacers on the
  mandrel sidewalls remain, then remove the mandrels. Two spacers per mandrel halve the pitch,
  and their width is set by deposition rather than by an exposure, so no second overlay is
  involved. The method suits the regular lines of memory arrays better than random logic, and
  Mack expected the big memory makers to use it [19].
- **SAQP.** Repeat the spacer step on the spacers to quarter the pitch. Intel's 10 nm process
  (IEDM 2017) listed self-aligned quad patterning in the title of its paper [20].

<figure markdown="span">
  ![Two rows of cross-section sketches. Top row, LELE: first exposure prints resist lines at
  pitch P on a hard mask; etch transfers them; a second exposure prints lines shifted by P over
  2; a second etch leaves hard-mask lines at pitch P over 2, with an overlay error warning on
  the second exposure. Bottom row, SADP: mandrels at pitch P; a conformal film is deposited;
  the film is etched back leaving spacers on the mandrel sidewalls; the mandrels are removed
  leaving spacers at pitch P over 2; the spacer pattern is transferred into the hard
  mask.](../assets/images/history/multipatterning-lele-sadp-light.svg#only-light){ width="820" }
  ![Two rows of cross-section sketches. Top row, LELE: first exposure prints resist lines at
  pitch P on a hard mask; etch transfers them; a second exposure prints lines shifted by P over
  2; a second etch leaves hard-mask lines at pitch P over 2, with an overlay error warning on
  the second exposure. Bottom row, SADP: mandrels at pitch P; a conformal film is deposited;
  the film is etched back leaving spacers on the mandrel sidewalls; the mandrels are removed
  leaving spacers at pitch P over 2; the spacer pattern is transferred into the hard
  mask.](../assets/images/history/multipatterning-lele-sadp-dark.svg#only-dark){ width="820" }
  <figcaption markdown="span">Pitch splitting. LELE needs two exposures and ties the result to their overlay.
  SADP needs one exposure plus deposition and etch steps, and is self-aligned. Schematic
  cross-sections; script:
  [make_history_schematics.py](../figures/history/make_history_schematics.py).</figcaption>
</figure>

Multiple patterning worked. Bruning reported double patterning imaging at an effective k₁ of
0.14 in 2007 [14]. The cost was more masks, more process steps, layout rules that forced
designs into forms the decomposition could handle, and overlay budgets of a few nanometres.
That cost is what finally made the economic case for EUV: when Samsung started EUV production of
its 7LPP process in 2018, it cited about 20 % fewer masks [21]. See [the road to EUV](euv.md).

!!! info "Multiple patterning in highuvlith"
    Multiple patterning is 🔶 Simplified. LELE runs two engine exposures, thresholds each one
    and takes the union, with a sub-pixel overlay shift. SADP and SAQP are purely geometric:
    line and space widths follow from the mandrel and spacer dimensions, with no deposition or
    etch physics. See [research modules](../research-modules.md) and the
    [capability matrix](../capability-matrix.md).

## Computational lithography

By the immersion era, what the mask looks like and what prints had drifted far apart, and the
mask was designed by simulation:

- **Model-based OPC** became standard from the mid-1990s (see [deep UV](excimer-duv.md)).
- **Source–mask optimization (SMO).** Rosenbluth and co-workers at IBM showed in 2002 how to
  optimize the illumination pupil and the mask pattern together for a given target [22].
- **Inverse lithography technology (ILT)** treats mask design as an inverse problem: find the
  mask, often with curvilinear shapes, whose image best matches the target. Pang's review
  traces it from concept to full-chip use over about thirty years [23]. Curvilinear masks became
  practical with **multi-beam mask writers**; IMS Nanofabrication's MBMW-201 entered the market
  in the first quarter of 2019 for the 5 nm node [24].
- **Scale.** ASML bought the computational-lithography company Brion in 2007 [25]. In March 2023
  NVIDIA announced cuLitho, a GPU library for computational lithography, with TSMC, ASML and
  Synopsys, claiming speed-ups of up to 40× [26].

!!! info "Computational lithography in highuvlith"
    Rule-based and model-based OPC are ✅ Implemented. ILT is ✅ as well: it follows the exact
    adjoint gradient through the engine's imaging kernels, on a continuous pixel mask without
    mask-rule checks or polygon output. SRAF insertion, a rule deck with a model print check,
    is 🔶 Simplified. Joint source–mask optimization is not implemented. See
    [research modules](../research-modules.md).

## Try it

!!! example "Try it in highuvlith: 45 nm half-pitch, dry against water"
    At NA 0.93 a 90 nm pitch is below the single-exposure limit (k₁ = 0.22): no first order
    reaches the pupil. Water immersion at NA 1.35 raises k₁ to 0.31 and the grating prints.

    <!-- verify-example -->
    ```python
    import highuvlith as huv

    source = huv.SourceConfig.arf_laser(sigma=0.9)
    mask = huv.MaskConfig.line_space(cd_nm=45.0, pitch_nm=90.0)
    grid = huv.GridConfig(size=128, pixel_nm=1.40625)  # 180 nm field = 2 periods
    for label, optics in (
        ("dry, NA 0.93", huv.OpticsConfig(numerical_aperture=0.93)),
        ("water, NA 1.35", huv.OpticsConfig.immersion_193i()),  # n = 1.437
    ):
        engine = huv.SimulationEngine(source, optics, mask, grid=grid)
        print(f"{label}: contrast {engine.compute_aerial_image(focus_nm=0.0).image_contrast():.3f}")
    ```

    ??? success "Output"

        ```text
        dry, NA 0.93: contrast 0.000
        water, NA 1.35: contrast 0.200
        ```

<figure markdown="span">

![Contrast versus half-pitch for a dry 157 nm lens at NA 0.85, a dry 193 nm lens at NA 0.93 and 193 nm water immersion at NA 1.35: immersion keeps imaging down to about 38 nm half-pitch, the dry 157 nm lens only to about 49 nm and dry 193 nm to about 55 nm.](../assets/images/sim/site/site-why-immersion-light.png#gh-light-mode-only)
![Contrast versus half-pitch for a dry 157 nm lens at NA 0.85, a dry 193 nm lens at NA 0.93 and 193 nm water immersion at NA 1.35: immersion keeps imaging down to about 38 nm half-pitch, the dry 157 nm lens only to about 49 nm and dry 193 nm to about 55 nm.](../assets/images/sim/site/site-why-immersion-dark.png#gh-dark-mode-only)

<figcaption>The comparison that ended the 157 nm programme, across half-pitch: conventional σ 0.9, default 2 % flare, two-period commensurate fields. Dotted lines mark each tool's k<sub>1</sub> = 0.25 half-pitch; with σ 0.9 the image vanishes slightly above it, at λ/(2NA·1.9). The dashed line is the 45 nm example on the page. Models: scalar Hopkins imaging ✅, immersion optics ✅ (water n = 1.437; no polarization in this chart).</figcaption>
</figure>

!!! example "Try it in highuvlith: why pitch splitting works"
    A 76 nm pitch cannot be printed in one dry ArF exposure (k₁ = 0.18): its image has no
    modulation at all. Each LELE exposure carries only every other line, at twice the pitch
    (k₁ = 0.37), which a dry NA 0.93 lens can image.

    <!-- verify-example -->
    ```python
    import highuvlith as huv

    source = huv.SourceConfig.arf_laser(sigma=0.8)
    optics = huv.OpticsConfig(numerical_aperture=0.93)
    grid = huv.GridConfig(size=256, pixel_nm=1.1875)  # 304 nm field
    for label, pitch in (("target, 76 nm pitch", 76.0), ("one LELE exposure, 152 nm pitch", 152.0)):
        mask = huv.MaskConfig.line_space(cd_nm=38.0, pitch_nm=pitch)
        engine = huv.SimulationEngine(source, optics, mask, grid=grid)
        print(f"{label}: contrast {engine.compute_aerial_image(focus_nm=0.0).image_contrast():.3f}")
    ```

    ??? success "Output"

        ```text
        target, 76 nm pitch: contrast 0.000
        one LELE exposure, 152 nm pitch: contrast 0.198
        ```

<figure markdown="span">

![A 76 nm pitch grating gives a flat image in one dry ArF exposure, while each of two LELE exposures at 152 nm pitch is well modulated; thresholding each and taking the union prints all lines at 76 nm pitch.](../assets/images/sim/site/site-lele-pitch-split-light.png#gh-light-mode-only)
![A 76 nm pitch grating gives a flat image in one dry ArF exposure, while each of two LELE exposures at 152 nm pitch is well modulated; thresholding each and taking the union prints all lines at 76 nm pitch.](../assets/images/sim/site/site-lele-pitch-split-dark.png#gh-dark-mode-only)

<figcaption>Why pitch splitting works (the page's LELE example): one dry ArF exposure of a 76 nm pitch (k<sub>1</sub> = 0.18) has no modulation, but each LELE exposure carries every other line at 152 nm pitch. Each exposure is thresholded at the dose that prints 38 nm lines (dashed) and the two printed patterns are united (bottom). Default 2 % flare; perfect overlay. Models: scalar imaging ✅, multiple patterning 🔶 (constant-threshold resist, no etch).</figcaption>
</figure>

## Key takeaways

- Water immersion (n ≈ 1.44 at 193 nm) lifted NA from 0.93 to 1.35 between 2004 and 2007 and
  gave an effective wavelength of about 134 nm without a new laser or lens material.
- NA 1.35 and k₁ ≈ 0.27 marked the end of single-exposure scaling at 193 nm: about 38 nm
  half-pitch, with 36 nm the theoretical floor.
- Double and quadruple patterning (LELE, SADP, SAQP) pushed the pitch further, at the price of
  cost, process complexity, design restrictions and overlay control.
- Mask design became computational: model-based OPC, source–mask optimization and inverse
  lithography, now running on GPUs and printed with multi-beam mask writers.

## References and further reading

1. ZEISS, ["Oil immersion, refractive index and lens design"](https://www.zeiss.com/microscopy/en/resources/insights-hub/foundational-knowledge/oil-immersion-refractive-index-and-lens-design.html).
2. W. Tabarelli, E. W. Löbach, "Photolithographic method for the manufacture of integrated
   circuits," US patent 4,346,164 (filed 1980, granted 1982),
   [Google Patents](https://patents.google.com/patent/US4346164A/en).
3. A. Takanashi et al. (Hitachi), "Pattern forming apparatus," US patent 4,480,910 (granted 1984),
   [Google Patents](https://patents.google.com/patent/US4480910A/en).
4. H. Kawata, J. M. Carter, A. Yen, H. I. Smith, "Optical projection lithography using lenses
   with numerical apertures greater than unity," *Microelectron. Eng.* **9**, 31–36 (1989),
   [doi:10.1016/0167-9317(89)90008-7](https://doi.org/10.1016/0167-9317(89)90008-7).
5. M. Switkes, M. Rothschild, "Immersion lithography at 157 nm," *J. Vac. Sci. Technol. B*
   **19**, 2353–2356 (2001), [doi:10.1116/1.1412895](https://doi.org/10.1116/1.1412895).
6. B. J. Lin, "The k₃ coefficient in nonparaxial λ/NA scaling equations for resolution, depth of
   focus, and immersion lithography," *J. Microlith. Microfab. Microsyst.* **1**, 7–12 (2002),
   [doi:10.1117/1.1445798](https://doi.org/10.1117/1.1445798).
7. B. J. Lin, "Optical lithography—present and future challenges," *C. R. Physique* **7**,
   858–874 (2006), [doi:10.1016/j.crhy.2006.10.005](https://doi.org/10.1016/j.crhy.2006.10.005).
8. EE Times, ["TSMC is first to commit to 193-nm immersion litho"](https://www.eetimes.com/tsmc-is-first-to-commit-to-193-nm-immersion-litho/) (December 2003).
9. ASML, [company history](https://www.asml.com/en/company/about-asml/history).
10. A. Kato, "Chronology of Lithography Milestones," v0.9 (May 2007),
    [lithoguru.com](https://www.lithoguru.com/scientist/litho_history/Kato_Litho_History.pdf).
11. Nikon, [history of semiconductor lithography systems](https://www.nikon.com/business/semi/history/)
    and [lithography system archive](https://www.nikon.com/business/semi/lineup/archives/).
12. J. de Klerk et al., "Performance of a 1.35NA ArF immersion lithography system for 40-nm
    applications," *Proc. SPIE* **6520**, 65201Y (2007),
    [doi:10.1117/12.712094](https://doi.org/10.1117/12.712094).
13. ASML, ["TWINSCAN: 20 years of lithography innovation"](https://www.asml.com/en/company/stories/2021/twinscan-20-years-innovation) (2021).
14. J. H. Bruning, "Optical lithography … 40 years and holding," *Proc. SPIE* **6520**, 652004
    (2007), [doi:10.1117/12.720631](https://doi.org/10.1117/12.720631).
15. K. Ronse, "Optical lithography—a historical perspective," *C. R. Physique* **7**, 844–857
    (2006), [doi:10.1016/j.crhy.2006.10.007](https://doi.org/10.1016/j.crhy.2006.10.007).
16. Nikon Precision, ["Nikon pushes dry lithography below the 65 nm node with POLANO"](https://www.nikonprecision.com/nikon-pushes-dry-lithography-below-the-65-nm-node-with-polano/) (20 November 2004).
17. P. Agnello et al., "High performance 45-nm SOI technology with enhanced strain, porous low-k
    BEOL, and immersion lithography," *IEDM* (2006),
    [doi:10.1109/IEDM.2006.346879](https://doi.org/10.1109/IEDM.2006.346879).
18. K. Mistry et al., "A 45nm logic technology with high-k+metal gate transistors, strained
    silicon, 9 Cu interconnect layers, 193nm dry patterning, and 100% Pb-free packaging," *IEDM*
    (2007), [doi:10.1109/IEDM.2007.4418914](https://doi.org/10.1109/IEDM.2007.4418914).
19. C. A. Mack, "Seeing double," *IEEE Spectrum* (November 2008),
    [spectrum.ieee.org](https://spectrum.ieee.org/seeing-double).
20. C. Auth et al., "A 10nm high performance and low-power CMOS technology featuring 3rd
    generation FinFET transistors, Self-Aligned Quad Patterning, contact over active gate and
    cobalt local interconnects," *IEDM* (2017),
    [doi:10.1109/IEDM.2017.8268472](https://doi.org/10.1109/IEDM.2017.8268472).
21. Samsung Newsroom, ["Samsung Electronics starts production of EUV-based 7nm LPP process"](https://news.samsung.com/global/samsung-electronics-starts-production-of-euv-based-7nm-lpp-process) (18 October 2018).
22. A. E. Rosenbluth et al., "Optimum mask and source patterns to print a given shape,"
    *J. Microlith. Microfab. Microsyst.* **1**, 13–30 (2002),
    [doi:10.1117/1.1448500](https://doi.org/10.1117/1.1448500).
23. L. Pang, "Inverse lithography technology: 30 years from concept to practical, full-chip
    reality," *J. Micro/Nanopattern. Mater. Metrol.* **20**, 030901 (2021),
    [doi:10.1117/1.JMM.20.3.030901](https://doi.org/10.1117/1.JMM.20.3.030901).
24. IMS Nanofabrication, [company page](https://www.ims.co.at/en/company/).
25. ASML, [company history](https://www.asml.com/en/company/about-asml/history) (Brion acquisition, 2007).
26. NVIDIA newsroom, ["NVIDIA, ASML, TSMC and Synopsys set foundation for next-generation chip manufacturing"](https://nvidianews.nvidia.com/news/nvidia-asml-tsmc-and-synopsys-set-foundation-for-next-generation-chip-manufacturing) (21 March 2023).
27. Phys.org, ["ASML impacts industry roadmap with immersion"](https://phys.org/news/2005-12-asml-impacts-industry-roadmap-immersion.html) (December 2005).
