---
description: >-
  How lithography went from Senefelder's printing stone to High-NA EUV scanners: a timeline
  of the whole arc and the resolution story, R = k₁λ/NA, told with sourced tool data.
---

# History

<p class="huv-lead"><em>Lithography</em> means "writing on stone"; the name comes from a Munich printing process of the 1790s, and the same word now covers the most precise manufacturing step ever put into mass production. This section tells how one became the other in seven chapters, from the first light-hardened coatings to tin-plasma EUV scanners (1796–2025), with the tool data behind every chart. One equation threads the whole story: the smallest half-pitch a projection tool prints is R = k₁·λ/NA, and every generation attacked one of its three factors.</p>

<figure markdown="span">

![Vendor-stated resolution of 75 tool models on a log axis from 2000 to 8 nm against their year from 1973 to 2027, appearing in order of introduction with a year counter and a ticker naming each tool that set a new minimum, from 1000 nm in 1980 to 8 nm with High-NA EUV in 2023.](../assets/images/history/resolution-story-anim-light.svg#gh-light-mode-only)
![Vendor-stated resolution of 75 tool models on a log axis from 2000 to 8 nm against their year from 1973 to 2027, appearing in order of introduction with a year counter and a ticker naming each tool that set a new minimum, from 1000 nm in 1980 to 8 nm with High-NA EUV in 2023.](../assets/images/history/resolution-story-anim-dark.svg#gh-dark-mode-only)

<figcaption><strong>75 tool models with a stated resolution, in order of introduction.</strong> Vendor-stated resolution on a log scale; squares immersion, diamonds EUV, circles dry (open from 2007 on); the ticker names each tool that set a new minimum with its λ, NA and k₁ = R·NA/λ, an indicator, not a definition; a sample, not a catalogue (two Micrascan points are lamp-based). Data: the tool data page. <em>Data chart from litho_tools.csv; no model badge.</em></figcaption>
</figure>

<div class="grid cards huv-cards" markdown>

-   **[From stone to silicon](origins.md)**

    ---

    <span class="huv-swatch huv-swatch--g"></span>1796–1973 · Printing lithography, the first photoresists, the planar process, and contact and proximity printing.

-   **[The projection era](projection-era.md)**

    ---

    <span class="huv-swatch huv-swatch--i"></span>1973–2001 · Scanning projection, the wafer stepper, g-line and i-line lenses, and the move to step-and-scan.

-   **[Excimer lasers & deep UV](excimer-duv.md)**

    ---

    <span class="huv-swatch huv-swatch--krf"></span>1982–2004 · KrF and ArF lasers, chemically amplified resists, and the resolution-enhancement tricks that took k₁ below 0.5.

-   **[The 157 nm detour](157nm-detour.md)**

    ---

    <span class="huv-swatch huv-swatch--f2"></span>1997–2004 · The F₂-laser programme this simulator was first built for, and why water immersion ended it.

-   **[Immersion & multipatterning](immersion-multipatterning.md)**

    ---

    <span class="huv-swatch huv-swatch--193i"></span>2003–2023 · NA above 1, pitch splitting, and computational lithography.

-   **[The road to EUV](euv.md)**

    ---

    <span class="huv-swatch huv-swatch--euv"></span>1986–2025 · From soft-X-ray projection experiments to tin-plasma sources and High-NA EUV.

-   **[Parallel paths](parallel-paths.md)**

    ---

    <span class="huv-swatch huv-swatch--xray"></span>1960–2025 · X-ray proximity printing, LIGA, electron and ion beams, nanoimprint and self-assembly: what survived, and where.

</div>

## The whole arc on one timeline

Nine exposure technologies have carried the pattern since the 1960s, and most of them are still
in use. The ribbon places each one by the first and last tool model of its kind in the data
set, the dated events of the chapters on the rail above, and the seven chapters as shaded
spans beneath the year axis.

<figure markdown="span">

<picture><source media="(max-width: 600px)" srcset="../assets/images/history/era-ribbon-narrow-light.svg"><img class="huv-no-zoom" src="../assets/images/history/era-ribbon-light.svg#gh-light-mode-only" alt="A timeline from 1796 to 2027 with the axis broken at 1960: dated events on a top rail from Senefelder's printing to High-NA EUV, one lane per exposure technology spanning its first to last tool model in the data set, and the seven history chapters as shaded spans beneath the year axis."></picture>
<picture><source media="(max-width: 600px)" srcset="../assets/images/history/era-ribbon-narrow-dark.svg"><img class="huv-no-zoom" src="../assets/images/history/era-ribbon-dark.svg#gh-dark-mode-only" alt="A timeline from 1796 to 2027 with the axis broken at 1960: dated events on a top rail from Senefelder's printing to High-NA EUV, one lane per exposure technology spanning its first to last tool model in the data set, and the seven history chapters as shaded spans beneath the year axis."></picture>

<figcaption><strong>Nine exposure technologies, first to last tool model in the data set.</strong> Band ends are sampling artefacts of an 86-tool sample, not retirement dates; the contact/proximity lane and the events on the top rail are dated in the chapters, and the 157 nm lane is dashed because no production tool shipped. <em>Data chart from litho_tools.csv; no model badge.</em></figcaption>
</figure>

Era by era, the same arc as a table. The last column names the simulator preset that stands
for each era's light, with its grade copied from the [capability
matrix](../capability-matrix.md#light-sources).

<!-- huv-table: stack -->
| Era | First tools | λ | NA (tools in the data) | k₁ (derived) | highuvlith preset |
|---|---|---|---|---|---|
| [Contact and proximity](origins.md) | 1960s | mercury lamp, broadband | no lens | — | not modelled as such: the engine is a projection imager |
| [1:1 scanning projection](projection-era.md) | 1973 | mercury lamp | about 0.16 | — | — |
| [g-line steppers](projection-era.md) | 1978 | 436 nm | 0.28–0.45 | about 0.8 | `SourceConfig.hg_g_line()` ✅ Implemented (🔶 parameters) |
| [i-line steppers](projection-era.md) | 1984 | 365 nm | 0.40–0.63 | 0.4–0.8 | `SourceConfig.hg_i_line()` ✅ Implemented (🔶 parameters) |
| [KrF](excimer-duv.md) | 1988 (R&D), 1995 (scanners) | 248 nm | 0.42–0.86 | 0.85 → 0.31 | `SourceConfig.krf_laser()` ✅ Implemented (🔶 parameters) |
| [ArF dry](excimer-duv.md) | 1998–1999 | 193 nm | 0.60–0.93 | 0.5 → 0.31 | `SourceConfig.arf_laser()` ✅ Implemented (🔶 parameters) |
| [F₂ (abandoned)](157nm-detour.md) | 2003 (prototype) | 157 nm | 0.75 | about 0.48 | `SourceConfig.f2_laser()` ✅ Implemented (🧪 Ar₂ preset) |
| [ArF immersion](immersion-multipatterning.md) | 2004 | 193 nm (134 nm in water) | 0.85–1.35 | 0.31 → 0.27 | `OpticsConfig.immersion_193i()` ✅ Implemented |
| [EUV](euv.md) | 2006 (demo), 2013 (NXE:3300B) | 13.5 nm | 0.25–0.33 | 0.54 → 0.32 | `SourceConfig.lpp_sn_13nm5()` ✅ Implemented (🔶 CE defaults, étendue; 🧪 Gd/Tb power) |
| [High-NA EUV](euv.md) | 2023 | 13.5 nm | 0.55 | about 0.33 | `OpticsConfig.euv_high_na()` 🔶 Simplified |

## One equation, three levers

The smallest half-pitch a projection system can print in a single exposure is usually
written as the Rayleigh-style scaling law

```math
R = k_1\,\frac{\lambda}{\mathrm{NA}},
\qquad
\mathrm{DOF} = k_2\,\frac{\lambda}{\mathrm{NA}^2},
\qquad
\mathrm{NA} = n\,\sin\theta .
```

- **λ, the exposure wavelength.** Lithography went from the mercury g-line (436 nm) to the
  i-line (365 nm), then to KrF (248 nm) and ArF (193 nm) excimer lasers, and finally to
  13.5 nm extreme ultraviolet (EUV) light from a tin plasma.
- **NA, the numerical aperture.** NA measures the widest cone of light the lens accepts: θ is
  the half-angle of that cone at the wafer and *n* the refractive index of the medium the
  light ends in. Lens NA grew from about 0.16 in 1973 to 0.93 for dry lenses. Filling the gap
  under the lens with water (*n* ≈ 1.44 at 193 nm) took it to 1.35.
- **k₁, the process factor.** k₁ lumps together everything else: illumination shape, mask
  type, resist and process control. Rayleigh's classical two-point criterion gives
  k₁ = 0.61. For dense lines and spaces there is a hard floor: a grating of pitch *p* can only
  be imaged if at least two of its diffraction orders get through the pupil, and even with the
  most oblique illumination those orders can be at most 2NA/λ apart. The smallest printable
  pitch in one exposure is therefore λ/(2NA), a half-pitch of λ/(4NA), which is **k₁ = 0.25**.

The depth of focus (DOF) falls as 1/NA², so each gain in NA is paid for in focus budget.
Flatter wafers and tighter focus control had to keep pace. Bruning (2007) puts the practical
DOF of 193 nm tools at about λ/2, roughly 100 nm, which is workable only because chemical
mechanical polishing planarizes the wafer before most lithography steps [1]. The
[playground](../playground/index.md) has a Rayleigh calculator: enter any tool on this page and
read off R and DOF for your own choice of k₁; [lithography arithmetic](../nodes/litho-math.md)
works the same numbers node by node.

## The resolution story in data

The chart below plots 86 exposure-tool models, from the 1973 Perkin-Elmer Micralign to
ASML's 2025 EXE:5200B. It shows each factor in turn and then the k₁ that results. Every
value comes from a cited source (vendor pages and filings, SPIE papers, museum and company
histories); [the tool data](tool-data.md) lists them.

<figure markdown="span">

![Four stacked charts sharing a year axis from 1973 to 2027: exposure wavelength falling from 436 to 13.5 nm, numerical aperture rising to 1.35 in water, vendor-stated resolution falling from 1000 to 8 nm on a log scale, and the derived k1 falling from about 0.8 to 0.27 before EUV restarts near 0.5.](../assets/images/history/resolution-story-light.svg#gh-light-mode-only)
![Four stacked charts sharing a year axis from 1973 to 2027: exposure wavelength falling from 436 to 13.5 nm, numerical aperture rising to 1.35 in water, vendor-stated resolution falling from 1000 to 8 nm on a log scale, and the derived k1 falling from about 0.8 to 0.27 before EUV restarts near 0.5.](../assets/images/history/resolution-story-dark.svg#gh-dark-mode-only)

<figcaption><strong>Three levers, one dot per tool model.</strong> Squares are 193 nm water-immersion scanners, diamonds EUV systems, circles dry tools (open from 2007 on: newer models of older exposure technologies); the two Micrascan points near 250 nm used a filtered mercury-lamp band and the 157 nm point is the Micrascan VII prototype. k₁ = R·NA/λ from the vendor's stated resolution, which vendors define differently, so read the trend. <em>Data chart from litho_tools.csv; no model badge.</em></figcaption>
</figure>

Read top to bottom, the four panels show how the attack moved between the three factors:

1. **Panels 1 and 2, 1978 to about 1995: wavelength and NA together.** The g-line went to the
   i-line and then to KrF, while lens NA rose from 0.28 to about 0.6. k₁ (panel 4) stayed high,
   around 0.6–0.8.
2. **Panel 4, about 1995 to 2008: k₁.** With ArF at 193 nm and a hard NA ceiling approaching,
   most of the gain came from lowering k₁. Off-axis illumination, phase-shift masks, OPC and
   better resists took it from about 0.6 to 0.3. Water immersion then pushed NA past 1, and the
   stated k₁ settled at about 0.27, just above the 0.25 floor.
3. **Panel 3, 2008 to about 2019: a pause in single exposure.** Immersion scanners at NA 1.35
   were specified to 38 nm half-pitch from 2008 on (the flat row of squares). Smaller pitches
   came from splitting each layer into several exposures (multiple patterning).
4. **Panel 1, 2010 onward: a fourteen-fold jump in wavelength.** EUV cut λ from 193 to
   13.5 nm. It restarted near k₁ ≈ 0.5 with NA 0.25–0.33, and the familiar descent began again.
   High-NA EUV (0.55) is the next NA step.

The NA panel deserves its own chart, because the ceiling moved twice: dry lenses stopped just
short of NA 1, water lifted the limit to 1.35, and EUV started over with mirrors.

<figure markdown="span">

![Numerical aperture of each tool model against its year from 1973 to 2025: dry lenses climb from 0.17 to 0.93, water immersion lifts the ceiling to 1.35 in 2007, EUV restarts at 0.25 to 0.33 and High-NA EUV reaches 0.55 in 2023.](../assets/images/history/na-vs-year-light.svg#gh-light-mode-only)
![Numerical aperture of each tool model against its year from 1973 to 2025: dry lenses climb from 0.17 to 0.93, water immersion lifts the ceiling to 1.35 in 2007, EUV restarts at 0.25 to 0.33 and High-NA EUV reaches 0.55 in 2023.](../assets/images/history/na-vs-year-dark.svg#gh-dark-mode-only)

<figcaption><strong>Numerical aperture of the tool models in the data set.</strong> Dry NA climbed until it neared the physical limit of 1; water immersion moved the ceiling to 1.35; EUV started again at low NA with mirror optics. Open circles are dry tools introduced from 2007 on. <em>Data chart from litho_tools.csv; no model badge.</em></figcaption>
</figure>

**The data behind the charts:** all 86 tool models with their year, λ, NA, stated resolution
and sources S1–S41 are on [the tool data](tool-data.md) page, together with how the sample
was assembled and what it is not.

## Old wavelengths never die

A new wavelength does not retire the old ones. The chart below marks each year in which a
tool model in the data set was introduced for a given exposure technology. i-line tools are
still being launched (Nikon NSR-2205iL1 in 2023, ASML's XT:260 packaging scanner in 2025),
and so are KrF (ASML XT:860N, 2021) and dry ArF tools (Nikon NSR-S333F, 2025). A modern
chip is built from dozens of patterned layers, and only the most critical need the newest
tools; the rest can go to older, cheaper ones that still meet their specifications. The
[process-nodes section](../nodes/index.md) follows that mix layer by layer.

<figure markdown="span">

![Timeline lanes, one per exposure technology, with a dot for each tool model introduced: g-line 1978 to 1990, i-line 1984 to 2025, KrF 1988 to 2021, dry ArF 1998 to 2025, immersion ArF 2004 to 2023, EUV 2006 to 2024, High-NA EUV 2023 and 2025.](../assets/images/history/tool-lanes-light.svg#gh-light-mode-only)
![Timeline lanes, one per exposure technology, with a dot for each tool model introduced: g-line 1978 to 1990, i-line 1984 to 2025, KrF 1988 to 2021, dry ArF 1998 to 2025, immersion ArF 2004 to 2023, EUV 2006 to 2024, High-NA EUV 2023 and 2025.](../assets/images/history/tool-lanes-dark.svg#gh-dark-mode-only)

<figcaption><strong>Old wavelengths never die.</strong> When each exposure technology received new tool models, from the same data set: a sample of Nikon, Canon, ASML, GCA and Perkin-Elmer/SVG models, not a catalogue, so a gap means only that no model from those years was included. <em>Data chart from litho_tools.csv; no model badge.</em></figcaption>
</figure>

## What highuvlith models on this tour

**highuvlith began at the far end of this story, as a vacuum-ultraviolet simulator for the
157 nm F₂ laser, and today the same imaging engine takes sources from across the whole arc;
the [157 nm detour](157nm-detour.md) is its origin story.** The ledger below says what each
chapter's physics looks like inside the simulator, with grades copied from the
[capability matrix](../capability-matrix.md), which is the authority on what runs and how
faithfully. It does not model mask topography (3-D/EMF effects) or the mechanics of scanners.

<!-- huv-table: stack -->

| Issue | In highuvlith | Grade | Link |
|---|---|---|---|
| Projection imaging through a lens or mirrors | Hopkins partially coherent imaging, scalar; a separate vector model above NA ≈ 0.8 | ✅ Implemented | [pipeline](../pipeline.md), [vector imaging](../vector-imaging.md) |
| Off-axis illumination (OAI) | conventional, annular, dipole, quadrupole pupil fills | ✅ Implemented | [pipeline](../pipeline.md) |
| Mercury lines and excimer lasers | g/h/i-line, KrF and ArF presets with line shape and photon budget | ✅ Implemented (🔶 parameters) | [DUV/UV heritage](../sources/duv-heritage.md) |
| The 157 nm F₂ laser, where highuvlith began | F₂ preset with Lorentzian line and bandwidth; the Ar₂ 126 nm preset is hypothetical | ✅ Implemented (🧪 Ar₂ preset) | [the 157 nm detour](157nm-detour.md), [VUV excimer](../sources/vuv-excimer.md) |
| Water immersion, NA above 1 | `immersion_193i()`: NA 1.35 in water, defocus phase in the medium | ✅ Implemented | [optics](../optics.md) |
| Multiple patterning | LELE as two thresholded exposures; SADP/SAQP geometric | 🔶 Simplified | [research modules](../research-modules.md) |
| OPC and inverse lithography | rule and model OPC; ILT with the exact adjoint gradient | ✅ Implemented | [research modules](../research-modules.md) |
| Tin-plasma EUV source | in-band spectrum and power chain to the intermediate focus | ✅ Implemented (🔶 CE defaults, étendue; 🧪 Gd/Tb power) | [laser-produced plasma](../sources/lpp.md) |
| EUV projection optics | isotropic pupil with optional obscuration; no anamorphic magnification, no mask 3-D | 🔶 Simplified | [optics](../optics.md) |
| Schwarzschild objective | annular scalar pupil | ✅ Implemented | [optics](../optics.md) |
| Mo/Si multilayer mirrors | Parratt recursion with CXRO/Henke constants | ✅ Implemented | [materials](../materials.md) |
| LIGA deep X-ray shadow printing | spectral depth dose and exposure time through a white beam | ✅ Implemented | [LIGA deep X-ray](../processes/liga-deep-xray.md) |
| Chemically amplified resist bake | acid–quencher reaction–diffusion with illustrative constants | 🔶 Simplified | [resist models](../processes/resist-models.md) |

## Try it in highuvlith

The same 200 nm lines and spaces, imaged by four representative tools from the data set. The
g-line stepper prints no pattern at all: at k₁ ≈ 0.21 neither first diffraction order enters
its pupil, so only uniform background light reaches the wafer. The i-line stepper (k₁ ≈ 0.35)
shows only a faint modulation, because with σ = 0.5 just the rim of the illumination disc
tilts a first order into its pupil.

<!-- verify-example -->
```python
import highuvlith as huv

mask = huv.MaskConfig.line_space(cd_nm=200.0, pitch_nm=400.0)
grid = huv.GridConfig(size=128, pixel_nm=6.25)  # 800 nm field = 2 periods
eras = [
    ("g-line stepper, NA 0.45 (1987)", huv.SourceConfig.hg_g_line(sigma=0.5), 0.45),
    ("i-line stepper, NA 0.63 (1994)", huv.SourceConfig.hg_i_line(sigma=0.5), 0.63),
    ("KrF scanner,    NA 0.68 (1998)", huv.SourceConfig.krf_laser(sigma=0.5), 0.68),
    ("ArF scanner,    NA 0.92 (2004)", huv.SourceConfig.arf_laser(sigma=0.5), 0.92),
]
for label, source, na in eras:
    optics = huv.OpticsConfig(numerical_aperture=na)
    engine = huv.SimulationEngine(source, optics, mask, grid=grid)
    contrast = engine.compute_aerial_image(focus_nm=0.0).image_contrast()
    print(f"{label}: k1 = {200.0 * na / source.wavelength_nm:.2f}, contrast = {contrast:.2f}")
```

???+ success "Output"

    ```text
    g-line stepper, NA 0.45 (1987): k1 = 0.21, contrast = 0.00
    i-line stepper, NA 0.63 (1994): k1 = 0.35, contrast = 0.04
    KrF scanner,    NA 0.68 (1998): k1 = 0.55, contrast = 0.90
    ArF scanner,    NA 0.92 (2004): k1 = 0.95, contrast = 0.99
    ```

<figure markdown="span">

![Aerial-image cross-sections of 200 nm lines on a 400 nm pitch for a g-line, an i-line, a KrF and an ArF tool: the modulation deepens with each generation as k1 grows from 0.21 to 0.95.](../assets/images/sim/site/site-eras-400nm-pitch-light.png#gh-light-mode-only)
![Aerial-image cross-sections of 200 nm lines on a 400 nm pitch for a g-line, an i-line, a KrF and an ArF tool: the modulation deepens with each generation as k1 grows from 0.21 to 0.95.](../assets/images/sim/site/site-eras-400nm-pitch-dark.png#gh-dark-mode-only)

<figcaption><strong>One pitch through four eras.</strong> 200 nm lines and spaces imaged through a g-line (NA 0.45), i-line (NA 0.63), KrF (NA 0.68) and ArF (NA 0.92) tool with conventional σ 0.5 and the default 2 % flare, two periods of the simulated field — the page's example as images. <em>Model: Hopkins imaging ✅ (thin mask, no resist).</em></figcaption>
</figure>

## Key takeaways

- Resolution scales as R = k₁λ/NA. Lithography has improved by shortening λ, raising NA and
  lowering k₁, usually two at a time.
- Dense lines cannot go below k₁ = 0.25 in one exposure. Immersion scanners reached about
  0.27 by 2008; beyond that, pitch splitting and then EUV took over.
- The wavelength fell in steps (436 → 365 → 248 → 193 → 13.5 nm), and every step needed a
  new light source, new resists and new optical materials. The 157 nm step was attempted
  and abandoned.
- Old wavelengths stay in production for the less critical layers; new tool models for
  i-line, KrF and dry ArF were still being introduced in the 2020s.

**Where next:** [From stone to silicon](origins.md) → · Related: [Process nodes](../nodes/index.md) · [The tool data](tool-data.md) · [Capability matrix](../capability-matrix.md)

## References and further reading

1. J. H. Bruning, "Optical lithography … 40 years and holding," *Proc. SPIE* **6520**, 652004
   (2007), [doi:10.1117/12.720631](https://doi.org/10.1117/12.720631). An author's copy is
   hosted on [lithoguru.com](http://www.lithoguru.com/scientist/litho_history/Optical_lithography_40_years_and_holding_Bruning_2007.pdf).
2. K. Ronse, "Optical lithography—a historical perspective," *C. R. Physique* **7**, 844–857
   (2006), [doi:10.1016/j.crhy.2006.10.007](https://doi.org/10.1016/j.crhy.2006.10.007) (open access).
3. B. J. Lin, "Optical lithography—present and future challenges," *C. R. Physique* **7**,
   858–874 (2006), [doi:10.1016/j.crhy.2006.10.005](https://doi.org/10.1016/j.crhy.2006.10.005).
4. C. A. Mack, "Milestones in Optical Lithography Tool Suppliers" (2005),
   [lithoguru.com](https://lithoguru.com/scientist/litho_history/milestones_tools.pdf).
5. A. Kato, "Chronology of Lithography Milestones," version 0.9 (May 2007),
   [lithoguru.com](https://www.lithoguru.com/scientist/litho_history/Kato_Litho_History.pdf).
6. C. A. Mack, "Seeing double," *IEEE Spectrum* (November 2008),
   [spectrum.ieee.org](https://spectrum.ieee.org/seeing-double).
7. Nikon, [history of semiconductor lithography systems](https://www.nikon.com/business/semi/history/);
   ASML, [company history](https://www.asml.com/en/company/about-asml/history).
8. Computer History Museum, [*The Silicon Engine* timeline](https://www.computerhistory.org/siliconengine/).
9. Online Etymology Dictionary, ["lithography"](https://www.etymonline.com/word/lithography).
10. Tool data for the charts: [litho_tools.csv](../figures/history/litho_tools.csv) and
    [make_history_figures.py](../figures/history/make_history_figures.py); sources S1–S41 on
    [the tool data](tool-data.md#sources) page.
