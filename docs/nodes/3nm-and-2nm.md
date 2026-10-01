---
description: "2022–2026: metal pitches in the low 20s of nm push 0.33-NA EUV into double patterning; gate-all-around nanosheets replace FinFETs; power delivery moves to the back of the wafer."
---

# 3 nm and 2 nm: nanosheets, EUV double patterning and backside power

Three things change in this generation. The tightest metal pitches drop into the low 20s of
nanometres, below what 0.33-NA EUV prints reliably in one exposure, so some layers need EUV
double patterning. The FinFET, after a decade, gives way to the gate-all-around nanosheet.
And the power wiring starts moving from the top of the chip to the back of the wafer, which
changes what the front-side metal layers, and therefore lithography, have to do.

## The processes

| Process | Volume production | Transistor | Gate / tightest metal pitch | Notes |
|---|---|---|---|---|
| Samsung SF3E ("3GAE") | "initial production" from 30 June 2022 | GAA nanosheet (MBCFET) | — | first gate-all-around process in production |
| TSMC N3 (N3B) / N3E | N3 announced in volume production 29 Dec 2022; N3E in Q4 2023 | FinFET with FinFlex | N3 gate pitch 45 nm; N3E 48 nm gate, 26 nm fin, 23 nm metal | TSMC's last FinFET generation |
| Samsung SF2 | 2025 | GAA nanosheet (MBCFET) | — | |
| TSMC N2 | volume production in 4Q 2025 | first-generation nanosheet | not disclosed (gate pitch ≈ 48 nm reported) | N2P scheduled for 2H 2026 |
| Intel 18A | high-volume production from late 2025 (Fab 52, Arizona) | RibbonFET (GAA) + PowerVia backside power | ≈ 50 / 32 nm (reported) | first Intel GAA node; 20A was dropped in 2024 |
| TSMC A16 | "planned production in 2026" (TSMC, 2024) | nanosheet + "Super Power Rail" backside power | — | see the [Ångström era](angstrom-era.md) |

Samsung reported that its first 3 nm GAA process cut power by up to 45 %, raised performance by
23 % and reduced area by 16 % relative to its 5 nm process. TSMC kept FinFETs for N3 and used
FinFlex, the ability to mix cells with different numbers of fins on one chip, to trade density
against speed; its IEDM 2022 paper reports a 45 nm contacted gate pitch for N3 and a 23 nm
minimum metal pitch for N3E, the latter enabled, according to TSMC, by a new liner for the
copper lines. TSMC's N2, its first nanosheet process, entered volume production in the fourth
quarter of 2025. Intel skipped from Intel 3 (FinFET) to 18A after announcing in September 2024
that its Arrow Lake processors, planned for 20A, would be built mainly by external partners;
Intel had already demonstrated RibbonFET and PowerVia together on 20A, and 18A reached
high-volume production at its Arizona fab by December 2025.

## Gate-all-around nanosheets

<figure markdown="span">
  ![Four schematic transistor cross-sections cut across the channel: planar, FinFET, gate-all-around nanosheet with three stacked sheets fully surrounded by gate, and CFET.](../assets/images/nodes/transistor-evolution-light.svg#only-light){ width="780" }
  ![Four schematic transistor cross-sections cut across the channel: planar, FinFET, gate-all-around nanosheet with three stacked sheets fully surrounded by gate, and CFET.](../assets/images/nodes/transistor-evolution-dark.svg#only-dark){ width="780" }
  <figcaption>The nanosheet (third panel) surrounds each silicon sheet with gate on all
  four sides. Schematic.</figcaption>
</figure>

A nanosheet transistor is built from a stack of alternating silicon and silicon-germanium
layers (IEEE Spectrum's article on imec's roadmap walks through the flow). After the stack is
patterned into narrow ribbons, the SiGe is etched away selectively, leaving suspended silicon
sheets that the gate dielectric and metal then wrap completely. Two things improve on the
FinFET: the gate controls each sheet from all sides, and the sheet *width* is a design variable
again, instead of the whole-fin quantization of FinFETs. The IRDS 2024 roadmap assumes three
stacked sheets per device in 2025 and four in 2027; its 2022 lithography table lists a
lateral-GAA pitch of 26 nm (2025) and 24 nm (2028) with a minimum width of 7 and 6 nm, and a CD
control target of 0.7 nm (3σ) in 2025.

For lithography the transistor change matters less than it sounds: the gate and sheet
patterns are still regular gratings, well suited to spacer patterning, and the gate pitch
barely moves (45–50 nm across these processes). The squeeze is in the wiring above.

## EUV at its limit: metal pitches of 20–24 nm

At 23 nm pitch a single 0.33-NA EUV exposure runs at k₁ = 0.28; at 20 nm, at 0.24, below the
optical floor ([arithmetic](litho-math.md)). In practice the limit comes earlier: volume
production has printed about 30–36 nm pitch in one EUV exposure, with stochastic failures
setting the limit (see the [7 nm and 5 nm page](7-and-5nm.md)). The consequence is visible in
TSMC's own process history: for N3E, WikiChip's analysis reports that three critical layers that
required EUV double patterning in the first N3 were replaced by single EUV exposures, and N3E
also relaxed the gate pitch (48 versus 45 nm) and the SRAM cell (0.021 versus 0.0199 µm²). Analysts estimated about 25 EUV layers for the first N3 and about 19 for
N3E (TSMC does not publish the count).

The IRDS roadmap lists "193i, EUV DP" (EUV double patterning) as the patterning technology for
the tightest interconnect in 2022 and 2025, and its 2022 lithography table names "EUV 0.33 NA
multiple patterning" as the primary option for those years, with 0.55-NA single patterning
arriving later. EUV double patterning takes the same forms as in the immersion era, one
generation later: pitch-split LELE (each mask at twice the pitch: k₁ ≈ 0.56 for a 23 nm final
pitch), EUV-printed mandrels for SADP, and self-aligned hybrids of the two. The same trade-offs
return too: LELE spends overlay on the spaces, spacer schemes need cuts. What changed is the
scale: the IRDS overlay target for this generation is about 2–2.4 nm (3σ), a fifth of a
10–12 nm half-pitch.

<figure markdown="span">

![Aerial-image cross-sections: on 193 nm immersion an 80 nm pitch keeps only a faint modulation and a 64 nm pitch none; at 24 nm pitch the 0.33 NA EUV image is weak while the 0.55 NA image is strongly modulated.](../assets/images/sim/site/site-node-cross-sections-light.png#gh-light-mode-only)
![Aerial-image cross-sections: on 193 nm immersion an 80 nm pitch keeps only a faint modulation and a 64 nm pitch none; at 24 nm pitch the 0.33 NA EUV image is weak while the 0.55 NA image is strongly modulated.](../assets/images/sim/site/site-node-cross-sections-dark.png#gh-dark-mode-only)

<figcaption>Where single exposure runs out, as images (two periods each, x in units of the pitch). (a) 193i with a conventional σ 0.9 source: the 80 nm pitch is just above the λ/(NA·1.9) cut-off and keeps little contrast, 64 nm has none. (b) 24 nm pitch on the 0.33 and 0.55 NA EUV presets. Production uses dipole-like illumination near these limits (see the companion chart). Thin mask, no resist. Models: scalar Hopkins imaging ✅, immersion ✅, EUV projection optics 🔶.</figcaption>
</figure>

## Backside power delivery

<figure markdown="span">
  ![Two schematic cross-sections. Front-side power delivery: power and signal wires share one metal stack above the transistors, and supply current crosses the whole stack. Backside power delivery: the wafer is thinned, a power network on the back connects through nano through-silicon vias, and the front metal stack carries only signals.](../assets/images/nodes/backside-power-light.svg#only-light){ width="760" }
  ![Two schematic cross-sections. Front-side power delivery: power and signal wires share one metal stack above the transistors, and supply current crosses the whole stack. Backside power delivery: the wafer is thinned, a power network on the back connects through nano through-silicon vias, and the front metal stack carries only signals.](../assets/images/nodes/backside-power-dark.svg#only-dark){ width="760" }
  <figcaption>Front-side versus backside power delivery. Schematic, not to scale.</figcaption>
</figure>

In every process up to this generation, power reaches the transistors from the top, through
the same stack of metal layers that carries the signals: the supply current crosses the thin,
resistive lowest levels, and the power rails occupy tracks inside every standard cell. Backside
power delivery (BSPDN) bonds the wafer face-down to a carrier after the front-side wiring is
built, thins away almost all of the original silicon, and builds a dedicated power network on
the back, connected to the devices through nano through-silicon vias or direct backside
contacts. As IEEE Spectrum summarises it, "backside power removes all the power-delivering
interconnects to beneath the silicon."

Intel tested the idea first on a non-product test chip, an Intel 4 core with PowerVia reported
at VLSI 2023; Intel summarised the result as ">5% frequency improvement and >90% cell
density" (cell utilisation). Intel's
18A pairs its RibbonFET transistors with PowerVia; TSMC's A16 adds its "Super Power Rail". The
IRDS 2024 roadmap describes the sequence: in the first introduction (2025) backside metal
connects to buried rails or local interconnect; from 2027 it contacts the transistor terminals
directly, "to further reduce the cell height", because "providing power from the backside
metallization will free up the standard cell from the power rails".

For lithography this cuts both ways. The front-side metal no longer has to carry wide,
low-resistance power lines in its tightest levels, which frees routing tracks and gives
designers the option of relaxing the most crowded layers. Intel 18A is a case in point: its
reported tightest metal pitch (about 32 nm) is slightly looser than Intel 4's (30 nm) and far
looser than N3E's (23 nm), even though it is the newer, denser process. At the same time the
back of the wafer gains its own patterned layers, which must be aligned through a bonded,
thinned wafer to transistors on the other side: a new overlay problem.

## Try it in highuvlith

!!! example "24 nm pitch: EUV single exposure, EUV LELE, or High-NA"
    Compare one 0.33-NA exposure of a 24 nm pitch grating (k₁ = 0.29), one mask of an EUV
    LELE split (48 nm pitch, k₁ = 0.59) and one 0.55-NA exposure (k₁ = 0.49). The High-NA
    preset includes a central obscuration of the pupil, an assumed representative value.

    <!-- verify-example -->
    ```python
    import highuvlith as huv

    src = huv.SourceConfig.lpp_sn_13nm5(0.9)
    cases = [("EUV 0.33, single exposure, 24 nm", huv.OpticsConfig.euv_nxe(), 24.0),
             ("EUV 0.33, one LELE mask, 48 nm", huv.OpticsConfig.euv_nxe(), 48.0),
             ("High-NA 0.55, single exposure, 24 nm", huv.OpticsConfig.euv_high_na(), 24.0)]
    for label, opt, pitch in cases:
        mask = huv.MaskConfig.line_space(12.0, pitch)            # 12 nm lines
        eng = huv.SimulationEngine(src, opt, mask, grid=huv.GridConfig(64, pitch / 16))
        print(f"{label:38s} contrast {eng.compute_aerial_image(focus_nm=0.0).image_contrast():.2f}")
    ```

    ??? success "Output"

        ```text
        EUV 0.33, single exposure, 24 nm       contrast 0.11
        EUV 0.33, one LELE mask, 48 nm         contrast 0.54
        High-NA 0.55, single exposure, 24 nm   contrast 0.74
        ```

The comparison is purely optical: the LELE row shows why the split makes each exposure easy,
not the overlay cost of recombining them (highuvlith's LELE module, 🔶, adds the second
exposure with an overlay shift; see the [research modules](../research-modules.md) page).

## Key takeaways

- Gate-all-around nanosheets entered production with Samsung's 3 nm (2022) and reached TSMC
  (N2, 4Q 2025) and Intel (18A, 2025); TSMC's N3 was its last leading-edge FinFET.
- Tightest metal pitches of about 20–24 nm are beyond reliable 0.33-NA EUV single exposure;
  TSMC's first N3 double-patterned some critical layers, and the roadmap answer for these years
  is EUV double patterning, with High-NA single exposure to follow.
- Backside power delivery (Intel PowerVia in 18A, TSMC Super Power Rail in A16) removes power
  rails from the front-side cells and can relax the tightest front-side metal, while adding
  patterned layers on the back of the wafer.

## References and further reading

- Samsung, "[Samsung Begins Chip Production Using 3nm Process Technology With GAA Architecture](https://news.samsung.com/global/samsung-begins-chip-production-using-3nm-process-technology-with-gaa-architecture)"
  (30 June 2022).
- TSMC technology pages: [3nm](https://www.tsmc.com/english/dedicatedFoundry/technology/logic/l_3nm)
  ("In 2022, TSMC became the first foundry to move 3nm FinFET (N3) technology into high-volume
  production"), [2nm](https://www.tsmc.com/english/dedicatedFoundry/technology/logic/l_2nm)
  ("TSMC's 2nm (N2) technology has started volume production in 4Q25 as planned"; N2P in
  2H 2026), [A16](https://www.tsmc.com/english/dedicatedFoundry/technology/logic/l_A16); press
  release "[TSMC Celebrates 30th North America Technology Symposium with Innovations Powering AI with Silicon Leadership](https://pr.tsmc.com/english/news/3136)"
  (24 April 2024, A16).
- S.-Y. Wu et al., "A 3nm CMOS FinFlex™ Platform Technology with Enhanced Power Efficiency and
  Performance for Mobile SoC and High Performance Computing Applications," *IEDM* 2022,
  [doi:10.1109/IEDM45625.2022.10019498](https://doi.org/10.1109/IEDM45625.2022.10019498);
  WikiChip Fuse, "TSMC N3, And Challenges Ahead" (May 2023,
  [link](https://fuse.wikichip.org/news/7375/tsmc-n3-and-challenges-ahead/)): N3E pitches, the
  three layers moved from double patterning to single EUV, EUV layer estimates.
- G. Yeap et al., "2nm Platform Technology Featuring Energy-Efficient Nanosheet Transistors and
  Interconnects Co-Optimized with 3DIC for AI, HPC and Mobile SoC Applications," *IEDM* 2024,
  [doi:10.1109/IEDM50854.2024.10873475](https://doi.org/10.1109/IEDM50854.2024.10873475).
- J. Jeong et al., "World's First GAA 3nm Foundry Platform Technology (SF3) with Novel
  Multi-Bridge-Channel-FET (MBCFET™) Process," *Symp. VLSI Technology and Circuits* 2023,
  [doi:10.23919/VLSITechnologyandCir57934.2023.10185353](https://doi.org/10.23919/VLSITechnologyandCir57934.2023.10185353).
- Intel, "[Continued Momentum for Intel 18A](https://www.intel.com/content/www/us/en/newsroom/opinion/continued-momentum-intel-18a.html)"
  (4 September 2024); CNBC, 19 December 2025, on 18A high-volume production at Fab 52
  ([link](https://www.cnbc.com/2025/12/19/intel-aims-to-find-clients-and-catch-tsmc-with-new-chip-fab-in-arizona.html)).
- X. Wang et al., "A 0.021 µm² High-Density SRAM in Intel-18A-RibbonFET Technology with
  PowerVia-Backside Power Delivery," *ISSCC* 2025,
  [doi:10.1109/ISSCC49661.2025.10904657](https://doi.org/10.1109/ISSCC49661.2025.10904657).
- W. Hafez et al., "Intel PowerVia Technology: Backside Power Delivery for High Density and
  High-Performance Computing," *Symp. VLSI Technology and Circuits* 2023,
  [doi:10.23919/VLSITechnologyandCir57934.2023.10185208](https://doi.org/10.23919/VLSITechnologyandCir57934.2023.10185208);
  Intel newsroom, 5 June 2023, on the PowerVia test chip
  ([link](https://www.intel.com/content/www/us/en/newsroom/news/powervia-intel-achieves-chipmaking-breakthrough.html)).
- IEEE Spectrum on backside power delivery ([link](https://spectrum.ieee.org/backside-power-delivery));
  E. Beyne, A. Jourdain, G. Beyer, "Nano-Through Silicon Vias (nTSV) for Backside Power Delivery
  Networks (BSPDN)," *Symp. VLSI Technology and Circuits* 2023,
  [doi:10.23919/VLSITechnologyandCir57934.2023.10185227](https://doi.org/10.23919/VLSITechnologyandCir57934.2023.10185227).
- IEEE IRDS, *2024 Edition: More Moore*
  ([pdf](https://irds.ieee.org/images/files/pdf/2024/2024IRDS_MM.pdf),
  [tables](https://irds.ieee.org/images/files/pdf/2024/2024IRDS_MM_Tables.xlsx)) and
  *2022 Edition: Lithography* tables
  ([xlsx](https://irds.ieee.org/images/files/pdf/2022/2022IRDS_Litho_Tables.xlsx)).
- S. K. Moore, "The Next 15 Years of Moore's Law, According to Imec," *IEEE Spectrum*,
  19 May 2026 ([link](https://spectrum.ieee.org/semiconductor-technology-roadmap)): the
  Si/SiGe nanosheet flow.
- Wikipedia: "[3 nm process](https://en.wikipedia.org/wiki/3_nm_process)" and
  "[2 nm process](https://en.wikipedia.org/wiki/2_nm_process)" (production chronology,
  Samsung's June 2022 figures, Intel 18A pitches as reported from its VLSI 2025 paper).
