---
description: >-
  How lithography moved from contact printing to projection: the Micralign scanning
  projection aligner, the wafer stepper, g-line and i-line lenses, the rise of Nikon, Canon
  and ASML, and the switch to step-and-scan.
---

# The projection era

<p class="huv-lead">Chapter 2 of 7 — Putting optics between the mask and the wafer solved contact printing's defect problem, and it made the lens the product. From the Micralign of 1973 to the first TWINSCAN of 2001 the lithography business was a contest in building larger, sharper and more stable projection optics, and in moving masks and wafers under them precisely (1973–2001). The contest also reshaped the industry: American companies supplied about 90 % of lithography tools in 1980 and about 10 % in 1990 [1].</p>

<figure markdown="span">

<picture><source media="(max-width: 600px)" srcset="../assets/images/diagrams/spectrum-strip-ribbon-projection-narrow-light.svg"><img class="huv-no-zoom" src="../assets/images/diagrams/spectrum-strip-ribbon-projection-light.svg#gh-light-mode-only" alt="The spectrum strip from 500 to 0.05 nm with the g- and i-line projection span highlighted and labelled 'You are here'; the other lines, optics and era pills are faded."></picture>
<picture><source media="(max-width: 600px)" srcset="../assets/images/diagrams/spectrum-strip-ribbon-projection-narrow-dark.svg"><img class="huv-no-zoom" src="../assets/images/diagrams/spectrum-strip-ribbon-projection-dark.svg#gh-dark-mode-only" alt="The spectrum strip from 500 to 0.05 nm with the g- and i-line projection span highlighted and labelled 'You are here'; the other lines, optics and era pills are faded."></picture>

<figcaption><strong>You are here on the axis of light.</strong> The chapter's wavelengths and eras at full strength, the rest faded; years are first tool models in litho_tools.csv; the log axis is expanded above 100 nm. <em>Data chart from litho_tools.csv; no model badge.</em></figcaption>
</figure>

<figure markdown="span">

![Three panels: the Micralign 1:1 scanning projection aligner with an Offner two-mirror relay, an arc slit and a carriage moving mask and wafer together; a step-and-repeat stepper lighting a whole reticle field; a step-and-scan system sweeping a slit while reticle and wafer scan oppositely.](../assets/images/history/three-ways-to-move-light.svg#gh-light-mode-only)
![Three panels: the Micralign 1:1 scanning projection aligner with an Offner two-mirror relay, an arc slit and a carriage moving mask and wafer together; a step-and-repeat stepper lighting a whole reticle field; a step-and-scan system sweeping a slit while reticle and wafer scan oppositely.](../assets/images/history/three-ways-to-move-dark.svg#gh-dark-mode-only)

<figcaption><strong>Three ways to move the image.</strong> The Micralign scans mask and wafer together through a 1:1 ring-field mirror relay; a stepper exposes a whole reduced field and steps; a step-and-scan tool sweeps a slit while reticle and wafer scan at 4v and v. <em>Schematic, not to scale.</em></figcaption>
</figure>

## Scanning projection: the Micralign, 1973–1981

The first attempts at 1:1 projection used refractive lenses and imaged the whole wafer at
once. Canon's PPC-1 (1970) used a g-line lens of NA 0.14 specified for 2–3 µm on wafers up to
50 mm [2]. Such systems, from Canon, Telefunken and others, suffered from long air paths,
absorbing and inhomogeneous glass, and a lack of telecentricity, which turned wafer
unflatness into overlay error [2, 3].

Perkin-Elmer took a different route. Its **Micralign**, introduced in 1973 after R&D that
began in 1969 under an Air Force contract, was an all-mirror, 1:1 **scanning** projection
aligner [1, 4]. The optics were an **Offner relay**: two concentric spherical mirrors and
three flat fold mirrors [3]. Because it had no lenses it had no chromatic aberration and a
flat, telecentric field. It was, however, well corrected only in a thin ring, about 1 mm wide
on a 150 mm diameter. A mercury capillary lamp bent into an arc lit that ring, and a carriage
swept the mask and wafer through it together to expose the whole wafer [3]. NA was limited to
about 0.16 [3, 1].

The Micralign combined with the new positive DNQ resists "dramatically reduced defect rate",
and Perkin-Elmer became the largest firm in the industry [1]. More than 2,000 were sold, at
about $170,000 each in 1979. The Micralign 500 of 1981 listed at $675,000 and was the first
tool to reach 100 wafers per hour [4, 1]. Later versions resolved about 1.2 µm by using the
mercury lamp's shorter 250 nm radiation [2]. Canon built a competing mirror projection
aligner, the MPA-500FA, and by 1986 held 90 % of the projection-aligner market in Japan [1].
Perkin-Elmer sued Cobilt over its competing design in 1977; the case settled in 1984 with an
$18 million payment [4].

Full-wafer 1:1 imaging had a built-in limit. A 1:1 mask must be as accurate as the wafer
pattern, and overlay must be held across the whole wafer. As wafers grew, keeping
layer-to-layer overlay within the usual rule of thumb (about 20–30 % of the minimum feature)
became too hard [2, 3].

## The wafer stepper, 1975–1988

The alternative came from mask making. Photorepeaters already stepped a reduced image of a
reticle across a plate to build a mask. A **wafer stepper** does the same directly on the
wafer, one field at a time, with an alignment step before each exposure. IBM showed
step-and-repeat printing of 1 µm features in 1975, with 405 nm light at NA 0.32 [1].

**GCA's DSW 4800** (1978) was the first successful commercial wafer stepper. It had a Zeiss
10:1 reduction lens of NA 0.28, a 10 × 10 mm field, and the spectrally isolated mercury
g-line at 436 nm. It listed at $450,000 [4, 3]. It was also the first such tool with automated
wafer handling and alignment [3]. Steppers gave far fewer defects and better overlay than
projection aligners but much lower throughput. According to Kato, it was the 256K DRAM
generation that made them the mainstream tool [1].

Japan's answer followed quickly:

- **Nikon** developed a step-and-repeat system under the MITI-funded VLSI programme and
  launched the **NSR-1010G** in 1980, the first commercial stepper made in Japan, for customers
  NEC and Toshiba [4, 5]. The **NSR-1505G** (1981) switched to 5:1 reduction and a 15 × 15 mm
  field with NA 0.30 [6]. Mack credits Nikon's brighter illumination, larger field,
  laser-interferometer stages and automatic alignment with better throughput and overlay.
  Nikon matched GCA's stepper shipments in 1984 and passed them in 1985 [4].
- **Canon** shipped its first stepper, the FPA-1500FA, in 1984 [1, 4]. In that year more
  than 1,000 steppers were shipped worldwide, close to 600 of them to Japan [1].
- **ASML** was founded in 1984 as a joint venture of Philips and ASM International, starting in
  a "leaky shed" next to a Philips office in Eindhoven, to commercialize a Philips stepper
  [7, 4]. Its first system, the PAS 2000, dates from 1984 by ASML's account and 1985 by
  Mack's and Kato's. The PAS 2500 followed in 1986 and the PAS 5500 platform in 1991
  [7, 4, 1].

The American pioneers did not survive the decade in their old form. General Signal bought
the struggling GCA for $76 million in 1988 and closed it in 1993, despite
SEMATECH investing $60–75 million in GCA's deep-UV steppers. Perkin-Elmer's lithography
business went to Silicon Valley Group in 1990 [4]. A quotation in Mack's history gives a
sense of why the business was hard. GCA's first g-line stepper cost it $5 million to develop,
its i-line stepper $25 million, and its deep-UV stepper $140 million [4].

## g-line, i-line and ever larger lenses, 1978–1994

Steppers used single lines of the mercury arc lamp. The **g-line** is at 435.83 nm, the
h-line at 404.66 nm and the **i-line** at 365.02 nm (wavelengths in air) [8]. Moving from g to
i shortened λ by 16 %. Sources disagree on the first i-line stepper: Mack credits TRE
Semiconductor (1982, with a Zeiss lens), while Nikon calls its 1984 model the first i-line
stepper produced [4, 5]. ASML's first i-line stepper, the PAS 2500/40 (1987), had NA 0.40 and
0.7 µm resolution. Nikon's NSR-1505G4B was the first stepper to go below 1 µm, at 0.9 µm, in
1987 [1].

Bruning's summary of the turning points shows the shape of the trend: reduction ratio fell as
fields grew, while the wavelength and the minimum feature shrank (turning points in
reduction-stepper evolution, from Bruning (2007), Table 1 [3]).

<!-- huv-table: stack -->
| Year | Reduction | Wafer | Image field | λ | Minimum CD |
|---|---|---|---|---|---|
| 1970 | 1:1 | 50 mm | full wafer | 400–440 nm lamp | 5 µm |
| 1980 | 10:1 | 100 mm | 10 × 10 mm | 436 nm | 1.5 µm |
| 1985 | 5:1 | 150 mm | 14 × 14 mm | 436 nm | 1.0 µm |
| 1990 | 4:1 | 200 mm | 20 × 20 mm | 365 nm | 0.5 µm |
| 1993 | 4:1 | 200 mm | 22 × 22 mm | 248 nm | 0.25 µm |

Lens NA more than doubled over the stepper era, from 0.28 in 1978 to 0.63 in 1994 (Nikon
NSR-2205i11D and Canon FPA-3000i4 [5, 9]). Each gain cost something: a larger, more complex lens
and a shallower depth of focus, which scales as λ/NA². The i-line's roughly 3 nm spectral
width also forced lens designers to combine several glasses to correct colour, as for the
g-line [3]. The excimer laser removed that burden (see [deep UV](excimer-duv.md)).

<figure markdown="span">

![Numerical aperture of each tool model from 1973 to 2000, the projection-era tools drawn in colour and later tools greyed out: the Micralign at 0.17, the GCA DSW 4800 stepper at 0.28, i-line steppers near 0.63 and the first step-and-scan tools at 0.6 to 0.68.](../assets/images/history/na-vs-year-to-2000-light.svg#gh-light-mode-only)
![Numerical aperture of each tool model from 1973 to 2000, the projection-era tools drawn in colour and later tools greyed out: the Micralign at 0.17, the GCA DSW 4800 stepper at 0.28, i-line steppers near 0.63 and the first step-and-scan tools at 0.6 to 0.68.](../assets/images/history/na-vs-year-to-2000-dark.svg#gh-dark-mode-only)

<figcaption><strong>NA through the projection era, 1973–2000.</strong> The same chart as the full NA history, cut at 2000 with the projection-era tools in colour: every step is a larger lens, from the Micralign's mirrors at 0.17 to 0.68 at the end of the century. <em>Data chart from litho_tools.csv; no model badge.</em></figcaption>
</figure>

The full chart, through immersion and EUV, is on the [history overview](index.md); every
point's source is on [the tool data](tool-data.md) page.

## Step-and-scan, 1990–2001

A lens with a round field can only print the square inscribed in that circle. Scanning
removes the restriction. Illuminate only a slit and move the reticle and wafer through it in
step. The lens then needs to be corrected only across the slit, and the printed field can be
longer than the lens field in the scan direction. Bruning notes that a round stepper lens can
scan a field nearly 30 % taller than its inscribed square, so a smaller lens, or a same-size
lens with higher NA, can print a larger field. The price is moving the reticle and wafer
stages precisely at different speeds [3]. With 4× reduction the reticle moves four times
faster than the wafer.

<figure markdown="span">

![Left, a step-and-repeat stepper: the whole reticle is lit in one flash, one field printed, the wafer stepped to the next. Right, a step-and-scan system: a slit of light sweeps the reticle while reticle and wafer scan in opposite directions, the reticle four times faster for a 4x lens.](../assets/images/history/stepper-vs-scanner-light.svg#gh-light-mode-only)
![Left, a step-and-repeat stepper: the whole reticle is lit in one flash, one field printed, the wafer stepped to the next. Right, a step-and-scan system: a slit of light sweeps the reticle while reticle and wafer scan in opposite directions, the reticle four times faster for a 4x lens.](../assets/images/history/stepper-vs-scanner-dark.svg#gh-dark-mode-only)

<figcaption><strong>Stepper and scanner.</strong> With an image-inverting 4× lens the reticle and wafer stages scan in opposite directions, the reticle four times faster, so a lens corrected over the slit alone prints a 26 × 33 mm field. <em>Schematic, not to scale.</em></figcaption>
</figure>

The first step-and-scan tool was Perkin-Elmer's design, introduced in 1990 as the SVG
Lithography **Micrascan** [1, 4]. It used catadioptric optics (mirrors and lenses) with an
annular field, a mercury-lamp band near 250 nm and NA 0.35 [3]. It was hard to manufacture and
sold fewer than ten units a year from 1991 to 1994, despite about $30 million of SEMATECH
support [1, 4]. The Micrascan II scanned a rectangular strip with NA 0.50. The Micrascan III
of 1996 switched to a KrF laser at NA 0.6, and its catadioptric design meant the laser did not
need line narrowing [3, 1]. SVGL shipped its 100th Micrascan in 1997 [1].

Meanwhile the lens makers found that ordinary refractive stepper lenses scanned well:

- **Nikon NSR-S201A** (1995): the first all-refractive scanner. KrF, NA up to 0.60, 250 nm,
  25 × 33 mm field; Kato calls it the first production-worthy KrF scanner [5, 4, 1].
- **ASML PAS 5500/500** (1997): ASML's first scanner. KrF, variable NA 0.4–0.63, 0.22 µm and
  96 wafers per hour on 200 mm wafers [1, 4].
- **Canon FPA-4000ES1** (1997): Canon's first KrF scanner [1].

The scanner field settled at **26 × 33 mm**, still the standard for DUV and 0.33-NA EUV tools
today [5, 10]. ASML's **TWINSCAN** platform (announced in 2000 according to Mack, introduced in
2001 according to ASML, and first installed at TSMC in October 2001) added a second wafer stage,
so one wafer could be measured while another was exposed [4, 7].
ASML bought SVG Lithography in 2001 and discontinued the Micrascan line. By 2002 ASML had the
largest share of the lithography market [1, 4].

**What highuvlith models here: the imaging engine is a projection imager of one field, a pupil
function feeding Hopkins partially coherent imaging; it does not model stages, scanning,
slit-dependent aberrations, dose integration during the scan, or overlay between layers.** The
refractive model has a single axial-chromatic coefficient, so the multi-glass colour correction
of real g- and i-line lenses is not represented; the source pages say how each lamp or laser
preset describes its line.

<!-- huv-table: stack -->

| Issue | In highuvlith | Grade | Link |
|---|---|---|---|
| Projection imaging through a reduction lens | Hopkins partially coherent imaging of one field, scalar; the vector model above NA ≈ 0.8 | ✅ Implemented | [pipeline](../pipeline.md), [vector imaging](../vector-imaging.md) |
| The lens: NA, reduction, flare, aberrations, colour | pupil function with Zernike aberrations and one axial-chromatic coefficient | ✅ Implemented | [optics](../optics.md) |
| Mercury g-, h- and i-lines | NIST air wavelengths, exact photon budgets, line-shape conventions | ✅ Implemented (🔶 parameters) | [DUV/UV heritage](../sources/duv-heritage.md) |
| Stages, scanning, overlay, throughput | not modelled | — | [architecture](../architecture.md) |

### Try it in highuvlith

`OpticsConfig.rayleigh_resolution` uses the classical k₁ = 0.61. The 1978 lens barely
resolves 0.8 µm lines: the first diffraction orders of a 1.6 µm pitch fall at the edge of its
NA 0.28 pupil. The 1994 i-line lens images them easily.

<!-- verify-example -->
```python
import highuvlith as huv

mask = huv.MaskConfig.line_space(cd_nm=800.0, pitch_nm=1600.0)
grid = huv.GridConfig(size=128, pixel_nm=25.0)  # 3.2 um field = 2 periods
tools = [
    ("GCA DSW 4800 (1978)", huv.SourceConfig.hg_g_line(sigma=0.5), 0.28),
    ("Nikon NSR-1505G4D (1987)", huv.SourceConfig.hg_g_line(sigma=0.5), 0.45),
    ("Nikon NSR-2205i11D (1994)", huv.SourceConfig.hg_i_line(sigma=0.5), 0.63),
]
for name, source, na in tools:
    optics = huv.OpticsConfig(numerical_aperture=na)
    engine = huv.SimulationEngine(source, optics, mask, grid=grid)
    contrast = engine.compute_aerial_image(focus_nm=0.0).image_contrast()
    r = optics.rayleigh_resolution(source.wavelength_nm)
    print(f"{name}: 0.61 lambda/NA = {r:.0f} nm, contrast at 1.6 um pitch = {contrast:.2f}")
```

???+ success "Output"

    ```text
    GCA DSW 4800 (1978): 0.61 lambda/NA = 949 nm, contrast at 1.6 um pitch = 0.85
    Nikon NSR-1505G4D (1987): 0.61 lambda/NA = 591 nm, contrast at 1.6 um pitch = 0.97
    Nikon NSR-2205i11D (1994): 0.61 lambda/NA = 353 nm, contrast at 1.6 um pitch = 0.98
    ```

<figure markdown="span">

![Cross-sections of 800 nm lines on a 1.6 micrometre pitch through three stepper lenses; the edges sharpen from the 1978 g-line tool to the 1994 i-line tool.](../assets/images/sim/site/site-steppers-1600nm-pitch-light.png#gh-light-mode-only)
![Cross-sections of 800 nm lines on a 1.6 micrometre pitch through three stepper lenses; the edges sharpen from the 1978 g-line tool to the 1994 i-line tool.](../assets/images/sim/site/site-steppers-1600nm-pitch-dark.png#gh-dark-mode-only)

<figcaption><strong>Three stepper generations on one 1.6 µm pitch.</strong> 800 nm lines through the GCA DSW 4800 (g-line, NA 0.28), Nikon NSR-1505G4D (g-line, NA 0.45) and NSR-2205i11D (i-line, NA 0.63), σ 0.5, default 2 % flare — the projection-era example as images. <em>Model: Hopkins imaging ✅ (ideal lenses: the tools' real aberrations and illuminators are not modelled).</em></figcaption>
</figure>

## Key takeaways

- Projection printing ended mask damage. The mirror-based Micralign (1973, NA ≈ 0.16) made it
  practical and dominated the 1970s.
- The wafer stepper (GCA DSW 4800, 1978) traded throughput for defect density, overlay and
  resolution. Reduction lenses went from 10:1 to 5:1 to 4:1 as fields grew.
- Nikon, Canon and ASML took the market from GCA and Perkin-Elmer in the 1980s. US suppliers
  went from about 90 % of the market in 1980 to about 10 % in 1990.
- Step-and-scan (Micrascan 1990; Nikon S201A 1995; ASML PAS 5500/500 1997) let a lens
  corrected over a narrow slit print a 26 × 33 mm field. Every advanced tool since has been a
  scanner.

**Where next:** ← [From stone to silicon](origins.md) · [Excimer lasers & deep UV](excimer-duv.md) → · Related: [Optical systems](../optics.md) · [Capability matrix](../capability-matrix.md#optical-systems)

## References and further reading

1. A. Kato, "Chronology of Lithography Milestones," v0.9 (May 2007),
   [lithoguru.com](https://www.lithoguru.com/scientist/litho_history/Kato_Litho_History.pdf).
2. J. H. Bruning, "Optical lithography: thirty years and three orders of magnitude," *Proc.
   SPIE* **3051**, 14–27 (1997),
   [author copy](http://www.lithoguru.com/scientist/litho_history/Optical_lithography_thirty_years_and_three_orders_of_magnitude_Bruning_1997.pdf).
3. J. H. Bruning, "Optical lithography … 40 years and holding," *Proc. SPIE* **6520**, 652004
   (2007), [doi:10.1117/12.720631](https://doi.org/10.1117/12.720631).
4. C. A. Mack, "Milestones in Optical Lithography Tool Suppliers" (2005),
   [lithoguru.com](https://lithoguru.com/scientist/litho_history/milestones_tools.pdf).
5. Nikon, [history of semiconductor lithography systems](https://www.nikon.com/business/semi/history/)
   and [lithography system archive](https://www.nikon.com/business/semi/lineup/archives/).
6. Semiconductor History Museum of Japan, [stepper exhibit](https://www.shmj.or.jp/museum2010/exhibi2446.html)
   (in Japanese).
7. ASML, [company history](https://www.asml.com/en/company/about-asml/history).
8. NIST, [Strong lines of mercury (Hg I)](https://physics.nist.gov/PhysRefData/Handbook/Tables/mercurytable2.htm),
   Handbook of Basic Atomic Spectroscopic Data.
9. Canon, [history of industrial equipment](https://global.canon/en/intellectual-property/history/industrial.html).
10. ASML product pages: [TWINSCAN NXT:1980Di](https://www.asml.com/en/products/duv-lithography-systems/twinscan-nxt1980di)
    and [TWINSCAN NXE:3400C](https://www.asml.com/en/products/euv-lithography-systems/twinscan-nxe3400c).
11. R. M. Henderson, K. B. Clark, "Architectural innovation: the reconfiguration of existing
    product technologies and the failure of established firms," *Administrative Science
    Quarterly* **35**, 9–30 (1990), [doi:10.2307/2393549](https://doi.org/10.2307/2393549).
    A classic management study built on the photolithographic aligner industry.
