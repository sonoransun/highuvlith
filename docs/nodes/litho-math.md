---
description: "Worked lithography arithmetic for every generation: k₁ = HP·NA/λ on ArF immersion, EUV and High-NA EUV, why one immersion exposure stops near 76–80 nm pitch, depth of focus, photons per contact, and overlay/EPE budgets."
---

# Lithography arithmetic, node by node

Why could one metal layer of a "7 nm"-class chip take four litho-etch passes on an immersion
scanner but only one on an EUV scanner? Four short formulas decide which tool prints which
layer: **resolution**, the **k₁ factor**, **depth of focus**, and **photon counting**. This page
runs them for each generation in this tour, using published scanner specifications and
roadmap targets, and ends with overlay and edge-placement budgets.

Everything here is back-of-the-envelope: no polarization, resist blur, mask 3D effects or
process control. highuvlith can check the imaging part with a real partially coherent
image calculation (Hopkins/SOCS, ✅ Implemented, see the [pipeline](../pipeline.md)) and
the photon statistics with its stochastic module (✅ Implemented, see
[research modules](../research-modules.md)). The examples at the end show how.

## Resolution and the k₁ factor

For a dense grating of lines and spaces with half-pitch HP and pitch $p = 2 \mathrm{HP}$:

```math
\mathrm{HP} = k_1 \frac{\lambda}{\mathrm{NA}}
\qquad\Longleftrightarrow\qquad
k_1 = \frac{p}{2} \cdot \frac{\mathrm{NA}}{\lambda}
```

λ is the exposure wavelength (in vacuum) and NA the numerical aperture of the projection lens on
the wafer side. The k₁ factor collects everything else (illumination shape, mask type, resist,
process control) into one "difficulty" number. Lower k₁ means harder lithography.

### The floor: k₁ = 0.25

A grating of pitch $p$ sends light into diffraction orders spaced $\lambda/p$ apart in
pupil coordinates. The lens forms an image of the grating only if the zeroth order and at
least one first order both get through the pupil, a circle of radius NA. The widest spacing
that fits is the whole pupil diameter, which you reach by tilting the illumination to the
pupil edge (dipole illumination):

```math
\frac{\lambda}{p} \le 2 \mathrm{NA}
\quad\Longrightarrow\quad
p_\text{min} = \frac{\lambda}{2 \mathrm{NA}}
\quad\Longrightarrow\quad
k_1 \ge 0.25 .
```

With a conventional (disc-shaped) source of partial coherence σ, the condition is
$\lambda/p \le \mathrm{NA} (1+\sigma)$ instead. That is the formula to use for the
examples below, which use conventional illumination.

<figure markdown="span">
  ![Three pupil diagrams: on-axis illumination leaves the first diffraction orders outside the pupil; dipole illumination puts the zeroth and minus-first orders inside; at the limit the two orders sit at opposite pupil edges, spaced 2 NA, which gives k1 = 0.25.](../assets/images/nodes/pupil-orders-light.svg#only-light){ width="760" }
  ![Three pupil diagrams: on-axis illumination leaves the first diffraction orders outside the pupil; dipole illumination puts the zeroth and minus-first orders inside; at the limit the two orders sit at opposite pupil edges, spaced 2 NA, which gives k1 = 0.25.](../assets/images/nodes/pupil-orders-dark.svg#only-dark){ width="760" }
  <figcaption>Where k₁ = 0.25 comes from: two-beam imaging with the zeroth and a first
  diffraction order at opposite edges of the pupil. Schematic; pupil radius = NA.</figcaption>
</figure>

Below k₁ = 0.25 no single exposure can image a dense grating, whatever the resist or the
mask. Production stays above the floor, because image contrast, and with it the
dose and focus latitude, shrinks towards zero as k₁ approaches 0.25.

### The scanners, in numbers

The table uses the resolution figures ASML publishes for its current systems (retrieved
September 2026). "k₁ at spec" is what those figures imply.

| System (example) | λ (nm) | NA | λ/NA (nm) | Vendor resolution (half-pitch) | k₁ at spec | Pitch at k₁ = 0.25 |
|---|---|---|---|---|---|---|
| i-line (TWINSCAN XT:400M) | 365 | 0.65 | 562 | ≤ 350 nm | 0.62 | 281 nm |
| KrF (TWINSCAN XT:860N, NXT:870) | 248 | 0.80 | 310 | ≤ 110 nm | 0.35 | 155 nm |
| Dry ArF (TWINSCAN XT:1460K) | 193 | 0.93 | 208 | ≤ 65 nm | 0.31 | 104 nm |
| Dry ArF (TWINSCAN NXT:1470) | 193 | 0.93 | 208 | ≤ 57 nm | 0.27 | 104 nm |
| ArF immersion (NXT:1980Di to NXT:2150i) | 193 | 1.35 | 143 | 38 nm dipole, 40 nm C-quad | 0.27 / 0.28 | 71.5 nm |
| EUV (NXE:3400C, 3600D, 3800E) | 13.5 | 0.33 | 40.9 | 13 nm | 0.32 | 20.5 nm |
| High-NA EUV (EXE:5000, EXE:5200B) | 13.5 | 0.55 | 24.5 | 8 nm | 0.33 | 12.3 nm |

Two things stand out. Every modern scanner is specified at about the same k₁ (0.27–0.33),
so it is λ/NA that sets the pitch a single exposure can reach. And the jump from ArF
immersion to EUV is a factor of 3.5 in λ/NA, far bigger than any earlier step.

<figure markdown="span">
  ![Line chart of required k1 against line/space pitch for ArF immersion at NA 1.35, EUV at NA 0.33 and High-NA EUV at NA 0.55. Each line rises linearly with pitch; the shaded band below k1 = 0.25 marks pitches a single exposure cannot print. Dots mark the vendor resolution specs: 38 nm, 13 nm and 8 nm half-pitch; vertical marks show the tightest metal pitches of several processes.](../assets/images/nodes/k1-vs-pitch-light.svg#only-light){ width="760" }
  ![Line chart of required k1 against line/space pitch for ArF immersion at NA 1.35, EUV at NA 0.33 and High-NA EUV at NA 0.55. Each line rises linearly with pitch; the shaded band below k1 = 0.25 marks pitches a single exposure cannot print. Dots mark the vendor resolution specs: 38 nm, 13 nm and 8 nm half-pitch; vertical marks show the tightest metal pitches of several processes.](../assets/images/nodes/k1-vs-pitch-dark.svg#only-dark){ width="760" }
  <figcaption>k₁ needed to print a pitch in one exposure. Where a tool's line enters the
  shaded band, that tool needs multipatterning. Dots: vendor half-pitch resolution specs
  (ASML product pages); vertical marks: tightest metal pitches of the processes named.
  Generated by <code>docs/figures/nodes/make_node_figures.py</code>.</figcaption>
</figure>

## Why single-exposure 193i stops near 76–80 nm pitch

ASML quotes production resolutions for its 1.35-NA immersion lens of 40 nm half-pitch with
C-quad illumination and 38 nm with dipole illumination, i.e. **80 nm and 76 nm pitch**
(k₁ = 0.28 and 0.27). The hard floor, 71.5 nm, is not a production option: when ASML introduced
its first 1.35-NA scanner in 2007 it demonstrated 36.5 nm half-pitch, k₁ = 0.255, "very close to
the theoretical limit". In production the margin is needed because:

- as k₁ approaches 0.25 the image contrast approaches zero, and dose and focus latitude
  with it;
- a dipole that reaches the limit for vertical lines does nothing for horizontal ones, so
  either the layout goes one-directional or the other orientation needs its own exposure;
- the mask error enhancement factor (MEEF) grows at low k₁, so mask CD errors are
  amplified on the wafer.

Hence the working rule used throughout this tour: **one ArF-immersion exposure prints about
76–80 nm pitch**. Anything tighter has to be split across several exposures or built with
spacers. In "Seeing Double" (IEEE Spectrum, November 2008), Chris Mack put the smallest period
possible with 193 nm light at NA 1.35 at "about 72 nm", right at the k₁ = 0.25 floor, and
concluded that "double-patterning lithography will be the only game in town" until a new
wavelength arrived.

<figure markdown="span">

![Contrast versus pitch on 193 nm immersion, 0.33 NA EUV and 0.55 NA EUV for conventional and dipole illumination; the dipole keeps contrast down to close to the k1 = 0.25 pitch on each tool, the conventional source loses it well before.](../assets/images/sim/site/site-k1-contrast-vs-pitch-oai-light.png#gh-light-mode-only)
![Contrast versus pitch on 193 nm immersion, 0.33 NA EUV and 0.55 NA EUV for conventional and dipole illumination; the dipole keeps contrast down to close to the k1 = 0.25 pitch on each tool, the conventional source loses it well before.](../assets/images/sim/site/site-k1-contrast-vs-pitch-oai-dark.png#gh-dark-mode-only)

<figcaption>Companion to the k<sub>1</sub>-versus-pitch chart: dense 1:1 lines on the page's three tools (193i: 193 nm, water NA 1.35; EUV: Sn LPP through the NA 0.33 and 0.55 presets), conventional σ 0.9 against an x-dipole whose poles sit at σ 0.75. The dipole puts all its light where a two-beam image forms and keeps contrast to just above the k<sub>1</sub> = 0.25 pitch (dashed); the conventional disc fades out near λ/(NA·1.9). On the High-NA preset the dipole's contrast falls again above ≈ 22 nm pitch, where first orders from the poles start to land in the assumed 0.2·NA central obscuration. Thin mask, isotropic pupils (High-NA obscuration 0.2·NA assumed, no anamorphic optics), no resist. Models: scalar Hopkins imaging ✅, illumination pupil fills ✅ (two-pole dipole), EUV projection optics 🔶, immersion ✅.</figcaption>
</figure>

## Node by node: the k₁ each tool would need

The table takes the tightest metal pitch reported for each generation (rounded; sources on
the generation pages) and computes the k₁ a single exposure would need. **Bold** entries are
below the 0.25 floor, where a single exposure is impossible.

| Generation (example) | Tightest metal pitch | 193i, NA 1.35 | EUV, NA 0.33 | High-NA, NA 0.55 | How it was (or is planned to be) patterned |
|---|---|---|---|---|---|
| 32 nm (Intel) | 112.5 nm | 0.39 | — | — | ArF immersion, single exposure |
| 22 nm (Intel) | 80 nm | 0.28 | — | — | ArF immersion, single patterning |
| 16/14 nm (TSMC, Samsung) | ≈ 64–67 nm | **0.22–0.23** | — | — | double patterning |
| 14 nm (Intel) | 52 nm | **0.18** | — | — | self-aligned double patterning |
| 10 nm class | 36–51 nm | **0.13–0.18** | — | — | SADP, SAQP or triple patterning |
| 7 nm (TSMC N7) | 40 nm | **0.14** | 0.49 | — | spacer multipatterning on N7; EUV on some layers from N7+ |
| 5 nm (TSMC N5) | ≈ 28–30 nm | **0.10** | 0.34–0.37 | — | EUV on more than ten layers; tightest (M0) reportedly double patterned |
| 3 nm (TSMC N3E) | 23 nm | **0.08** | 0.28 | 0.47 | EUV; the first N3 double-patterned some critical layers |
| IRDS 2025 ("2 nm") | 20–22 nm | **0.07–0.08** | **0.24**–0.27 | 0.41–0.45 | IRDS: 193i and EUV double patterning |
| IRDS 2028–2033 | 16 nm | **0.06** | **0.20** | 0.33 | IRDS: High-NA single patterning |
| IRDS 2035+ (2024 edition) | 14 nm | **0.05** | **0.17** | 0.29 | IRDS: High-NA (with multipatterning) |

IRDS rows use the International Roadmap for Devices and Systems More Moore and Lithography
tables (2022 and 2024 editions); roadmap years are projections, not announcements.

Notice that EUV at 23–30 nm pitch sits well above the optical floor, yet production still
double-patterned some of those layers. Optics is not the binding limit there: photon shot noise
and resist stochastics are (see [Counting photons](#counting-photons) below). In volume
production, 0.33-NA EUV has printed about 30–36 nm pitch in a single exposure.

### What multipatterning buys, in k₁

Splitting a grating across exposures relaxes the pitch each exposure has to print:

| Scheme | Pitch printed by the lithography step | k₁ for a 36 nm final pitch on 193i |
|---|---|---|
| Single exposure | $p$ | 0.13 (impossible) |
| LELE double patterning (two masks) | $2p$ per mask | 0.25 per mask (at the floor) |
| SADP (one mandrel mask + spacer) | $2p$ mandrel pitch | 0.25 |
| SAQP (one mandrel mask + two spacers) | $4p$ mandrel pitch | 0.50 |

Spacer schemes make the lithography easy again but hand the problem to deposition and etch
(spacer thickness and mandrel CD now set line positions), and they print only regular
gratings: the actual circuit is carved out afterwards by extra "cut" or "block" masks, which
are themselves among the hardest exposures. The [22 nm to 10 nm page](22-to-10nm.md) walks
through the flows.

## Depth of focus

The Rayleigh depth of focus scales as

```math
\mathrm{DOF} = k_2 \frac{\lambda}{\mathrm{NA}^2} .
```

This paraxial form is good enough to compare the two EUV systems: $\lambda/\mathrm{NA}^2$ =
124 nm at NA 0.33 and 44.6 nm at NA 0.55, so at equal $k_2$ the High-NA focus budget is
$(0.55/0.33)^2 \approx 2.8$ times smaller (2.9 times with B. J. Lin's non-paraxial formula,
which scales with $\lambda/\sin^2(\theta/2)$ instead of $\lambda/\mathrm{NA}^2$). That is why
High-NA processes use thinner resists and tighter focus control, and the IRDS lists "solutions
for meeting small depths-of-focus at 0.55 NA" among the key High-NA challenges. At NA 1.35 the
paraxial formula is not reliable at all (the angles in the resist are large); Lin's non-paraxial
equations were developed for exactly that case.

## Counting photons

A photon of wavelength λ carries $E_\text{ph} = hc/\lambda$ = 1239.84 eV·nm / λ: 6.42 eV
at 193 nm and 91.8 eV at 13.5 nm. Since 1 mJ/cm² = 10⁻¹⁷ J/nm², the photon areal density
per unit dose is

```math
n = \frac{10^{-17}\ \mathrm{J\ nm^{-2}}}{E_\text{ph}}
= 9.72\ \mathrm{nm^{-2}}\ \text{at 193 nm},
\qquad 0.680\ \mathrm{nm^{-2}}\ \text{at 13.5 nm}
\qquad(\text{per mJ/cm}^2).
```

At the same dose, EUV delivers **14.3× fewer photons** than ArF, simply because each photon
carries 14.3× more energy. Photon arrival is a Poisson process, so the relative fluctuation
of the count $N$ in an area is $\sigma_N/N = 1/\sqrt{N}$:

| Case | Area | Dose (mJ/cm²) | Incident photons $N$ | 3σ count noise |
|---|---|---|---|---|
| ArF | (20 nm)² | 30 | 116,590 | 0.9 % |
| EUV, NXE throughput-spec dose | (20 nm)² | 30 | 8,155 | 3.3 % |
| EUV, EXE throughput-spec dose | (20 nm)² | 50 | 13,592 | 2.6 % |
| High-NA-sized contact | (12 nm)² | 50 | 4,893 | 4.3 % |

(30 and 50 mJ/cm² are the doses at which ASML quotes NXE:3600D/3800E and EXE throughput;
real processes choose their own doses.) These are *incident* photons. A resist film of
thickness $d$ and absorption coefficient $\alpha$ absorbs a fraction $A = 1 - e^{-\alpha d}$ of
them. Organic chemically amplified resists absorb roughly 5 µm⁻¹ at 13.5 nm, so a 35 nm film
takes up only about 16 %; tin-based metal-oxide resists, at roughly 10–20 µm⁻¹, take up about
30–50 % (absorption data: Fallica et al. 2018; MacKenzie et al. 2025).
The absorbed count is $A N$, and the noise grows by $1/\sqrt{A}$: 2.5× for $A = 0.16$. Secondary
electrons, acid generation and diffusion then add their own randomness; the
[7 nm and 5 nm page](7-and-5nm.md) covers what that does to real patterns, and
[the stochastic frontier](../future/stochastic-frontier.md) where it leads next. The scaling is
unforgiving: to keep the same relative roughness, a 0.7× shrink in CD needs about three times
the dose (C. A. Mack, 2018).

<figure markdown="span">
  ![Two panels. Left: incident photons on a 20 nm by 20 nm area against dose, log scale; the ArF line sits about 14 times above the EUV line; at 30 mJ/cm² EUV gives 8,155 photons and at 50 mJ/cm² 13,592. Right: the corresponding 3-sigma photon-count noise, 3.3 percent and 2.6 percent for EUV at those doses and under 1 percent for ArF.](../assets/images/nodes/photons-vs-dose-light.svg#only-light){ width="760" }
  ![Two panels. Left: incident photons on a 20 nm by 20 nm area against dose, log scale; the ArF line sits about 14 times above the EUV line; at 30 mJ/cm² EUV gives 8,155 photons and at 50 mJ/cm² 13,592. Right: the corresponding 3-sigma photon-count noise, 3.3 percent and 2.6 percent for EUV at those doses and under 1 percent for ArF.](../assets/images/nodes/photons-vs-dose-dark.svg#only-dark){ width="760" }
  <figcaption>Photon shot noise at ArF and EUV doses, from E = hc/λ and Poisson statistics.
  Dots: the doses at which ASML quotes NXE and EXE throughput. Generated by
  <code>docs/figures/nodes/make_node_figures.py</code>.</figcaption>
</figure>

highuvlith's stochastic module derives its photon density from the source wavelength in the
same way, and a regression test pins the 13.5 nm value to the 0.68 photons/nm² per mJ/cm²
used in the EUV literature ([research modules](../research-modules.md)).

## Overlay and edge-placement budgets

Overlay is how accurately one layer lands on the previous one. The IRDS 2022 Lithography
tables set these targets for leading-edge logic (all 3σ, nm):

| IRDS 2022 target year | 2022 | 2025 | 2028 |
|---|---|---|---|
| Ground rules (gate pitch, metal pitch) | G48M24 | G45M20 | G42M16 |
| Minimum metal half-pitch | 12 | 10 | 8 |
| Overlay | 2.4 | 2.0 | 1.6 |
| Metal CD uniformity | 1.8 | 1.5 | 1.2 |
| Metal line-width roughness (LWR) | 1.8 | 1.5 | 1.2 |
| Metal line-edge roughness (LER) | 1.3 | 1.1 | 0.8 |

The overlay target is 20 % of the minimum half-pitch in every column. For scale, ASML
specifies the NXE:3400C at 1.4 nm dedicated-chuck and 1.5 nm matched-machine overlay, and the
NXT:2000i immersion scanner at 1.4 nm and 2.0 nm; the [22 nm to 10 nm page](22-to-10nm.md)
shows how those numbers improved over a decade.

What ultimately matters is **edge placement error (EPE)**: how far a printed edge lands from
where the design wants it. The IRDS describes EPE as the sum of local variability terms
(LER/LWR, local CD uniformity) and global ones (overlay), and cites simulations in which
overlay takes about 40 % of the budget and LER about 25 %. A simplified way to see the
scale is to add the three biggest terms in quadrature:

```math
\mathrm{EPE}_{3\sigma} \approx \sqrt{\mathrm{OVL}^2 + \left(\tfrac{\mathrm{CDU}}{2}\right)^2 + \mathrm{LER}^2}
= \sqrt{2.4^2 + 0.9^2 + 1.3^2} \approx 2.9\ \mathrm{nm}
```

for the 2022 targets, about a quarter of the 12 nm half-pitch. Real budgets carry more terms
(OPC residuals, mask errors, pitch walk in spacer schemes, etch bias), so treat this only as
an order of magnitude. It explains why self-aligned schemes win at tight pitch: in
litho-etch-litho-etch double patterning an overlay error moves one set of lines relative to
the other and turns directly into a space-width error, whereas a spacer-defined line pair
cannot be misaligned relative to itself.

!!! note "What highuvlith does and does not model here"
    The aerial image is a scalar Hopkins/SOCS calculation (✅; flagged 🔶 above NA ≈ 0.8,
    where polarization matters, so ArF-immersion results at NA 1.35 are optimistic; see the
    [vector imaging](../vector-imaging.md) page for the polarization-aware path). The mask is
    thin (Kirchhoff): EUV mask 3D effects are not modeled, and the High-NA preset is an
    isotropic wafer-side model with a circular pupil (the anamorphic 4×/8× mask-side
    magnification is not modeled). Photon shot noise and dose jitter are ✅; the LELE
    double-patterning module is 🔶 (two exposures with an overlay shift, no resist chemistry
    between them). Current badges live in the [capability matrix](../capability-matrix.md).

## Try it in highuvlith

!!! example "Contrast versus pitch on three generations of tools"
    Image equal lines and spaces at four pitches with a conventional σ = 0.9 source. On
    each tool the contrast falls as the pitch shrinks and drops to zero once
    $\lambda/p > \mathrm{NA} (1+\sigma)$: below about 75 nm on 193i, 22 nm on EUV and 13 nm
    on High-NA. At 80 nm, just above its cut-off, 193i keeps only a few percent of contrast
    with this source: production reaches 76–80 nm with dipole or C-quad illumination, which
    puts the light at the angles that form a two-beam image.

    <!-- verify-example -->
    ```python
    import highuvlith as huv

    euv = huv.SourceConfig.lpp_sn_13nm5(0.9)
    tools = {
        "193i NA 1.35": (huv.SourceConfig.arf_laser(0.9), huv.OpticsConfig.immersion_193i()),
        "EUV NA 0.33": (euv, huv.OpticsConfig.euv_nxe()),
        "EUV NA 0.55": (euv, huv.OpticsConfig.euv_high_na()),
    }
    for name, (src, opt) in tools.items():
        row = []
        for pitch in (80.0, 40.0, 28.0, 20.0):
            mask = huv.MaskConfig.line_space(pitch / 2, pitch)
            eng = huv.SimulationEngine(src, opt, mask, grid=huv.GridConfig(64, pitch / 32))  # 2 periods
            row.append(round(eng.compute_aerial_image(focus_nm=0.0).image_contrast(), 2))
        print(name, row)
    ```

    ??? success "Output"

        ```text
        193i NA 1.35 [0.04, 0.0, 0.0, 0.0]
        EUV NA 0.33 [0.93, 0.75, 0.32, 0.0]
        EUV NA 0.55 [0.95, 0.84, 0.77, 0.58]
        ```

!!! example "Photons per contact, from the source model"
    The photon density follows from the source wavelength alone; Poisson sampling gives the
    contact-to-contact spread. highuvlith's stochastic module runs the full version of this
    (Poisson noise per pixel plus Gamma-distributed dose jitter, turned into LER/LWR) in Rust;
    from Python it is `huv.compute_ler_lwr` (🔶).

    <!-- verify-example -->
    ```python
    import numpy as np
    import highuvlith as huv

    rng = np.random.default_rng(1)
    for src in (huv.SourceConfig.arf_laser(0.9), huv.SourceConfig.lpp_sn_13nm5(0.9)):
        e_ph = 1239.84193 / src.wavelength_nm * 1.602176634e-19   # J per photon
        per_nm2 = 1e-17 / e_ph                                     # photons/nm² per mJ/cm²
        for dose in (30.0, 60.0):
            mean = dose * per_nm2 * 20.0**2                        # (20 nm)² contact
            n = rng.poisson(mean, 20_000)                          # 20,000 contacts
            print(f"{src.wavelength_nm:6.1f} nm  {dose:3.0f} mJ/cm²  N = {mean:9,.0f}"
                  f"  3σ = {300 * n.std() / n.mean():.2f} %")
    ```

    ??? success "Output"

        ```text
         193.4 nm   30 mJ/cm²  N =   116,812  3σ = 0.87 %
         193.4 nm   60 mJ/cm²  N =   233,625  3σ = 0.62 %
          13.5 nm   30 mJ/cm²  N =     8,155  3σ = 3.29 %
          13.5 nm   60 mJ/cm²  N =    16,311  3σ = 2.35 %
        ```

## Key takeaways

- $k_1 = (p/2) \mathrm{NA}/\lambda$; single exposure of a dense grating needs
  $k_1 \ge 0.25$, and production sits at about 0.27–0.33.
- One ArF-immersion exposure prints about 76–80 nm pitch. Every leading-edge node after Intel's
  22 nm has needed either multipatterning or EUV for its tightest metal.
- EUV (λ/NA = 40.9 nm) reset k₁ for 7 and 5 nm-class metal; in production, stochastics rather
  than optics limit its single exposure to about 30–36 nm pitch. High-NA (λ/NA = 24.5 nm) resets
  it again for roadmap pitches of 16–20 nm, at the price of about 2.8× less depth of focus and a
  half-size field.
- EUV photons carry 14× more energy, so the same dose brings 14× fewer of them; at
  (20 nm)² and 30 mJ/cm² that is about 8,000 incident photons and a 3 % 3σ count noise
  before the resist even absorbs them, and a typical organic resist absorbs only about a sixth.
- Roadmap overlay targets are about 20 % of the half-pitch (2.4 nm at 12 nm half-pitch);
  edge placement, not resolution alone, sets what can be manufactured.

## References and further reading

- ASML product pages (specifications quoted above, retrieved 2026-09-30):
  [NXT:2000i](https://www.asml.com/en/products/duv-lithography-systems/twinscan-nxt2000i),
  [NXT:2100i](https://www.asml.com/en/products/duv-lithography-systems/twinscan-nxt2100i),
  [NXT:2150i](https://www.asml.com/en/products/duv-lithography-systems/twinscan-nxt-2150i),
  [NXT:1980Di](https://www.asml.com/en/products/duv-lithography-systems/twinscan-nxt1980di),
  [NXT:1470](https://www.asml.com/en/products/duv-lithography-systems/twinscan-nxt1470),
  [XT:1460K](https://www.asml.com/en/products/duv-lithography-systems/twinscan-xt1460k),
  [XT:860N](https://www.asml.com/en/products/duv-lithography-systems/twinscan-xt860n),
  [NXT:870](https://www.asml.com/en/products/duv-lithography-systems/twinscan-nxt870),
  [XT:400M](https://www.asml.com/en/products/duv-lithography-systems/twinscan-xt-400m),
  [NXE:3400C](https://www.asml.com/en/products/euv-lithography-systems/twinscan-nxe3400c),
  [NXE:3600D](https://www.asml.com/en/products/euv-lithography-systems/twinscan-nxe-3600d),
  [NXE:3800E](https://www.asml.com/en/products/euv-lithography-systems/twinscan-nxe-3800e),
  [EXE:5000](https://www.asml.com/en/products/euv-lithography-systems/twinscan-exe-5000),
  [EXE:5200B](https://www.asml.com/en/products/euv-lithography-systems/twinscan-exe-5200b).
- J. de Klerk et al., "Performance of a 1.35NA ArF immersion lithography system for 40-nm
  applications," *Proc. SPIE* **6520**, 65201Y (2007),
  [doi:10.1117/12.712094](https://doi.org/10.1117/12.712094).
- C. A. Mack, "Seeing Double," *IEEE Spectrum*, November 2008
  ([link](https://spectrum.ieee.org/seeing-double)).
- IEEE IRDS, *International Roadmap for Devices and Systems, 2022 Edition: Lithography*,
  tables LITH-1 and LITH-2 ([xlsx](https://irds.ieee.org/images/files/pdf/2022/2022IRDS_Litho_Tables.xlsx)).
- IEEE IRDS, *2023 Edition: Lithography* (EPE discussion, section 5.2)
  ([pdf](https://irds.ieee.org/images/files/pdf/2023/2023IRDS_Litho.pdf)).
- IEEE IRDS, *More Moore* tables, 2022 and 2024 editions
  ([2022 xlsx](https://irds.ieee.org/images/files/pdf/2022/2022IRDS_MM_Tables.xlsx),
  [2024 xlsx](https://irds.ieee.org/images/files/pdf/2024/2024IRDS_MM_Tables.xlsx)).
- B. J. Lin, "The k3 coefficient in non-paraxial λ/NA scaling equations for resolution,
  depth of focus, and immersion lithography," *J. Micro/Nanolith. MEMS MOEMS* **1**(1), 7–12
  (2002), [doi:10.1117/1.1445798](https://doi.org/10.1117/1.1445798).
- P. De Bisschop, "Stochastic effects in EUV lithography: random, local CD variability, and
  printing failures," *J. Micro/Nanolith. MEMS MOEMS* **16**(4), 041013 (2017),
  [doi:10.1117/1.JMM.16.4.041013](https://doi.org/10.1117/1.JMM.16.4.041013).
- C. A. Mack, "Shot noise: a 100-year history, with applications to lithography,"
  *J. Micro/Nanolith. MEMS MOEMS* **17**(4), 041002 (2018),
  [doi:10.1117/1.JMM.17.4.041002](https://doi.org/10.1117/1.JMM.17.4.041002).
- R. Fallica et al., "Absorption coefficient of metal-containing photoresists in the extreme
  ultraviolet," *J. Micro/Nanolith. MEMS MOEMS* **17**(2), 023505 (2018),
  [doi:10.1117/1.JMM.17.2.023505](https://doi.org/10.1117/1.JMM.17.2.023505).
- H. K. MacKenzie et al., "Bismuth-oxo clusters for next generation extreme ultraviolet light
  photolithography," *Chem. Mater.* **37**(22), 9116–9123 (2025),
  [doi:10.1021/acs.chemmater.5c01797](https://doi.org/10.1021/acs.chemmater.5c01797).
- C. A. Mack, *Fundamental Principles of Optical Lithography: The Science of
  Microfabrication* (Wiley, 2007), [doi:10.1002/9780470723876](https://doi.org/10.1002/9780470723876).
- H. J. Levinson, *Principles of Lithography*, 4th ed. (SPIE Press, 2019),
  [doi:10.1117/3.2525393](https://doi.org/10.1117/3.2525393).
- B. J. Lin, *Optical Lithography: Here Is Why*, 2nd ed. (SPIE Press, 2021),
  [doi:10.1117/3.2586123](https://doi.org/10.1117/3.2586123).
