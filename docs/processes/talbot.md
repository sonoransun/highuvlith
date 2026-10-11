---
description: "Talbot and EUV interference lithography in highuvlith: the scalar angular-spectrum Talbot carpet, displacement (DTL) and achromatic (ATL) stationary images with half the mask period, and two-grating EUV-IL fringes of period p/2m — with the thin-mask and coherence approximations stated."
---

# Talbot lithography

**Status:** 🔶 Simplified — scalar Talbot carpets, DTL/ATL stationary images and two-grating EUV-IL fringes; thin Kirchhoff grating, perfect coherence, λ-independent coefficients · [matrix](../capability-matrix.md#deep-layer-and-non-projection-processes)

<p class="huv-lead">A periodic grating lit by a coherent plane wave re-images itself every Talbot length, with a half-period-shifted copy halfway between — a lens-less printer for periodic arrays at EUV wavelengths. A fixed gap is fragile, so displacement and achromatic Talbot lithography scan the gap or the spectrum and leave a stationary image with half the mask period. <code>talbot.rs</code> computes these and two-grating EUV interference.</p>

<div class="huv-glance" markdown>

| Property | Value |
|---|---|
| Computes | grating orders; carpets; DTL/ATL stationary images; EUV-IL fringes |
| Does not | mask 3-D; λ-dependent coefficients; partial coherence; resist Fresnel |
| Default scene | `examples/talbot.toml`: 100 nm grating, 13.5 nm, z_T 1481 nm |
| Grade | 🔶 Simplified |
| Compare with | [Interference & two-photon](interference-volumetric.md) · [Alternatives to projection](../future/alternative-patterning.md) |

</div>

<div class="huv-physics" markdown>

<!-- talbot-schemes -> processes/talbot.md -->
<figure markdown="span">

![Four schemes behind a transmission grating: a fixed gap inside the Talbot carpet, displacement Talbot with the gap scanned over one Talbot length, achromatic Talbot beyond z_A = 2p²/Δλ with a broadband source, and two gratings whose +m and −m orders cross at ±θ to print fringes of period p/2m.](../assets/images/diagrams/talbot-schemes-light.svg#gh-light-mode-only)
![Four schemes behind a transmission grating: a fixed gap inside the Talbot carpet, displacement Talbot with the gap scanned over one Talbot length, achromatic Talbot beyond z_A = 2p²/Δλ with a broadband source, and two gratings whose +m and −m orders cross at ±θ to print fringes of period p/2m.](../assets/images/diagrams/talbot-schemes-dark.svg#gh-dark-mode-only)

<figcaption><strong>Mechanism.</strong> A grating re-images itself every Talbot length z_T = 2p²/λ with a half-period-shifted copy in between, so a fixed gap is fragile; displacement Talbot scans the gap over z_T and achromatic Talbot uses a band Δλ beyond z_A = 2p²/Δλ, both leaving a stationary image of period p/2. Two gratings whose +m and −m orders cross at ±θ print fringes of period p/2m whatever the wavelength. <em>Schematic, not to scale.</em></figcaption>
</figure>

<!-- talbot-carpet -> processes/talbot.md -->
<figure markdown="span">

![A Talbot carpet: the intensity behind a 100 nm grating, 25 percent open, lit at 13.5 nm repeats itself at the Talbot length and shows shifted and frequency-doubled images in between; beside it, the displacement Talbot image averaged over the gap has half the grating period.](../assets/images/sim/talbot-carpet-light.png#gh-light-mode-only)
![A Talbot carpet: the intensity behind a 100 nm grating, 25 percent open, lit at 13.5 nm repeats itself at the Talbot length and shows shifted and frequency-doubled images in between; beside it, the displacement Talbot image averaged over the gap has half the grating period.](../assets/images/sim/talbot-carpet-dark.png#gh-dark-mode-only)

<figcaption><strong>Talbot self-imaging at 13.5 nm.</strong> (a) Intensity behind a 100 nm-period binary amplitude grating, 25 % open (±10 orders, exact angular-spectrum propagation): the grating re-images at the Talbot length (2p²/λ ≈ 1481 nm paraxially, 1475 nm exactly for the ±1 orders) with the half-shifted and doubled images in between; a 50 % open grating is uniform at z<sub>T</sub>/4 and has no doubled image. (b) Averaging over a scanned gap (displacement Talbot lithography) gives a stationary image with half the period. <em>Model: Talbot / DTL / ATL 🔶 (scalar thin mask, coherent normal illumination, wavelength-independent grating coefficients).</em></figcaption>
</figure>

<!-- huv-table: stack -->
| What is modeled | Grade | Dominant approximation | Details |
|---|---|---|---|
| Talbot / DTL / ATL proximity lithography | 🔶 Simplified | thin Kirchhoff grating, coherent normal illumination | [Propagation](#propagation-and-the-talbot-length) |
| Two-grating EUV interference lithography | 🔶 Simplified | ideal TE beams, no zero-order background | [EUV-IL](#two-grating-euv-interference) |
| Multi-beam interference / two-photon | ✅ Implemented (🔶 absorption, interfaces) | the module that records the EUV-IL fringes | [Interference](interference-volumetric.md) |
| 3D development (fast marching) | ✅ Implemented | static rate field (downstream) | [Volumetric exposure](volumetric-exposure.md#development-tier-2-fast-marching) |

**Same thing elsewhere:** Python `api.simulate_euv_il(100.0, 13.5, order=1)` · CLI `deep --config examples/talbot.toml` · TOML `[deep] talbot_mode = "atl"` · GUI volume viewer · physics → [interference](interference-volumetric.md)

</div>

## How it works

The grating's Fourier orders are grouped into shells of equal $|\mathbf f|$, propagated with the exact or paraxial angular-spectrum transfer function, and summed coherently (a carpet), over a scanned gap (DTL) or over a spectrum (ATL); the volume then enters the resist with its own $k_z$ and absorption.

<details markdown="1">
<summary>Flowchart — from grating orders to the development tiers</summary>

```mermaid
flowchart LR
    G["Grating t(x,y)<br/>Fourier orders c_nm"] --> S["Group orders into<br/>equal-|f| shells"]
    S --> P["Angular-spectrum transfer<br/>exp(i(k_z − k)z)"]
    P --> C["Coherent image / carpet<br/>|Σ P_s U_s|²"]
    P --> D["DTL: gap average<br/>sinc(δL/2π) shell weights"]
    P --> A["ATL: spectral average<br/>sinc / Gaussian envelopes"]
    C --> R["Resist volume<br/>(in-resist k_z, absorption)"]
    D --> R
    A --> R
    G2["Two gratings,<br/>+m / −m orders"] --> IL["InterferenceSetup<br/>Λ = p/(2m)"]
    R --> E["interference::expose<br/>→ PAC"]
    IL --> E
    E --> V["volumetric development tiers"]
```

</details>

### Grating orders

The mask is a thin (Kirchhoff) transmission `t(x, y) = Σ c_nm exp(2πi(n x/p_x + m y/p_y))`. For a binary 1D grating whose "line" of width $f p$ (transmission $t_l$) is centred at $x = 0$ in a space of transmission $t_s$:

```math
c_0 = t_s + (t_l - t_s)\,f, \qquad c_n = (t_l - t_s)\, f\, \mathrm{sinc}(n f), \qquad \mathrm{sinc}(u) = \frac{\sin \pi u}{\pi u}.
```

An **amplitude** grating is $t_l = 1, t_s = 0$ (open fraction $f$); a **phase** grating is $t_l = e^{i\varphi}, t_s = 1$, and a π step at 50 % duty cycle suppresses the zeroth order ($|c_{\pm1}|^2 = 4/\pi^2 \approx 0.405$). Circular-hole arrays use the exact Airy form factor

```math
c_{nm} = \frac{\pi r^2}{A}\, S_{nm}\, \frac{2 J_1(2\pi r |\mathbf f|)}{2\pi r |\mathbf f|},
```

with $A$ the unit-cell area and $S_{nm}$ the structure factor of the holes in the cell (the hexagonal lattice uses the rectangular supercell $a \times \sqrt3 a$ with $S = 1 + (-1)^{n+m}$). Arbitrary unit cells are sampled and transformed exactly as pixelated (piecewise-constant) cells.

### Propagation and the Talbot length

Each order is a plane wave of transverse frequency $\mathbf f = (n/p_x, m/p_y)$. Dropping the common carrier $e^{ikz}$, the exact scalar (angular-spectrum) transfer through a medium of index $n_g$ is

```math
U(\mathbf r, z) = \sum_{nm} c_{nm}\, e^{2\pi i \mathbf f\cdot\mathbf r}\, e^{i(k_z - k)z}, \qquad k_z - k = -\frac{2\pi |\mathbf f|^2}{n_g/\lambda + \sqrt{(n_g/\lambda)^2 - |\mathbf f|^2}},
```

(orders with $|\mathbf f| > n_g/\lambda$ are evanescent and decay), or in the paraxial (Fresnel) approximation $k_z - k = -\pi\lambda|\mathbf f|^2/n_g$. In the paraxial model every order rephases after the **Talbot length**, and half-way the image is shifted by $p/2$:

```math
z_T = \frac{2p^2}{\lambda}, \qquad U(x, z_T/2) = U(x + p/2, 0).
```

The exact rephasing length of the 0 and ±1 orders is

```math
z_T^{\text{exact}} = \frac{\lambda}{1 - \sqrt{1 - \lambda^2/p^2}} = \frac{p^2}{\lambda}\left(1 + \sqrt{1 - \lambda^2/p^2}\right) = \frac{2p^2}{\lambda}\left(1 - \frac{\lambda^2}{4p^2} + \dots\right).
```

**It is an exact period of the field only when those are the only propagating orders ($p < 2\lambda$) or for a sinusoidal grating; with more propagating orders the non-paraxial field is quasi-periodic**, so the revivals of a ±10-order carpet at 13.5 nm / 100 nm are approximate — the fly-through below (a 25 % open grating: a 50 % grating is uniform at $z_T/4$ and has no period-halved image) pauses where the *paraxial* carpet halves its period, shifts and re-images.

<!-- talbot-flythrough-anim -> processes/talbot.md -->
<figure markdown="span">

![Animation: a cursor sweeps through the Talbot carpet behind a 100 nm grating (25 percent open) at 13.5 nm, its intensity profile drawn at right; it pauses where the period halves, where the image shifts by half a period and at the Talbot length, then a gap average becomes the half-period DTL image.](../assets/images/sim/anim/talbot-flythrough-anim-light.svg#gh-light-mode-only)
![Animation: a cursor sweeps through the Talbot carpet behind a 100 nm grating (25 percent open) at 13.5 nm, its intensity profile drawn at right; it pauses where the period halves, where the image shifts by half a period and at the Talbot length, then a gap average becomes the half-period DTL image.](../assets/images/sim/anim/talbot-flythrough-anim-dark.svg#gh-dark-mode-only)

<figcaption><strong>Flying through a Talbot carpet.</strong> Intensity behind a 100 nm binary amplitude grating, 25 % open, at 13.5 nm (±10 orders, exact angular-spectrum propagation; 128 × 240 engine samples); the cursor's row is drawn at right and pauses at z<sub>T</sub>/4, z<sub>T</sub>/2 and z<sub>T</sub>, where the paraxial carpet halves its period (a 50 % open grating would be uniform there), shifts by p/2 and re-images the grating (z<sub>T</sub> = 1475 nm exact for the ±1 orders, 1481 nm paraxial) — at λ/p = 0.135 the higher orders are non-paraxial, so the exact revivals are approximate; past 3z<sub>T</sub>/2 the running average over the scanned gap converges to the displacement-Talbot image with half the period. <em>Model: Talbot / DTL / ATL 🔶 (scalar thin mask, coherent normal illumination, wavelength-independent grating coefficients); computed, not measured.</em></figcaption>
</figure>

How much the higher orders matter is the difference between the exact and the paraxial carpet:

<!-- site-talbot-exact-vs-paraxial -> processes/talbot.md -->
<figure markdown="span">

![The Talbot carpet behind a 100 nm grating at 13.5 nm from exact propagation and its difference from the paraxial carpet, which is large in the fine structure carried by the higher orders; beside them the DTL image with half the period and the coherent row.](../assets/images/sim/site/site-talbot-exact-vs-paraxial-light.png#gh-light-mode-only)
![The Talbot carpet behind a 100 nm grating at 13.5 nm from exact propagation and its difference from the paraxial carpet, which is large in the fine structure carried by the higher orders; beside them the DTL image with half the period and the coherent row.](../assets/images/sim/site/site-talbot-exact-vs-paraxial-dark.png#gh-dark-mode-only)

<figcaption><strong>Exact against paraxial Talbot carpets.</strong> A 100 nm binary amplitude grating at 13.5 nm, ±10 orders — the alternative-patterning page's example: (a) exact angular-spectrum propagation, (b) its difference from the paraxial (Fresnel) carpet, both in units of the paraxial Talbot length 2p²/λ ≈ 1481 nm (dashed); with p/λ ≈ 7 the higher orders dephase in the exact propagation, so the fine structure differs strongly even though the 0/±1 self-image only moves to ≈ 1475 nm; (c) the stationary DTL image at half the period. <em>Model: Talbot / DTL 🔶 (scalar thin mask, coherent normal illumination).</em></figcaption>
</figure>

### Shell formulation of incoherent averages

The transfer function depends on $|\mathbf f|$ only, so orders are grouped into *shells* $s$ of equal $|\mathbf f|$ with shell fields $U_s(\mathbf r) = \sum_{a \in s} c_a e^{2\pi i \mathbf f_a\cdot\mathbf r}$. Every exposure mode is then $I(\mathbf r) = \sum_{st} W_{st} U_s U_t^*$ with the Hermitian weight matrix $W_{st} = \langle P_s P_t^*\rangle$:

- **Coherent**: $W = P P^\dagger$ at the gap $g$.
- **DTL** (gap uniformly scanned over $[g, g + L]$), a closed form: $W_{st} = P_s P_t^*\big|_{g + L/2}\ \mathrm{sinc}(\delta_{st} L / 2\pi)$, $\delta_{st} = \partial_g(\phi_s - \phi_t)$. In the paraxial model $\delta_{st} L/2\pi = -(n_s^2 - n_t^2)$ for $L = z_T$, so every cross term between different shells vanishes; $L \to \infty$ gives the **stationary image**

```math
I_\text{DTL}(\mathbf r) = \sum_s |U_s(\mathbf r)|^2 \;\xrightarrow{\ \text{1D}\ }\; \sum_n |c_n|^2 + 2\,\mathrm{Re}\sum_{n>0} c_n c_{-n}^* e^{4\pi i n x/p},
```

whose period is $p/2$.
- **ATL** (spectral average at gap $g$). Paraxially the cross-term phase is linear in λ, $\phi_s - \phi_t = -\pi\lambda z (f_s^2 - f_t^2)$, so for a flat-top band of full width $\Delta\lambda$ and $f^2 = n^2/p^2$:

```math
\left\langle e^{-i\pi\lambda z D/p^2}\right\rangle = e^{-i\pi\lambda_0 z D/p^2}\,\mathrm{sinc}\!\left(\frac{D\,\Delta\lambda\, z}{2p^2}\right), \qquad D = n^2 - n'^2,
```

whose first zero for the slowest term ($D = 1$, orders 0/±1) defines the **achromatic distance**

```math
z_A = \frac{2p^2}{\Delta\lambda}.
```

For a Gaussian band of rms width $\sigma_\lambda$ the envelope is $\exp[-(\pi\sigma_\lambda z D/p^2)^2/2]$ (≈ 0.03 at $z = z_A$ for FWHM $= \Delta\lambda$). Beyond a few $z_A$ the ATL image equals the stationary DTL image — panel (a) of the figure under EUV-IL below. With exact propagation the phase is not linear in λ; the spectrum is split into bins, each treated with a linear phase across the bin (a sinc-weighted midpoint rule that damps, rather than aliases, rapidly varying high-order terms).

### Recording in the resist

Inside a resist of real index $n_r$ (which may be < 1 at EUV) each order continues with its own $k_z$ at $\lambda/n_r$ and decays as $\exp(-\alpha z/(2\cos\theta))$ along its slanted path — the convention of [interference-volumetric.md](./interference-volumetric.md). **No Fresnel transmission coefficients or back-reflections are applied.** Because the stationary DTL image only contains equal-$|\mathbf f|$ pairs, which share the in-resist $k_z$, it is depth-invariant apart from absorption: DTL/ATL prints have essentially unlimited depth of field.

### Two-grating EUV interference

The +m order of one grating and the −m order of a second grating of the same period overlap at $\pm\theta$ with $\sin\theta = m\lambda/p$ (requires $m\lambda < p$):

```math
\Lambda = \frac{\lambda}{2\sin\theta} = \frac{p}{2m}, \qquad V = \frac{2\sqrt{I_1 I_2}}{I_1 + I_2}, \qquad I_{1,2} = |c_{\pm m}|^2.
```

The fringe period is set by the grating alone — wavelength and resist index drop out (the same refraction invariance as [two-beam interference](./interference-volumetric.md#the-field-and-its-period)). For $m = 1$ the printed half-pitch is $p/4$, the relation used at the PSI EUV-IL beamline [7]. `TwoGratingInterference::to_interference_setup` turns the geometry into a two-beam, TE-polarized [`InterferenceSetup`](../../crates/highuvlith-core/src/interference.rs), so vector contrast and absorption are handled by that module.

<!-- talbot-atl-euvil -> processes/talbot.md -->
<figure markdown="span">

![Left: image contrast against the mask–wafer gap for a 0.3 nm band, settling on the stationary DTL value beyond the achromatic distance; right: two-grating EUV-IL fringes of period 50 and 25 nm from the first and second orders at 13.5 nm, the 6.7 nm first-order fringes coinciding with the 50 nm ones.](../assets/images/sim/talbot-atl-euvil-light.png#gh-light-mode-only)
![Left: image contrast against the mask–wafer gap for a 0.3 nm band, settling on the stationary DTL value beyond the achromatic distance; right: two-grating EUV-IL fringes of period 50 and 25 nm from the first and second orders at 13.5 nm, the 6.7 nm first-order fringes coinciding with the 50 nm ones.](../assets/images/sim/talbot-atl-euvil-dark.png#gh-dark-mode-only)

<figcaption><strong>Washing out the revivals, and fringes that ignore the wavelength.</strong> (a) Contrast of the spectrally averaged image behind a 100 nm grating for a 0.3 nm FWHM Gaussian band: beyond the achromatic distance z<sub>A</sub> = 2p²/Δλ it settles on the stationary DTL image, which the scanned gap reaches by averaging instead; (b) two-grating EUV-IL fringes from the ±1 and ±2 orders have periods p/2 and p/4 at 13.5 nm, and the ±1 fringes at 6.7 nm have the same 50 nm period. <em>Model: Talbot / DTL / ATL 🔶 · two-grating EUV-IL 🔶 (ideal gratings, no zero-order background); computed, not measured.</em></figcaption>
</figure>

## Process regime

Illustrative numbers computed with the formulas above (they are what `examples/talbot.toml` prints).

| Quantity | Example | Notes |
|---|---|---|
| Wavelength | 13.5 nm (EUV undulator / HHG / SXRL); UV lasers for DTL | ATL and EUV-IL need spatially coherent sources |
| Mask period $p$ | 100 nm (EUV); µm-scale for UV DTL | printed period $p/2$ (DTL/ATL, 1D) or $p/(2m)$ (EUV-IL) |
| Talbot length | $2p^2/\lambda$ = 1.48 µm (p = 100 nm, 13.5 nm) | exact 0/±1 value 1.474 µm |
| Achromatic distance | $2p^2/\Delta\lambda$ = 66.7 µm (Δλ = 0.3 nm) | proximity gaps of 10²-µm scale |
| First-order efficiency | $1/\pi^2 \approx 10$ % (50 % slits), $4/\pi^2 \approx 41$ % (π-phase) | sets the EUV-IL dose |
| EUV-IL angle | θ = 7.76° for m = 1, p = 100 nm, 13.5 nm | $\sin\theta = m\lambda/p$ |

**Nothing on this page is simulated or calibrated against an experiment**; the reported context below is for orientation only.

<details markdown="1">
<summary>Reported experimental context (for orientation only)</summary>

| Topic | Reported | Source |
|---|---|---|
| Achromatic Talbot lithography | 1D/2D patterns with 50 nm features at EUV; depth of field described as "practically unlimited" | [5] |
| Displacement Talbot lithography | gap-integrated image independent of the mask–wafer gap | [4] |
| Coherent (single-plane) Talbot printing | table-top 46.9 nm laser, up to the 6th Talbot plane (not achromatic) | [8] |
| EUV-IL half-pitch records (PSI) | 7 nm [9], 6 nm [10]; 5 nm reported in 2024 with mirror interference lithography (two mirrors + central stop) [11] | [9–11] |
| EUV-IL resist screening | down to 18 nm half-pitch across resist families | [7] |
| PSI XIL-II beamline | ≈ 3×10¹⁵ photons s⁻¹ cm⁻² per 4 % bandwidth at 92 eV, 5 × 5 mm² field (SLS 2.0 user operation expected from mid-2026) | [12] |

</details>

## What the code provides

All in [`crates/highuvlith-core/src/talbot.rs`](../../crates/highuvlith-core/src/talbot.rs):

- **`Grating`** — `period_x_nm`, `period_y_nm` (None for 1D), and the retained `orders: Vec<DiffractionOrder>` (`n`, `m`, complex `amplitude`). Constructors: `binary`, `binary_amplitude`, `binary_phase`, `sinusoidal_amplitude`, `from_coefficients`, `from_profile`, `separable`, `from_unit_cell`, `hole_array_square`, `hole_array_hexagonal` (hole arrays truncate orders on a circle so the retained set keeps the lattice symmetry). Helpers `coefficient`, `efficiency`, `total_efficiency`, `transmission`.
- **`TalbotSetup`** — grating + `wavelength_nm` + `gap_index` + `propagation` (`AngularSpectrum` default, or `Paraxial`). `talbot_length_nm`, `talbot_length_exact_nm`, `propagating_efficiency`, `field`, `intensity_plane`, `carpet` (optionally spectrally averaged), `exposure_image`, `resist_volume`.
- **`ExposureMode`** — `Coherent`, `Displacement { scan_length_nm }`, `DisplacementStationary`, `Achromatic(Spectrum)`; **`Spectrum`** — `FlatTop`, `Gaussian`, or discrete `Lines` (e.g. HHG harmonics, summed incoherently).
- **`ResistMedium`** — resist `index` and `absorption_per_nm`.
- **`TwoGratingInterference`** — period, wavelength, order, beam intensities, relative phase; constructors `new` (beam intensities from a `Grating`'s ±m orders) and `from_efficiencies`; `sin_theta`, `fringe_period_nm`, `diffraction_angle_deg`, `visibility`, `intensity_at`, `to_interference_setup`.
- Free functions: `talbot_length_paraxial_nm`, `talbot_length_exact_nm`, `achromatic_distance_nm`, `two_grating_fringe_period_nm`, `dominant_period_nm`, `sinc`, `bessel_j1`.
- Python: `simulate_talbot`, `simulate_euv_il` in `highuvlith.api`.

Intensities are normalized to the incident plane wave (a clear mask gives 1 everywhere). The `resist_volume` output feeds `interference::expose` (Dill one-photon map to PAC) and then the volumetric bake and development tiers ([volumetric-exposure.md](./volumetric-exposure.md)).

## Model coverage

| Field | Unit | Consumed by | Status |
|---|---|---|---|
| `Grating.period_x_nm` / `.period_y_nm` | nm | order frequencies, Talbot lengths | live |
| `Grating.orders[].amplitude` | — | shell fields $U_s$ | live |
| `TalbotSetup.wavelength_nm` | nm | transfer function, centre of the spectra | live |
| `TalbotSetup.gap_index` | — | $k_z$ in the gap, $z_T$ at $\lambda/n_g$ | live |
| `TalbotSetup.propagation` | enum | exact vs Fresnel transfer | live |
| `Spectrum::{FlatTop, Gaussian}.bins` | — | exact-propagation ATL quadrature (ignored by the paraxial closed forms) | live |
| `ResistMedium.index` / `.absorption_per_nm` | — / 1/nm | in-resist $k_z$, slanted-path decay | live |
| `TwoGratingInterference.beam_intensity_{1,2}`, `.relative_phase_rad` | — / rad | fringe visibility and position | live |
| Source spatial coherence / angular spread | — | — | planned |
| Wavelength-dependent grating coefficients (phase step ∝ 1/λ, EUV absorber n,k) | — | — | planned |
| Mask 3D / rigorous (EMF) grating diffraction | — | — | planned (non-goal for now, see roadmap) |
| Fresnel coefficients and substrate reflection at the resist | — | — | planned |
| Finite illuminated field (walk-off of high orders at large gaps) | — | — | planned |

## Usage

### Python

Wrappers in [`python/highuvlith/api.py`](../../python/highuvlith/api.py); the example prints the two Talbot lengths, the achromatic distance and the EUV-IL fringe period:

<!-- verify-example -->
```python
from highuvlith.api import simulate_euv_il, simulate_talbot

res = simulate_talbot(13.5, 100.0, bandwidth_nm=0.3, max_order=10, nx=128,
                      resist_thickness_nm=50.0, resist_index=0.97,
                      absorption_per_nm=0.004, exposure="stationary",
                      dose_scale=15.0, dill_c=0.1)
res.carpet               # (nz, nx) coherent Talbot carpet, rows = z
res.stationary_image     # DTL limit, period p/2
res.atl_image            # spectrally averaged image at res.gap_nm (= 2 z_A)
res.talbot_length_nm, res.achromatic_distance_nm
res.pac_volume.depth_map(0.5)   # VolumetricResult → development

il = simulate_euv_il(100.0, 13.5, order=1, intensity_ratio=0.8)
il.fringe_period_nm, il.visibility, il.intensity.values
print(f"z_T = {res.talbot_length_nm:.1f} nm, z_A = {res.achromatic_distance_nm / 1000:.1f} µm; "
      f"EUV-IL fringe period {il.fringe_period_nm:.1f} nm, visibility {il.visibility:.3f}")
```

??? success "Output"

    ```text
    z_T = 1481.5 nm, z_A = 66.7 µm; EUV-IL fringe period 50.0 nm, visibility 0.994
    ```

### CLI and TOML

[`examples/talbot.toml`](../../examples/talbot.toml) with `[deep] mode = "talbot"` and `talbot_mode = "carpet" | "dtl" | "atl" | "euv_il"` ([configuration](../configuration.md#deep)):

| Keys | Meaning |
|---|---|
| `grating_period_nm` (default `[mask] pitch_nm`), `grating_type` (`amplitude` / `phase`), `duty_cycle`, `phase_rad`, `max_order` | the grating |
| `propagation`, `gap_um`, `carpet_z_max_um`, `carpet_nz` | exact or paraxial transfer; the gap; the carpet extent |
| `bandwidth_nm` (default: the `[source]` model's `bandwidth_pm` / 1000), `spectrum_shape`, `scan_talbot_lengths` | ATL band and DTL scan |
| `il_order` | EUV-IL order |
| `n_medium`, `absorption_per_nm`, `z_span_nm`, `nz`, `dose_scale`, `dill_c`, `fill_threshold` | the shared resist-volume keys |

```bash
highuvlith deep --config examples/talbot.toml --output /tmp/talbot.json  # summary
highuvlith deep --config examples/talbot.toml --output /tmp/talbot.png   # image / carpet
```

The summary reports the Talbot lengths, achromatic distance, printed period (DFT of the centre row), image contrast and PAC fill fraction; the GUI's [volume viewer](../gui.md#views) renders the same carpet.

<details markdown="1">
<summary>Rust</summary>

```rust
use highuvlith_core::talbot::{
    achromatic_distance_nm, ExposureMode, Grating, ResistMedium, Spectrum, TalbotSetup,
    TwoGratingInterference,
};
use highuvlith_core::types::{Grid2D, Grid3D};

// 100 nm binary amplitude grating (50 % open), orders |n| <= 10, at 13.5 nm.
let setup = TalbotSetup::new(Grating::binary_amplitude(100.0, 0.5, 10)?, 13.5)?;
let z_t = setup.talbot_length_nm(); // 2p²/λ ≈ 1481 nm

// Talbot carpet over two periods and two Talbot lengths.
let carpet = setup.carpet((0.0, 200.0), 128, (0.0, 2.0 * z_t), 256, None)?;

// ATL image beyond the achromatic distance (Gaussian band, 0.3 nm FWHM).
let spectrum = Spectrum::Gaussian { fwhm_nm: 0.3, bins: 256 };
let gap = 2.0 * achromatic_distance_nm(100.0, 0.3);
let mut image = Grid2D::<f64>::new(128, 1, (0.0, 200.0), (-0.5, 0.5))?;
setup.exposure_image(&mut image, gap, &ExposureMode::Achromatic(spectrum))?; // period 50 nm

// DTL volume in a 50 nm EUV resist → PAC → development tiers.
let mut vol = Grid3D::<f64>::new(128, 1, 16, (0.0, 200.0), (-0.5, 0.5), (0.0, 50.0))?;
let resist = ResistMedium { index: 0.97, absorption_per_nm: 0.004 };
setup.resist_volume(&mut vol, 1e5, &resist, &ExposureMode::DisplacementStationary)?;

// Two-grating EUV-IL: ±1 orders of 100 nm gratings → 50 nm fringes at any λ.
let il = TwoGratingInterference::new(&setup.grating, 13.5, 1)?;
let beams = il.to_interference_setup(0.97, 0.004)?; // an InterferenceSetup
```

</details>

## Limits

- **Thin, scalar mask.** Coefficients are Kirchhoff thin-mask transmissions; absorber thickness, shadowing, and polarization effects of real EUV gratings (mask 3D) are not modeled, and the grating coefficients do not change with wavelength across an ATL band (a real phase step scales ∝ 1/λ).
- **Perfect coherence.** Normal plane-wave illumination; the angular spread of a real source blurs DTL/ATL images (a lateral smear growing with the gap) and is not included.
- **Infinite mask.** Every order is assumed to overlap everywhere; at large gaps a finite illuminated field loses the high orders near its edges.
- **Interfaces.** No Fresnel coefficients, back-reflection, or standing waves at the resist; absorption is a per-order scalar decay.
- **EUV-IL.** Ideal two-beam overlap: no zero-order or higher-order background, TE beams only (vector contrast loss for TM is only available by building the `InterferenceSetup` by hand), no beam-overlap walk-off with bandwidth.
- **Quasi-periodic revivals.** With many propagating orders the exact field is only quasi-periodic in z; the Talbot-length self-image is exact for the 0/±1 orders and approximate for the rest.

<details markdown="1">
<summary>Behaviour changes</summary>

- No behaviour change is recorded for this module in the capability matrix audit log; the closed forms on this page (DTL shell weights, ATL envelopes, $z_A$, the exact 0/±1 rephasing length) are derived from the angular-spectrum model and pinned by the tests below.

</details>

<details markdown="1">
<summary>Validation — 30 tests</summary>

`#[test]` functions in [`talbot.rs`](../../crates/highuvlith-core/src/talbot.rs) (fixture numbers computed independently with numpy / 60-digit decimal series):

| Test | What it pins | Tolerance |
|---|---|---|
| `test_sinc_and_bessel_fixtures` | `J₁(1, 5, 10, 30)` against a 60-digit power series, first zero at 3.8317 | ≤ 1e-13 |
| `test_binary_amplitude_coefficients_fixture`, `test_binary_phase_coefficients_fixture` | $c_n$ for f = 0.5, 0.3 and φ = π, π/2, π/3; π-phase 50 % kills the zeroth order, $\lvert c_{\pm1}\rvert^2 = 4/\pi^2$ | 1e-15 |
| `test_parseval_total_efficiency` | $\sum \lvert c_n\rvert^2 \to \langle\lvert t\rvert^2\rangle$ | — |
| `test_from_profile_is_exact_for_pixelated_profile`, `test_unit_cell_sampler_converges_to_analytic_holes`, `test_hole_array_coefficients_fixture` | pixelated cells exact; 8×8-supersampled 128² cell within 2e-4 of the Airy coefficients; square and hexagonal lattices (odd $n+m$ vanish) | 2e-4 |
| `test_talbot_length_fixtures_and_limits` | 1481.48 / 1474.70 nm (p = 100, λ = 13.5), 2+√3 vs 4 at λ/p = ½, ratio $1 - \epsilon^2/4$ as λ/p → 0, $p^2$ scaling, $z_A$, $p/(2m)$ | — |
| `test_paraxial_self_image_and_half_talbot_shift` | revival at $z_T$ and $p/2$ shift at $z_T/2$ | 1e-10 |
| `test_exact_first_order_pair_rephases_at_exact_length` | at λ/p = 0.6 the exact field revives at the exact $z_T$ and not at $2p^2/\lambda$ | 1e-10 |
| `test_energy_conservation_with_evanescent_orders` | period-mean intensity = $\sum_{\lvert n\rvert \le 7}\lvert c_n\rvert^2$ at 5 µm, = $\sum_{\text{all}}\lvert c_n\rvert^2$ at z = 0 | — |
| `test_clear_mask_is_unity_everywhere`, `test_carpet_rows_and_revival`, `test_dominant_period_helper` | normalization; carpet rows; the period helper | — |
| `test_dtl_stationary_equals_brute_force_gap_average` | closed-form finite scan over $z_T$, the stationary image, and a brute-force 128-gap average agree (paraxial) | 1e-12 |
| `test_dtl_image_has_half_period_and_closed_form` | period $p/2$ and the 1D closed form; gap independent | 1e-12 |
| `test_pi_phase_grating_dtl_and_zero_order`, `test_exact_finite_scan_converges_to_stationary` | π-phase DTL; residual 1.5e-2 → 9.6e-6 as the exact-propagation scan grows from 1 to 100 $z_T$ | — |
| `test_atl_flat_top_is_stationary_exactly_at_z_a` | flat-top ATL = stationary at $z = z_A$ and matches the closed form at $z_A/2$ | 1e-12 |
| `test_atl_gaussian_converges_beyond_z_a_only` | exact propagation, Gaussian 0.5 nm FWHM: within 2e-3 of the stationary image at 2 and 2.5 $z_A$ (typically ~1e-4), but > 0.05 away at 0.05 $z_A$ | 2e-3 |
| `test_atl_carpet_washes_out`, `test_hex_lattice_images_are_six_fold_symmetric` | ATL carpet; six-fold symmetry | — |
| `test_index_matched_volume_equals_free_space_propagation`, `test_dtl_volume_is_depth_invariant_up_to_absorption`, `test_volume_feeds_pac_and_development` | resist volumes and the hand-off to development | — |
| `test_euv_il_fringe_period_is_wavelength_independent`, `test_euv_il_visibility_formula`, `test_euv_il_beam_intensities_from_grating`, `test_invalid_inputs_rejected` | 50 nm at 13.5 and 6.7 nm, 25 nm for m = 2 (measured from `InterferenceSetup` intensities); V = 0.8 for 4:1 beams; $1/\pi^2$, $4/\pi^2$; guards | — |
| Python `test_talbot_lengths_and_paraxial_revival`, `test_talbot_dtl_prints_half_period`, `test_talbot_atl_converges_beyond_achromatic_distance`, `test_talbot_resist_volume_feeds_development`, `test_talbot_hex_hole_array_is_2d`, `test_euv_il_fringe_period_independent_of_wavelength`, `test_euv_il_visibility_and_beam_intensities`; CLI `test_parse_talbot_config`, `test_talbot_end_to_end_modes` | the Python and CLI surfaces | — |

</details>

## References

1. H. F. Talbot, "Facts relating to optical science. No. IV," *Philos. Mag.* **9**, 401–407 (1836) — discovery of self-imaging.
2. Lord Rayleigh, "On copying diffraction-gratings, and on some phenomena connected therewith," *Philos. Mag.* **11**, 196–205 (1881) — the self-imaging distance $2p^2/\lambda$.
3. J. W. Goodman, *Introduction to Fourier Optics*, McGraw-Hill / Roberts & Co. — angular-spectrum and Fresnel transfer functions.
4. H. H. Solak, C. Dais, F. Clube, "Displacement Talbot lithography: a new method for high-resolution patterning of large areas," *Opt. Express* **19**, 10686–10691 (2011), doi:10.1364/OE.19.010686.
5. H. H. Solak, Y. Ekinci, "Achromatic spatial frequency multiplication: a method for production of nanometer-scale periodic structures," *J. Vac. Sci. Technol. B* **23**, 2705 (2005), doi:10.1116/1.2121735.
6. H. H. Solak, "Nanolithography with coherent extreme ultraviolet light," *J. Phys. D: Appl. Phys.* (2006) — review of EUV interference lithography with transmission gratings.
7. Mojarad, Gobrecht, Ekinci, *Sci. Rep.* **5**, 9235 (2015), doi:10.1038/srep09235 — two-grating EUV-IL (half-pitch $p/4$), resist screening down to 18 nm half-pitch.
8. Isoyan et al., *J. Vac. Sci. Technol. B* **27**, 2931 (2009), doi:10.1116/1.3258144 — generalized (coherent) Talbot imaging with a table-top 46.9 nm laser.
9. Mojarad et al., *Nanoscale* **7**, 4031 (2015), doi:10.1039/c4nr07420c — 7 nm half-pitch EUV-IL.
10. Fan, Ekinci, *J. Micro/Nanolithogr. MEMS MOEMS* **15**(3), 033505 (2016), doi:10.1117/1.JMM.15.3.033505 — 6 nm half-pitch.
11. Giannopoulos et al., *Proc. SPIE* (2024), doi:10.1117/12.3010388 — 5 nm half-pitch by mirror interference lithography (reported).
12. Paul Scherrer Institut, XIL-II beamline (X09LB), https://www.psi.ch/en/sls/xil — flux and field size.

**Part of Processes · see also:** [index](index.md) · [Interference & two-photon](interference-volumetric.md) · [Volumetric exposure](volumetric-exposure.md) · [Alternatives to projection](../future/alternative-patterning.md) · [GUI volume viewer](../gui.md#views) · [configuration](../configuration.md#deep)
