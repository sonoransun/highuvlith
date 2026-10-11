---
description: >-
  The road to extreme-ultraviolet lithography: soft-X-ray projection experiments in the
  1980s, multilayer mirrors, the EUV LLC and European and Japanese consortia, tin-plasma sources,
  the first production scanners, and High-NA EUV.
---

# The road to EUV

<p class="huv-lead">Chapter 6 of 7 — The first images at extreme-ultraviolet wavelengths were shown in 1986; chips made with EUV reached consumers in 2019, and High-NA systems began shipping in 2023 (1986–2025) [1, 2]. In between, the industry had to solve one of the hardest engineering problems it has taken on. At 13.5 nm everything absorbs, including glass, air and the photoresist itself, so an EUV scanner is made entirely of mirrors, runs in vacuum, and gets its light from droplets of molten tin turned into plasma 50,000 times a second [3].</p>

<figure markdown="span">

<picture><source media="(max-width: 600px)" srcset="../assets/images/diagrams/spectrum-strip-ribbon-euv-narrow-light.svg"><img class="huv-no-zoom" src="../assets/images/diagrams/spectrum-strip-ribbon-euv-light.svg#gh-light-mode-only" alt="The spectrum strip from 500 to 0.05 nm with the 13.5 nm EUV span highlighted and labelled 'You are here'; the other lines, optics and era pills are faded."></picture>
<picture><source media="(max-width: 600px)" srcset="../assets/images/diagrams/spectrum-strip-ribbon-euv-narrow-dark.svg"><img class="huv-no-zoom" src="../assets/images/diagrams/spectrum-strip-ribbon-euv-dark.svg#gh-dark-mode-only" alt="The spectrum strip from 500 to 0.05 nm with the 13.5 nm EUV span highlighted and labelled 'You are here'; the other lines, optics and era pills are faded."></picture>

<figcaption><strong>You are here on the axis of light.</strong> The chapter's wavelengths and eras at full strength, the rest faded; years are first tool models in litho_tools.csv; the log axis is expanded above 100 nm. <em>Data chart from litho_tools.csv; no model badge.</em></figcaption>
</figure>

<figure markdown="span">

![Schematic EUV scanner: a CO2 laser hits tin droplets, an ellipsoidal collector focuses the 13.5 nm light to an intermediate focus, facet mirrors shape it onto a reflective mask at oblique incidence, and six projection mirrors demagnify the pattern four times onto the wafer, all in vacuum.](../assets/images/history/euv-optical-path-light.svg#gh-light-mode-only)
![Schematic EUV scanner: a CO2 laser hits tin droplets, an ellipsoidal collector focuses the 13.5 nm light to an intermediate focus, facet mirrors shape it onto a reflective mask at oblique incidence, and six projection mirrors demagnify the pattern four times onto the wafer, all in vacuum.](../assets/images/history/euv-optical-path-dark.svg#gh-dark-mode-only)

<figcaption><strong>An EUV scanner, schematically.</strong> Tin-plasma source and collector, faceted illuminator, reflective mask at oblique incidence, six-mirror 4× projection optics and wafer, all in vacuum; real tools differ in geometry and use more illuminator mirrors. <em>Schematic, not to scale.</em></figcaption>
</figure>

## Why 13.5 nm means mirrors, 1972–1985

No material is transparent enough at 13.5 nm to make a lens, and single surfaces reflect almost
nothing at near-normal incidence. The way out is a **multilayer mirror**, a stack of alternating
thin films whose weak reflections add in phase like X-rays in a crystal. Eberhard Spiller at IBM
reported such soft-X-ray multilayer reflectors in 1972 [4]. James Underwood and Troy Barbee
worked out the theory of "layered synthetic microstructures" in 1981 [5], and in 1985 Barbee and
co-workers demonstrated molybdenum–silicon (Mo/Si) multilayers for the extreme ultraviolet [6].

<p class="huv-pull"><strong>1.4 %</strong> of the light is left after a dozen reflections from 70 % mirrors</p>

Mo/Si remains the standard. An ASML EUV mirror has more than 100 layers and is polished to a
smoothness below one atom's thickness; the largest mirrors are 1 m across [7]. The best
reflectance is around 70 %, and 13.5 nm sits near the peak for Mo/Si. As Jeff Hecht's history
of the field puts it, "after a dozen reflections from a 70 percent reflective mirror, you wind
up with about 1.4 percent of the light left" [1]. Most of the source's hard-won power never
reaches the wafer.

<figure markdown="span">

![A staircase of twelve light bars shrinking by 30 percent at each multilayer reflection, from the collector through four illuminator mirrors, the mask and six projection mirrors, ending at 1.4 percent; a dotted line shows the 64 percent La/B record falling to 0.5 percent.](../assets/images/history/mirror-staircase-light.svg#gh-light-mode-only)
![A staircase of twelve light bars shrinking by 30 percent at each multilayer reflection, from the collector through four illuminator mirrors, the mask and six projection mirrors, ending at 1.4 percent; a dotted line shows the 64 percent La/B record falling to 0.5 percent.](../assets/images/history/mirror-staircase-dark.svg#gh-dark-mode-only)

<figcaption><strong>Twelve reflections at 70 %: 1.4 % of the light left.</strong> 70 % is the cited best case and twelve is the count the EUV chapter and this staircase use; the Beyond-EUV reflectance chart and the throughput model's chain count eleven, ten mirrors and the mask (the throughput model also takes the mask at 0.65, a stated assumption); the dotted line is the 64.1 % La/B record at 6.7 nm. <em>Model: EUV/BEUV multilayer mirrors ✅.</em></figcaption>
</figure>

The 70 % figure and the narrow band around 13.5 nm both follow from the Bragg condition of
the stack: a 40-bilayer Mo/Si mirror with a 6.9 nm period reflects a band about 0.6 nm wide,
and its reflectance falls with the angle of incidence, faster for p-polarized light.

<figure markdown="span">

![Three reflectance charts: a Mo/Si multilayer peaking near 73 percent at 13.5 nm with a 0.6 nm bandwidth; La/B4C and La/B multilayers near 6.7 nm with much narrower peaks; and Mo/Si reflectance falling with angle of incidence, faster for p polarization.](../assets/images/sim/materials-multilayer-light.png#gh-light-mode-only)
![Three reflectance charts: a Mo/Si multilayer peaking near 73 percent at 13.5 nm with a 0.6 nm bandwidth; La/B4C and La/B multilayers near 6.7 nm with much narrower peaks; and Mo/Si reflectance falling with angle of incidence, faster for p polarization.](../assets/images/sim/materials-multilayer-dark.png#gh-dark-mode-only)

<figcaption><strong>Multilayer mirrors from Parratt recursion with CXRO/Henke constants.</strong> (a) Mo/Si 40 × 6.9 nm (Mo fraction 0.4, ideal interfaces) against the best reported measured reflectance; (b) La/B<sub>4</sub>C and La/B with periods tuned to 6.7 / 6.65 nm sit well above the 64.1 % La/B record (Opt. Lett. 40, 3778, 2015) because interdiffusion and roughness are off; (c) Mo/Si at 13.5 nm against the angle of incidence, s and p. <em>Model: EUV/BEUV multilayer mirrors ✅ (matches the CXRO calculator within 3·10⁻³; no interdiffusion layers by default).</em></figcaption>
</figure>

**What highuvlith models here: multilayer mirrors are computed by Parratt recursion from
CXRO/Henke optical constants, with ideal interfaces by default, so the 73 % peak is the ideal
value and measured Mo/Si mirrors reach about 70 %.**

<!-- huv-table: stack -->

| Issue | In highuvlith | Grade | Link |
|---|---|---|---|
| Mo/Si and La/B multilayer reflectance against wavelength and angle | Parratt recursion with Névot–Croce roughness, s/p, capping, period tuning; ideal Mo/Si 73.0 % at 13.48 nm | ✅ Implemented | [materials](../materials.md) |
| The mirror train in the image | an angle-dependent multilayer pupil, opt-in, from user-supplied incidence-angle maps | 🔶 Simplified | [optics](../optics.md) |

## Soft-X-ray projection lithography, 1984–1993

Hiroo Kinoshita at NTT began thinking about reduction lithography with X-rays in the
mid-1980s. Underwood and Barbee's multilayer paper convinced him that normal-incidence mirrors
could make it work. He built a demonstration and reported the first projected EUV images at the
Japan Society of Applied Physics meeting in 1986. "Unfortunately, the audience was highly
skeptical of my talk," he said later. "However, my belief did not change" [1]. His 1989 paper
describes reduction imaging at 4.5–13 nm with multilayer-coated **Schwarzschild** optics, a
reflective mask and synchrotron light [8].

In the United States the work started at Lawrence Livermore and Bell Labs. Hawryluk and
Seppala at LLNL published a design for "soft x-ray projection lithography using an x-ray
reduction camera" in 1988 [9]. In 1990 Bjorkholm and co-workers at AT&T Bell Labs printed
features smaller than 0.1 µm by reduction imaging at 14 nm with multilayer-coated optics [10]. A
year later Sandia and AT&T showed diffraction-limited imaging with a laser-produced plasma
source in place of the synchrotron [11].

Around 1993 a Bell Labs manager, Rick Freeman, proposed renaming the technology "extreme
ultraviolet lithography, which sounds like deep ultraviolet lithography", and "people
immediately jumped on that" [1]. Kinoshita organized a US–Japan EUV meeting of about 50
researchers in the same year [1].

Not everyone was convinced. In 1993 the proximity-X-ray pioneers Henry Smith and Mark
Schattenburg wrote: "We assert that projection XRL using multilayer mirrors at 13 nm can never
match the present performance of proximity XRL" [12]. Proximity X-ray lithography is covered on
[parallel paths](parallel-paths.md).

**What highuvlith models here: the two-mirror, centrally obscured Schwarzschild objective of
these first experiments is one of the simulator's optical systems; production scanners use
six-mirror off-axis designs, which the simulator represents with separate, simplified EUV
projection pupils.** The design is still in use for research: the MET5 micro-exposure tool at
Berkeley Lab's CXRO uses a 0.5-NA Schwarzschild optic and reaches 8 nm half-pitch lines [13].

<!-- huv-table: stack -->

| Issue | In highuvlith | Grade | Link |
|---|---|---|---|
| Schwarzschild objective | annular (centrally obscured) scalar pupil; mirror reflectance a scalar constant by default; no figure error | ✅ Implemented | [optics](../optics.md) |
| Angle-dependent multilayer pupil | opt-in Mo/Si, La/B₄C or La/B amplitude and phase across the pupil from user-supplied angle maps | 🔶 Simplified | [optics](../optics.md) |
| Six-mirror production optics | isotropic pupil with optional central obscuration | 🔶 Simplified | [optics](../optics.md) |

### Try it in highuvlith

The optic of Kinoshita's and Bell Labs' first experiments, and of today's micro-exposure tools,
at the 0.3-class NA of early EUV imaging research, on 30 nm lines and spaces.

<!-- verify-example -->
```python
import highuvlith as huv

source = huv.SourceConfig.lpp_sn_13nm5(sigma=0.5)
optics = huv.OpticsConfig.schwarzschild(numerical_aperture=0.3)
mask = huv.MaskConfig.line_space(cd_nm=30.0, pitch_nm=60.0)
grid = huv.GridConfig(size=128, pixel_nm=1.875)  # 240 nm field = 4 periods
engine = huv.SimulationEngine(source, optics, mask, grid=grid)
print(f"contrast: {engine.compute_aerial_image(focus_nm=0.0).image_contrast():.3f}")
```

???+ success "Output"

    ```text
    contrast: 0.750
    ```

<figure markdown="span">

![Image cross-sections of 30 nm lines through a 0.3 NA Schwarzschild objective with central obscuration 0, 0.25 and 0.4, and contrast versus pitch for the three, showing the mid-pitch contrast lost to the obscuration.](../assets/images/sim/site/site-schwarzschild-obscuration-light.png#gh-light-mode-only)
![Image cross-sections of 30 nm lines through a 0.3 NA Schwarzschild objective with central obscuration 0, 0.25 and 0.4, and contrast versus pitch for the three, showing the mid-pitch contrast lost to the obscuration.](../assets/images/sim/site/site-schwarzschild-obscuration-dark.png#gh-dark-mode-only)

<figcaption><strong>13.5 nm through a Schwarzschild objective.</strong> The page's example (Sn LPP σ 0.5, NA 0.3, 30 nm lines on 60 nm) for three central obscurations: blocking the pupil centre removes some zero-order/first-order pairs and changes contrast pitch by pitch; images are normalized to the clear field of the same pupil. Default 3 % flare; scalar mirror reflectance. <em>Model: Schwarzschild objective ✅ (annular scalar pupil, no figure error).</em></figcaption>
</figure>

## The consortia, 1997–2006

By the mid-1990s it looked as though 193 nm might be the end of the road for deep-UV lasers.
Intel decided to back EUV in a big way. In 1997, companies that were normally competitors
formed the **EUV LLC** to fund a research programme at three US Department of Energy
laboratories: Lawrence Berkeley, Lawrence Livermore and Sandia [1]. Announcing it in September
1997, US Energy Secretary Federico Peña described a three-year, $250 million cooperative agreement
paid for entirely by industry. The founding companies were Intel, AMD and Motorola [14]; Micron,
Infineon and IBM had joined by 2001 [17]. Japan formed a similar
group, EUVA, which cooperated with EUV LLC [1]. In Europe, ASML, Carl Zeiss and Oxford
Instruments formed the **EUCLIDES** programme in 1998 [15]; it joined forces with EUV LLC in
1999 [16].

The consortium rated its candidate approaches every six months. The laser-produced **tin**
plasma source, "initially rated last", became the top choice [1]. The EUV LLC **Engineering Test
Stand**, unveiled in April 2001, was a 13.4 nm step-and-scan tool built to demonstrate full-field
EUV imaging. Its four-mirror projection optics had NA 0.1 and 4× reduction, its source was a
laser-produced xenon plasma, and it printed 100 nm features [17, 18].

ASML started its own EUV programme in 1997 and built a prototype from 2001 [16]. In 2006 it
shipped two **Alpha Demo Tools** (NA 0.25), one to imec in Belgium and one to the College of
Nanoscale Science and Engineering in Albany, New York [16, 19].

## The source problem, 2001–2013

The EUV light comes from a **laser-produced plasma**. Tin droplets about 25 µm across leave a
generator at 70 m/s. Each is hit first by a weak laser pulse that flattens it into a pancake,
then by a powerful pulse that vaporizes it into a plasma emitting EUV light, 50,000 times a
second [3]. The drive laser is a CO₂ laser. The useful emission comes from highly charged tin
ions (roughly Sn⁸⁺ to Sn¹⁴⁺), whose many overlapping transitions pile up in a narrow band around
13.5 nm, the band the Mo/Si mirrors reflect [1, 20].

<figure markdown="span">

![The source end of an EUV scanner: a CO2 laser pulse strikes a stream of tin droplets, the plasma radiates 13.5 nm light, and an ellipsoidal multilayer collector focuses it to the intermediate focus.](../assets/images/history/euv-optical-path-source-light.svg#gh-light-mode-only)
![The source end of an EUV scanner: a CO2 laser pulse strikes a stream of tin droplets, the plasma radiates 13.5 nm light, and an ellipsoidal multilayer collector focuses it to the intermediate focus.](../assets/images/history/euv-optical-path-source-dark.svg#gh-dark-mode-only)

<figcaption><strong>The source problem, in one corner of the scanner.</strong> A CO₂ laser turns tin droplets into plasma; only the in-band light the collector catches reaches the intermediate focus, and every later mirror takes its 30 % cut. <em>Schematic, not to scale.</em></figcaption>
</figure>

Power was the long pole. Tin plasmas pumped by CO₂ lasers were working by the mid-2000s, but
"output [was] stuck in the watt range for several years". In 2013 Cymer reported more than
10 W using a pre-pulse, still far short of what production needed, and ASML bought Cymer that
May [1]. ASML's own account describes reaching 250 W, the level needed for production
throughput, as exceptionally difficult [16].

**What highuvlith models here: the laser-produced-plasma source is an in-band spectrum and a
power chain from drive laser through conversion efficiency to the intermediate focus, which
reproduces the 250 W of the NXE:3400B source; plasma dynamics and debris are out of scope.**
The conversion-efficiency defaults are simplified and the Gd and Tb powers are projections.
Speculative successors, such as free-electron lasers and steady-state microbunching rings, are
discussed under [accelerator light sources](../future/accelerator-light-sources.md).

<!-- huv-table: stack -->

| Issue | In highuvlith | Grade | Link |
|---|---|---|---|
| Tin laser-produced plasma | in-band spectrum at 13.5 nm (Sn), 6.7 nm (Gd) or 6.5 nm (Tb); drive power × CE × η(2π→IF) × étendue fraction → 250 W at IF | ✅ Implemented (🔶 CE defaults, étendue; 🧪 Gd/Tb power) | [laser-produced plasma](../sources/lpp.md) |
| Discharge-produced plasma | power chain from electrical input to the intermediate focus on reported anchors | 🔶 Simplified | [discharge-produced plasma](../sources/dpp.md) |
| Plasma dynamics, droplet generator, debris | not modelled | — | [light sources](../sources/index.md) |

## Into production, 2006–2024

<!-- huv-table: stack -->
| Year | System | NA | Resolution | k₁ (derived) | Milestone |
|---|---|---|---|---|---|
| 2006 | ASML Alpha Demo Tool | 0.25 | — | — | two prototypes, to imec and Albany [16, 19] |
| 2010 | NXE:3100 | 0.25 | 27 nm | 0.50 | pre-production; "first light" at Samsung on Christmas Eve [16, 21] |
| 2013 | NXE:3300B | 0.33 | 22 nm | 0.54 | first EUV production system shipped [16, 22] |
| 2015 | NXE:3350B | 0.33 | 16 nm | 0.39 | [22] |
| 2017 | NXE:3400B | 0.33 | 13 nm | 0.32 | ≥ 125 wafers/h at 20 mJ/cm² [23] |
| 2019 | NXE:3400C | 0.33 | 13 nm | 0.32 | ≥ 170 wafers/h at 20 mJ/cm² [24] |
| 2021 | NXE:3600D | 0.33 | 13 nm | 0.32 | ≥ 160 wafers/h at 30 mJ/cm² [25] |
| 2024 | NXE:3800E | 0.33 | 13 nm | 0.32 | ≥ 220 wafers/h at 30 mJ/cm² [26] |

k₁ = R·NA/λ from the vendor's stated resolution, as on [the tool data](tool-data.md) page:
vendors define resolution differently, so it is an indicator of the trend, not a definition.

In 2012 Intel, Samsung and TSMC joined ASML's Customer Co-Investment Program. They committed to
fund five years of R&D and took stakes in the company [16].

On **18 October 2018** Samsung announced that it had started wafer production of its 7LPP process
with EUV at its S3 fab in Hwaseong. It wrote that one EUV mask could replace up to four ArF
masks for a layer, cutting the total mask count by about 20 % [27]. The first commercial product
made with EUV, according to ASML, was Samsung's Galaxy Note10 smartphone in 2019. ASML's 2022
account dates its 100th EUV system shipment to December 2020 [16].

## Masks, pellicles and photons, 2001–2025

- **Reflective masks.** The EUV reticle is itself a Mo/Si multilayer mirror on a
  low-thermal-expansion substrate, with the pattern in an absorbing film on top. The Engineering
  Test Stand already used one, a multilayer-coated ULE glass substrate, in 2001 [18]. Because the mask reflects, light must strike it
  slightly off normal, and the thickness of the absorber then matters. These mask-3D effects
  are well known to lithographers.
- **Pellicles.** Protective films over the mask arrived late. In 2018 ASML's polysilicon-based
  pellicle, 50 nm thick, transmitted 83 % and survived a 250 W source but had to be replaced every
  3,000 wafers; the target was 90 % [28].

<figure markdown="span">

![Cross-section of an EUV mask: a low-expansion substrate, 40 to 50 Mo/Si bilayers under a Ru cap, a tantalum-based absorber on top, the 6-degree chief ray with the absorber's shadow hatched beside it, and a pellicle membrane above that the light crosses twice.](../assets/images/nodes/euv-mask-stack-light.svg#gh-light-mode-only)
![Cross-section of an EUV mask: a low-expansion substrate, 40 to 50 Mo/Si bilayers under a Ru cap, a tantalum-based absorber on top, the 6-degree chief ray with the absorber's shadow hatched beside it, and a pellicle membrane above that the light crosses twice.](../assets/images/nodes/euv-mask-stack-dark.svg#gh-dark-mode-only)

<figcaption><strong>An EUV mask is a mirror with a patterned absorber on top.</strong> 40–50 Mo/Si bilayers under a Ru cap reflect about 70 % at 13.5 nm; the 6° chief ray makes the tens-of-nanometre absorber cast a shadow, and a pellicle's transmission counts twice. <em>Schematic, not to scale. The engine's mask is a thin transmittance map: no mask 3-D, pellicles not modelled.</em></figcaption>
</figure>

- **Photon shot noise.** An EUV photon carries 91.8 eV against 6.4 eV at 193 nm, so the same dose
  delivers about 14 times fewer photons. Random fluctuation in the number of photons, and of acid
  molecules in the resist, grows correspondingly. It shows up as line-edge roughness and as
  occasional missing or bridging features, known as stochastic defects. The
  [stochastic frontier](../future/stochastic-frontier.md) follows this problem forward, and
  [7 nm and 5 nm](../nodes/7-and-5nm.md#stochastics-when-photons-run-out) shows where it first bit in production.

Photons in a 10 × 10 nm square at 30 mJ/cm², computed from E = hc/λ with CODATA constants
(1 mJ/cm² = 10⁻¹⁷ J/nm²), with the 1/√N shot noise each count implies:

<ul class="huv-tiles">
<li><span class="huv-tile__label">248 nm (KrF)</span><span class="huv-tile__value">≈ 37,500</span><span class="huv-tile__note">5.00 eV per photon · 12.5 per nm² at 1 mJ/cm² · noise 0.5 %</span></li>
<li><span class="huv-tile__label">193 nm (ArF)</span><span class="huv-tile__value">≈ 29,100</span><span class="huv-tile__note">6.42 eV per photon · 9.7 per nm² at 1 mJ/cm² · noise 0.6 %</span></li>
<li><span class="huv-tile__label">13.5 nm (EUV)</span><span class="huv-tile__value">≈ 2,040</span><span class="huv-tile__note">91.8 eV per photon · 0.68 per nm² at 1 mJ/cm² · noise 2.2 %</span></li>
</ul>

**What highuvlith models here: photon shot noise and the roughness it produces are computed
from Poisson counts at the source's photon energy, while the EUV projection pupils are
simplified — a circular pupil with optional central obscuration, no anamorphic magnification,
no mask-3D (thick-absorber) effects and no mask-side oblique incidence.** The
[playground](../playground/index.md) has a shot-noise calculator for the numbers above.

<!-- huv-table: stack -->

| Issue | In highuvlith | Grade | Link |
|---|---|---|---|
| Photon shot noise | Poisson counts per pixel with the photon density derived from the source wavelength (0.68 photons/nm² per mJ/cm² at 13.5 nm) | ✅ Implemented | [research modules](../research-modules.md) |
| Line-edge roughness from the noise | edge statistics of threshold crossings on noisy aerial images; no resist chemistry or electron blur | 🔶 Simplified | [research modules](../research-modules.md) |
| Reflective mask, absorber shadow, pellicle | not modelled: the mask is a thin transmittance map | — | [masks](../masks.md) |

### Try it in highuvlith

The arithmetic behind EUV stochastics, using the wavelengths of two source presets. The
ArF preset sits at 193.37 nm, a little above the nominal 193 nm, so its count comes
out slightly higher than the tile above.

<!-- verify-example -->
```python
import highuvlith as huv

HC_EV_NM, EV_J = 1239.84193, 1.602176634e-19
for source in (huv.SourceConfig.arf_laser(sigma=0.7), huv.SourceConfig.lpp_sn_13nm5(sigma=0.9)):
    e_photon = HC_EV_NM / source.wavelength_nm * EV_J        # J per photon
    n = 30e-3 * 1e-14 / e_photon * 100                          # 30 mJ/cm^2 on 10 x 10 nm
    print(f"{source.wavelength_nm:6.2f} nm: {n:6.0f} photons, shot noise {100 / n ** 0.5:.1f} %")
```

???+ success "Output"

    ```text
    193.37 nm:  29203 photons, shot noise 0.6 %
     13.50 nm:   2039 photons, shot noise 2.2 %
    ```

## High-NA EUV, 2023–2030

The next step raised the numerical aperture from 0.33 to **0.55**. Larger mirrors would increase
the angles at which light strikes the reticle. ASML's EXE systems therefore demagnify by 4× in
one direction and 8× in the other, and print a field half the size of the NXE field, with a
critical dimension of 8 nm [29]. ASML shipped the first modules of the first High-NA system to
Intel in December 2023 [29, 30]. The first second-generation EXE:5200B shipped in the second
quarter of 2025 (reported in ASML's results of 16 July 2025), rated at ≥ 175 wafers per hour at 50 mJ/cm² [31, 32]. ASML has also discussed a "Hyper-NA"
system near NA 0.75 as a possibility around 2030; it is at the feasibility-study stage [33].
[High-NA and Hyper-NA EUV](../future/high-na-and-hyper-na.md#what-it-costs-part-3-bigger-mirrors-and-a-central-obscuration) in the future section picks up the
story from there, and [beyond EUV](../future/beyond-euv.md) looks at wavelengths near 6.7 nm.

<figure markdown="span">

![Contrast versus pitch from 12 to 40 nm for 0.33 and 0.55 NA EUV optics with a tin plasma source: the 0.33 NA curve reaches zero near 22 nm pitch, the 0.55 NA curve near 13 nm.](../assets/images/sim/site/site-euv-nxe-vs-high-na-light.png#gh-light-mode-only)
![Contrast versus pitch from 12 to 40 nm for 0.33 and 0.55 NA EUV optics with a tin plasma source: the 0.33 NA curve reaches zero near 22 nm pitch, the 0.55 NA curve near 13 nm.](../assets/images/sim/site/site-euv-nxe-vs-high-na-dark.png#gh-dark-mode-only)

<figcaption><strong>Resolution at NA 0.33 against NA 0.55.</strong> Contrast of 1:1 lines against pitch through the simulator's EUV presets (Sn LPP, conventional σ 0.9, two-period commensurate fields): isotropic pupil — the anamorphic 4×/8× magnification is not modelled — with a 0.2·NA central obscuration assumed for High-NA; thin mask (no mask-3D shadowing), no flare. <em>Model: Hopkins imaging ✅ · EUV projection optics 🔶 (obscuration assumed).</em></figcaption>
</figure>

### Try it in highuvlith

8 nm lines and spaces put k₁ at 0.20 for an NXE-class 0.33-NA pupil, below the single-exposure
limit, and at 0.33 for High-NA. The High-NA preset assumes a central obscuration of 0.2 NA,
and neither pupil is anamorphic.

<!-- verify-example -->
```python
import highuvlith as huv

source = huv.SourceConfig.lpp_sn_13nm5(sigma=0.9)
mask = huv.MaskConfig.line_space(cd_nm=8.0, pitch_nm=16.0)
grid = huv.GridConfig(size=128, pixel_nm=1.0)  # 128 nm field = 8 periods
for label, optics in (("NXE, NA 0.33", huv.OpticsConfig.euv_nxe()),
                      ("High-NA, NA 0.55", huv.OpticsConfig.euv_high_na())):
    engine = huv.SimulationEngine(source, optics, mask, grid=grid)
    contrast = engine.compute_aerial_image(focus_nm=0.0).image_contrast()
    print(f"{label}: contrast of a 16 nm pitch = {contrast:.3f}")
```

???+ success "Output"

    ```text
    NXE, NA 0.33: contrast of a 16 nm pitch = 0.000
    High-NA, NA 0.55: contrast of a 16 nm pitch = 0.269
    ```

## Key takeaways

- EUV needs all-reflective optics in vacuum. Mo/Si multilayer mirrors reflect about 70 % each,
  so only a few percent of the light survives a dozen reflections.
- The idea dates from the mid-1980s (Kinoshita at NTT; LLNL; Bell Labs); industrial development
  ran through the EUV LLC (1997), EUCLIDES (1998) and EUVA, and the tin plasma source, first
  ranked last, took until the mid-2010s to approach production power.
- Production EUV began with Samsung's 7LPP in 2018. The 0.33-NA NXE scanners reached 13 nm
  resolution, and High-NA (0.55, anamorphic) systems started shipping in 2023.
- A 13.5 nm photon carries about 14 times the energy of a 193 nm photon, so shot noise and
  stochastic defects are EUV's defining problem.

**Where next:** ← [Immersion & multipatterning](immersion-multipatterning.md) · [Parallel paths](parallel-paths.md) → · Related: [High-NA and Hyper-NA EUV](../future/high-na-and-hyper-na.md) · [Laser-produced plasma](../sources/lpp.md) · [7 nm and 5 nm](../nodes/7-and-5nm.md) · [Capability matrix](../capability-matrix.md#optical-systems)

## References and further reading

1. J. Hecht, "Hiroo Kinoshita: Lighting the way for extreme ultraviolet lithography," *SPIE Photonics
   Focus* (May/June 2023), [spie.org](https://spie.org/news/photonics-focus/mayjune-2023/developing-extreme-ultraviolet-lithography).
2. ASML, ["Making EUV: from lab to fab"](https://www.asml.com/en/company/stories/2022/making-euv-lab-to-fab) (2022).
3. ASML, ["Light and lasers"](https://www.asml.com/en/technology/lithography-principles/light-and-lasers).
4. E. Spiller, "Low-loss reflection coatings using absorbing materials," *Appl. Phys. Lett.* **20**,
   365 (1972), [doi:10.1063/1.1654189](https://doi.org/10.1063/1.1654189).
5. J. H. Underwood, T. W. Barbee Jr., "Layered synthetic microstructures as Bragg diffractors for
   X rays and extreme ultraviolet: theory and predicted performance," *Appl. Opt.* **20**, 3027 (1981),
   [doi:10.1364/AO.20.003027](https://doi.org/10.1364/AO.20.003027).
6. T. W. Barbee Jr., S. Mrowka, M. C. Hettrick, "Molybdenum-silicon multilayer mirrors for the
   extreme ultraviolet," *Appl. Opt.* **24**, 883 (1985),
   [doi:10.1364/AO.24.000883](https://doi.org/10.1364/AO.24.000883).
7. ASML, ["Lenses and mirrors"](https://www.asml.com/en/technology/lithography-principles/lenses-and-mirrors).
8. H. Kinoshita, K. Kurihara, Y. Ishii, Y. Torii, "Soft x-ray reduction lithography using multilayer
   mirrors," *J. Vac. Sci. Technol. B* **7**, 1648 (1989),
   [doi:10.1116/1.584507](https://doi.org/10.1116/1.584507).
9. A. M. Hawryluk, L. G. Seppala, "Soft x-ray projection lithography using an x-ray reduction
   camera," *J. Vac. Sci. Technol. B* **6**, 2162 (1988),
   [doi:10.1116/1.584107](https://doi.org/10.1116/1.584107).
10. J. E. Bjorkholm et al., "Reduction imaging at 14 nm using multilayer-coated optics: printing of
    features smaller than 0.1 µm," *J. Vac. Sci. Technol. B* **8**, 1509 (1990),
    [doi:10.1116/1.585106](https://doi.org/10.1116/1.585106).
11. G. D. Kubiak et al., "Diffraction-limited soft x-ray projection lithography with a laser
    plasma source," *J. Vac. Sci. Technol. B* **9**, 3184 (1991),
    [doi:10.1116/1.585313](https://doi.org/10.1116/1.585313).
12. H. I. Smith, M. L. Schattenburg, "X-ray lithography from 500 to 30 nm: X-ray nanolithography,"
    *IBM J. Res. Dev.* **37**, 319–329 (1993), [doi:10.1147/rd.373.0319](https://doi.org/10.1147/rd.373.0319).
13. Center for X-Ray Optics, Berkeley Lab, ["MET5"](https://cxro.lbl.gov/met).
14. Remarks by US Energy Secretary Federico Peña announcing the EUV LLC agreement, 11 September 1997,
    [Intel press archive](https://www.intel.com/pressroom/archive/speeches/EUV91197.HTM).
15. J. P. H. Benschop et al., "EUCLIDES: European EUVL program," *J. Vac. Sci. Technol. B* **17**,
    2978 (1999), [doi:10.1116/1.590938](https://doi.org/10.1116/1.590938).
16. ASML, ["Making EUV: from lab to fab"](https://www.asml.com/en/company/stories/2022/making-euv-lab-to-fab) (2022).
17. Sandia National Laboratories, ["Partners unveil first extreme ultraviolet chip-making machine"](https://newsreleases.sandia.gov/partners-unveil-first-extreme-ultraviolet-chip-making-machine/) (April 2001).
18. H. N. Chapman et al., "First lithographic results from the extreme ultraviolet Engineering Test
    Stand," *J. Vac. Sci. Technol. B* **19**, 2389 (2001),
    [doi:10.1116/1.1414017](https://doi.org/10.1116/1.1414017).
19. ASML Form 20-F for fiscal 2006, [sec.gov](https://www.sec.gov/Archives/edgar/data/937966/000095012307000854/u51076e20vf.htm).
20. O. O. Versolato, "Physics of laser-driven tin plasma sources of EUV radiation for nanolithography,"
    *Plasma Sources Sci. Technol.* **28**, 083001 (2019),
    [doi:10.1088/1361-6595/ab3302](https://doi.org/10.1088/1361-6595/ab3302).
21. ASML Form 20-F for fiscal 2010, [sec.gov](https://www.sec.gov/Archives/edgar/data/937966/000095012311013996/u09689e20vf.htm).
22. ASML Form 20-F for fiscal 2015, [sec.gov](https://www.sec.gov/Archives/edgar/data/937966/000093796616000017/a20-fasml2015.htm).
23. ASML, [TWINSCAN NXE:3400B](https://www.asml.com/en/products/euv-lithography-systems/twinscan-nxe3400b);
    first volume shipments in ASML's [2017 integrated report](https://www.sec.gov/Archives/edgar/data/937966/000093796618000007/a2017integratedreportbased.htm).
24. ASML, [TWINSCAN NXE:3400C](https://www.asml.com/en/products/euv-lithography-systems/twinscan-nxe3400c).
25. ASML, [TWINSCAN NXE:3600D](https://www.asml.com/en/products/euv-lithography-systems/twinscan-nxe-3600d);
    first shipment in ASML's [Q2 2021 results press release](https://www.sec.gov/Archives/edgar/data/937966/000093796621000016/pressreleasequarterlyresul.htm).
26. ASML, [TWINSCAN NXE:3800E](https://www.asml.com/en/products/euv-lithography-systems/twinscan-nxe-3800e);
    first shipment in ASML's [Q1 2024 investor presentation](https://www.sec.gov/Archives/edgar/data/937966/000093796624000013/presentationinvestorrela.htm).
27. Samsung Newsroom, ["Samsung Electronics starts production of EUV-based 7nm LPP process"](https://news.samsung.com/global/samsung-electronics-starts-production-of-euv-based-7nm-lpp-process) (18 October 2018).
28. Semiconductor Engineering, ["EUV pellicle, uptime and resist issues continue"](https://semiengineering.com/euv-pellicle-uptime-and-resist-issues-continue/) (26 September 2018).
29. ASML, ["5 things you should know about High NA EUV lithography"](https://www.asml.com/en/news/stories/2024/5-things-high-na-euv) (2024).
30. ASML, [TWINSCAN EXE:5000](https://www.asml.com/en/products/euv-lithography-systems/twinscan-exe-5000).
31. ASML, [Q2 2025 results press release](https://www.sec.gov/Archives/edgar/data/937966/000162828025034992/pressreleasequarterlyresul.htm).
32. ASML, [TWINSCAN EXE:5200B](https://www.asml.com/en/products/euv-lithography-systems/twinscan-exe-5200b).
33. EE Times, ["ASML aims for Hyper-NA EUV, shrinking chip limits"](https://www.eetimes.com/asml-aims-for-hyper-na-euv-shrinking-chip-limits/) (2024).
34. H. J. Levinson, "High-NA EUV lithography: current status and outlook for the future," *Jpn. J.
    Appl. Phys.* **61**, SD0803 (2022), [doi:10.35848/1347-4065/ac49fa](https://doi.org/10.35848/1347-4065/ac49fa).
