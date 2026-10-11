---
description: "The egui desktop app: real captures beside the views they show, the source panel with its fidelity badges, optics and imaging settings, and what the GUI does not show."
---

# GUI

**Status:** ✅ Implemented — the [egui](https://github.com/emilk/egui) app in [`crates/highuvlith-gui`](../crates/highuvlith-gui/src) drives the same core models as the CLI and Python API on background threads and adds no physics of its own; every number it shows carries the grade of the core model behind it · [matrix](capability-matrix.md#frontends-and-infrastructure)

<p class="huv-lead">An interactive front end for the whole pipeline: pick any of the fourteen light-source families or their presets, watch the physics derived from the machine parameters, choose optics and imaging settings, and look at the aerial image, its cross-section, the dose-aware process window, 3-D resist / LIGA / Talbot volumes and the LIGA depth dose. It is for exploring; batch runs belong to the CLI and arrays to Python.</p>

**Same thing elsewhere:** Python [`simulate_line_space`](python-api.md#quick-start) · CLI [`simulate`](cli.md#simulate) · TOML [`examples/sim.toml`](../examples/sim.toml) · physics [Simulation pipeline](pipeline.md)

<!-- gui-layout-annotated -> gui.md -->
<figure markdown="span">

![The running desktop app, dark theme, with three numbered overlays on the Aerial image view: 1 the Source, Optics and Imaging controls, 2 the tab bar and the EUV aerial image with its metrics, 3 the status bar reporting 21 kernels, the captured TCC fraction and the contrast 0.4494.](assets/images/gui/gui-layout-annotated-light.png#gh-light-mode-only)
![The running desktop app, dark theme, with three numbered overlays on the Aerial image view: 1 the Source, Optics and Imaging controls, 2 the tab bar and the EUV aerial image with its metrics, 3 the status bar reporting 21 kernels, the captured TCC fraction and the contrast 0.4494.](assets/images/gui/gui-layout-annotated-dark.png#gh-dark-mode-only)

<figcaption><strong>The running app.</strong> Real <code>--screenshot</code> capture, dark theme; nothing is mocked: 1 controls, 2 views, 3 status bar showing kernels, captured TCC fraction and contrast. The app is dark-themed; the light variant is the same capture on a light frame. <em>Annotated real capture; nothing is mocked.</em></figcaption>
</figure>

Every image on this page is a real capture made by the app's own `--screenshot` mode (egui reads the rendered frame back from the GPU); the app is dark-themed, so the captures are dark on both colour schemes.

## Find by task

| I want to … | Where |
|---|---|
| start the app with a preset, optics and a mask | [Running](#running) |
| know what each region of the window does | [Layout](#layout) |
| pick a source family and read its fidelity badge | [Source panel](#source-panel) |
| choose a lens, mirror optics or the multilayer pupil | [Optics](#optics) |
| switch defocus, vector or polychromatic imaging | [Imaging settings](#imaging-settings) |
| set the mask, grid and threshold | [Mask, grid, focus](#mask-grid-focus) |
| read a view (aerial, cross-section, process window, volumes, LIGA, source) | [Views](#views) |
| save an image | [Export](#export) |
| check what is not in the GUI | [What the GUI does not show](#what-the-gui-does-not-show) |

## At a glance

<!-- huv-table: stack -->

| View (tab) | Shows | Core model | Grade | Captured |
|---|---|---|---|---|
| Aerial image | heatmap, contrast, printed CD, NILS, k₁ | Hopkins/SOCS imaging | ✅ Implemented | [yes](#views) |
| Cross-section | the y = 0 intensity row with the threshold and printed edges | the same image | ✅ Implemented | no (uncaptured) |
| Process window | Bossung curves, ED window, EL vs DOF | dose-aware `ProcessWindow::compute` | 🔶 Simplified constant-threshold resist | [yes](#views) |
| Volume viewer | latent, developed, LIGA and Talbot volumes | volumetric chain, LIGA, Talbot | 🔶 Simplified chain; LIGA depth dose ✅ Implemented; Talbot 🔶 Simplified | [yes](#views) (Talbot dataset) |
| LIGA depth dose | absorbed dose vs depth, exposure time | LIGA depth dose | ✅ Implemented | no (uncaptured) |
| Source | badge, derived quantities, spectrum, pupil fill | the source factories | grade per family | [yes](#views) |

## Running

```bash
cargo run -p highuvlith-gui --release
```

A debug build works but is noticeably slower on the process-window and volume views. Command-line options (all optional):

| Option | Meaning |
|---|---|
| `--preset NAME` | source preset, e.g. `arf`, `f2`, `lpp-sn`, `lpp-sn-500w`, `xfel-flash`, `hhg-ne`, `tube-w`, `betatron` (the full list is the `cli_name` table in [`sources.rs`](../crates/highuvlith-gui/src/sources.rs)) |
| `--optics NAME` | optics preset: `193i` (NA 1.35 water immersion), `nxe` (EUV NA 0.33), `high-na` (EUV NA 0.55) |
| `--tab NAME` | `aerial`, `cross-section`, `process-window`, `volume`, `liga`, `source` |
| `--dataset NAME` | volume dataset: `latent`, `developed`, `liga`, `talbot` |
| `--cd NM`, `--pitch NM` | mask line CD and pitch |
| `--size WxH` | window size in points (default 1500x950) |
| `--screenshot FILE.png` | save a PNG of the window once the view has been computed, then exit |

The canonical run of the [CLI](cli.md#simulate) and the [Python quick start](python-api.md#quick-start) is `--preset f2 --cd 65 --pitch 180` with the default refractive optics at NA 0.75.

<details markdown="1">
<summary>Screenshots — the four commands that made the captures on this page</summary>

Made on macOS at 2× scale and downscaled to 1500 px wide; the hashes of the four files are pinned by `docs/figures/gui/make_gui_media.py --check`.

```bash
B=target/release/highuvlith-gui
$B --preset lpp-sn --optics nxe --cd 16 --pitch 32 --tab aerial --screenshot gui-aerial-euv.png
$B --preset arf --optics 193i --cd 60 --pitch 130 --tab process-window --screenshot gui-process-window.png
$B --preset arf --tab volume --dataset talbot --screenshot gui-volume-talbot.png
$B --preset ssmb --optics nxe --tab source --screenshot gui-source-ssmb.png
```

</details>

## Layout

- **Left panel** (1) — collapsible sections: Source, Optics, Imaging settings, Mask, Grid, Focus & threshold (and LIGA exposure when an X-ray source is selected).
- **Central panel** (2) — six tabs: Aerial image, Cross-section, Process window, Volume viewer, LIGA depth dose, Source. Tabs that make no sense for the selected source are disabled (projection views for LIGA sources and vice versa).
- **Bottom bar** (3) — wavelength, NA, grid, number of SOCS kernels and the fraction of the TCC they capture, engine/image times and the contrast; a spinner while a computation runs.

All physics runs on worker threads with latest-wins coalescing: dragging a slider never queues a backlog and the UI never blocks. Engines are cached by everything that defines them (source, optics, imaging settings, grid), so moving focus, CD or threshold reuses the engine. Errors from the core (an HHG harmonic beyond the cutoff, an unreachable FEL set-point, NA above the immersion limit, …) are shown in place of the result and clear when the inputs become valid.

## Source panel

A dropdown selects the family; buttons below it load the family's presets. Every preset is built from the corresponding core factory, so its numbers are the core's, and every family shows the grade of its [matrix row](capability-matrix.md#light-sources) in the app itself:

<!-- gui-source-badge -> gui.md -->
<figure markdown="span">

![The Source tab header for the SSMB storage ring: the TOML type tag, the Theoretical badge and its one-line justification (the 1 kW point needs a bunching factor 0.146 never demonstrated), and the set-point line 13.500 nm, 1000.0 W average power.](assets/images/gui/gui-source-badge-light.png#gh-light-mode-only)
![The Source tab header for the SSMB storage ring: the TOML type tag, the Theoretical badge and its one-line justification (the 1 kW point needs a bunching factor 0.146 never demonstrated), and the set-point line 13.500 nm, 1000.0 W average power.](assets/images/gui/gui-source-badge-dark.png#gh-dark-mode-only)

<figcaption><strong>The fidelity badge.</strong> Every source family carries its matrix grade and the one-sentence reason in the app itself — here SSMB's 🧪 Theoretical badge, 1:1 from the Source tab. <em>Real capture, 1:1 crop; nothing is mocked.</em></figcaption>
</figure>

<!-- huv-table: stack -->

| Family (type tag) | Presets | Wavelength | Route |
|---|---|---|---|
| [VUV / DUV / UV](sources/vuv-excimer.md) (`vuv`) | F₂ 157.63, ArF 193.368, KrF 248.3, Hg i/h/g lines ([heritage](sources/duv-heritage.md)), Ar₂ 126 (🧪 hypothetical) | fixed line | projection |
| [LPA-FEL](sources/lpa-fel.md) (`lpa_fel`) | 25 nm / 500 MeV (projection), 420 nm / 100 MeV (BELLA, demonstrated) | **derived**: undulator resonance | projection |
| [LPP](sources/lpp.md) (`lpp`) | Sn CO₂ (NXE:3400B, 250 W at IF), Sn 1 µm, Sn 2 µm (projection), Sn 500 W at IF (NXE:3800E class; drive power assumed), Gd 6.7, Tb 6.5 (power projections) | fixed line | projection |
| [Synchrotron](sources/synchrotron.md) (`synchrotron`) | compact EUV undulator, LIGA bending magnet | **derived**: undulator resonance (bending magnet: white beam) | projection / LIGA |
| [HHG](sources/hhg.md) (`hhg`) | Ne q = 59 (13.56 nm), Ar q = 27 (29.6 nm) | **derived**: driver / q | projection |
| [XFEL / FEL](sources/xfel.md) (`xfel`) | FLASH-like SASE, FERMI-like seeded, CW-SC (🧪), ERL (🧪) | set-point (K gap-tuned) | projection |
| [Inverse Compton](sources/inverse-compton.md) (`ics`) | compact EUV design point | **derived**: Compton upshift | projection |
| [SSMB](sources/ssmb.md) (`ssmb`) | 1 kW EUV design point | **derived**: modulation λ / harmonic | projection |
| [Entangled photons](sources/entangled-photon.md) (`entangled`) | NOON N = 2 at 157.63 nm | set-point | projection (classical imaging) |
| [Hard X-ray tube](sources/xray-tube.md) (`xray_tube`) | W 60 kV, Mo 50 kV, Cu 40 kV, Rh 50 kV | **derived**: mean photon energy of the filtered spectrum | LIGA |
| [DPP / LDP](sources/dpp.md) (`dpp`) | Sn LDP (360 W into 2π, ≈ 34 W at IF), Xe DPP (metrology class) | fixed line | projection |
| [Soft-X-ray laser](sources/soft-xray-laser.md) (`sxrl`) | Ni-like Ag 13.9, Cd 13.2, Mo 18.9, Ne-like Ar 46.9 nm | fixed atomic line | projection |
| [Betatron](sources/betatron.md) (`betatron`) | 100 TW-class LWFA | **derived**: hc / mean photon energy | LIGA |
| [Smith–Purcell](sources/smith-purcell.md) (`smith_purcell`) | 30 keV, first order at 90° (13.5 nm) | **derived**: dispersion relation | projection |

For each family the panel shows:

- the **status badge** and a one-line justification mirroring the family's docs page, plus a preset-specific badge and provenance note where the preset differs (e.g. the hypothetical Ar₂ preset, the LPP Gd/Tb power projections, the XFEL CW-SC and ERL presets);
- the **wavelength** with how it arises — "fixed by the emitting line", a set-point, or "DERIVED from the machine" with the formula; for derived families the wavelength moves when the machine sliders move;
- the family's **main machine parameters** as sliders (e.g. LPP drive power, CE, 2π-to-IF efficiency and rep rate; undulator energy, period and K; HHG gas, intensity, harmonic with the live cutoff limit, and either the driver average power — the harmonic power is then derived as P_driver × η_q — or a stored pulse energy; X-ray tube anode, kV, mA and Be window);
- a collapsible **Derived quantities** table: every entry of the core's `derived_quantities()` with value, unit and note (gain lengths, flux, photon budgets, power ratios to a 250 W HVM source, exposure-time bounds, …);
- a collapsible **Pupil fill** section: the family default (a conventional disk of σ for incoherent sources, a coherence-derived Gaussian for beams) or an explicit conventional, annular, dipole (two poles) or quadrupole fill.

<!-- gui-source-derived -> gui.md -->
<figure markdown="span">

![The derived-quantities table of the Source tab for SSMB: photon energy 91.840 eV, harmonic number 78, electron gamma 783.8, bunching factor 0.1458, coherent power 1000.0 W, incoherent power 1.674 W, projected power 1000.0 W and photon rate 6.796e19 photons/s, each with its formula.](assets/images/gui/gui-source-derived-light.png#gh-light-mode-only)
![The derived-quantities table of the Source tab for SSMB: photon energy 91.840 eV, harmonic number 78, electron gamma 783.8, bunching factor 0.1458, coherent power 1000.0 W, incoherent power 1.674 W, projected power 1000.0 W and photon rate 6.796e19 photons/s, each with its formula.](assets/images/gui/gui-source-derived-dark.png#gh-dark-mode-only)

<figcaption><strong>Derived quantities, with their formulas.</strong> The informational block under every source: each number names the formula it comes from and the imaging pipeline never reads it — 1:1 from the SSMB Source tab. <em>Real capture, 1:1 crop; nothing is mocked.</em></figcaption>
</figure>

**X-ray tube, betatron and bending-magnet sources** are broadband and incoherent: the panel says so and routes them to the LIGA view (deep X-ray proximity printing) instead of projection imaging.

The **Source** tab repeats the badges and derived quantities at full width and plots the spectral samples and the sampled pupil fill (for X-ray sources: the photon spectrum) — see [Views](#views).

## Optics

- **Kind**: Auto (refractive above 110 nm, reflective EUV projection optics below — no transparent lens material exists below ~110 nm), dry refractive lens, refractive lens + water immersion (NA up to 0.95·n; n = 1.437 at 193 nm), EUV projection optics, Schwarzschild objective, Fresnel zone plate. Forcing a refractive lens below 110 nm is allowed but flagged as unphysical ([Optical systems](optics.md)).
- **Presets**: 193i (NA 1.35, water), EUV NXE (NA 0.33, unobscured), High-NA (NA 0.55 with a 0.2·NA central obscuration — an assumed value).
- **NA**, **central obscuration** (reflective optics), **flare fraction**.
- **Angle-dependent multilayer pupil** (EUV projection / Schwarzschild, opt-in, 🔶 Simplified): Mo/Si, La/B₄C or La/B coating, period, number of coated mirrors and an incidence-angle map (angle at the pupil centre, radial change at the rim, linear tilt and its azimuth) applied to every mirror. The panel shows the single-mirror reflectance at the source wavelength so a mistuned coating is visible. The angle map is a user assumption, not a ray trace of a real design; with the clear-field normalization it changes the image shape (apodization and phase), not the dose ([Materials](materials.md)).

**The EUV pupil is isotropic on the wafer side**: anamorphic 4×/8× magnification and mask-3D effects are not modelled.

## Imaging settings

- **Defocus model**: Exact (defocus inside the pupil at every source point, the default) or the legacy kernel-phase approximation, kept for comparison.
- **Imaging model**: scalar, or vector with illumination polarization X, Y, TE (azimuthal), TM (radial) or unpolarized ([Vector imaging](vector-imaging.md)).
- **Intensity scale**: relative (clear field = 1, the default) or absolute.
- **Spectrum**: monochromatic (centre wavelength), narrow-band (centre-wavelength kernels with a chromatic focus shift per spectral sample; valid for Δλ/λ ≪ 1) or per-wavelength (a TCC rebuilt at every spectral sample — the honest mode for an HHG comb or other broad spectra); the number of spectral samples can be overridden.
- **Max SOCS kernels**.

## Mask, grid, focus

- **Mask**: lines/spaces or a contact array; pitch and CD (or hole size); reverse field tone; 6 % / 180° attenuated PSM. Thin (Kirchhoff) mask with the exact Fourier-series spectrum; no mask-3D ([Masks](masks.md)).
- **Grid**: grid size (power of two) with an automatic commensurate grid — the field holds a whole number of pitches and the pixel is small enough to represent the full band (1+σ)·NA/λ that reaches the pupil. The chosen field, pixel and Nyquist-safe pixel are shown; a manual target pixel is available.
- **Focus & threshold**: focus slider scaled to the Rayleigh DOF nλ/(2NA²) and the constant-threshold print level used for printed CD and NILS.

## Views

**Aerial image** — heatmap (Inferno, Blues or grayscale colormap) with colour bar and value readout on hover; contrast, I_min/I_max, printed CD and NILS at the threshold, and k₁ with the pitch cutoff λ/(NA(1+σ)) ([Simulation pipeline](pipeline.md)).

<!-- gui-aerial-euv -> gui.md -->
<figure markdown="span">

![The running desktop app on its Aerial image tab: the Source and Optics controls (Sn LPP preset, NA 0.33), the EUV 13.5 nm aerial image of 16 nm lines on a 32 nm pitch with its colour scale, the metrics rows (contrast 0.4494, printed CD 16.637 nm, NILS 1.427) and the status bar naming 21 kernels.](assets/images/gui/gui-aerial-euv.png)

<figcaption><strong>Aerial image view.</strong> The Sn LPP preset (NXE:3400B class) through the EUV NXE optics, NA 0.33, 16 nm lines on a 32 nm pitch: the image, its colour scale, the metrics rows and the status bar with the kernel count and captured TCC fraction. Made with <code>highuvlith-gui --preset lpp-sn --optics nxe --cd 16 --pitch 32 --tab aerial --screenshot</code>. <em>Real capture, dark theme; nothing is mocked.</em></figcaption>
</figure>

**Cross-section** (uncaptured) — intensity along the y = 0 row with the threshold line and marked printed edges; a data table. No capture of this view exists on this page; it shows the row the aerial view's printed CD is measured on.

**Process window** — a dose × focus sweep through the core's dose-aware `ProcessWindow::compute` with a constant-threshold resist (🔶 Simplified): Bossung curves (printed CD vs focus per dose) with the CD spec band, the exposure–defocus window (in-spec region, largest DOF×EL rectangle, DOF at 5 % EL), and EL vs DOF. Best focus, dose-to-size, DOF and EL are tabulated ([Metrics and process window](metrics-and-process-window.md)).

<!-- gui-process-window -> gui.md -->
<figure markdown="span">

![The Process window tab for ArF 193 nm in water (NA 1.35), 60 nm lines on a 130 nm pitch: the sweep controls, summary rows (DOF at dose-to-size 167.2 nm, exposure latitude 19.645 %), Bossung curves per dose, the exposure–defocus window with its max DOF × EL rectangle and the EL-versus-DOF curve.](assets/images/gui/gui-process-window.png)

<figcaption><strong>Process window view.</strong> ArF 193 nm with water immersion, NA 1.35, 60 nm lines on a 130 nm pitch: Bossung curves per dose, the exposure–defocus window with its largest DOF × EL rectangle and the exposure-latitude curve, all from the dose-aware sweep in the panel above them. Made with <code>highuvlith-gui --preset arf --optics 193i --cd 60 --pitch 130 --tab process-window --screenshot</code>. <em>Real capture, dark theme; nothing is mocked.</em></figcaption>
</figure>

**Volume viewer** — z-slice scrubber with an x–y slice and an x–z cross-section at a chosen y, colormap with global or per-view range, colour bar and value readout on hover, PNG export of either view. Datasets:

- resist latent image (PAC) of the volumetric exposure through focus, after an optional Gaussian or CAR reaction–diffusion bake ([Volumetric exposure](processes/volumetric-exposure.md));
- developed resist from fast-marching or level-set development (1 = dissolved). The GUI's defaults for this dataset (resist thickness, dose, bake, development time) are stated once, beside the CLI's and Python's, in [Reference → Defaults by front end](reference/index.md#defaults-by-front-end); the bake is on by default because on bare Si the ≈ 48 nm standing waves otherwise stall the dissolution front at the first node, and the core's default resist is illustrative and low contrast, so the window is narrow;
- LIGA absorbed dose (kJ/cm³) — X-ray sources only ([LIGA](processes/liga-deep-xray.md));
- Talbot carpet I(x, z) behind an amplitude or π-phase grating, an x–z plane ([Talbot](processes/talbot.md)).

<!-- gui-volume-talbot -> gui.md -->
<figure markdown="span">

![The Volume viewer tab on the Talbot carpet dataset for ArF 193 nm: dataset controls (amplitude grating, ±10 orders, 2 Talbot lengths, 2 periods), the intensity profile at the z slice 1.309 nm behind the grating and the x–z section at y = 0 with its colour scale.](assets/images/gui/gui-volume-talbot.png)

<figcaption><strong>Volume viewer, Talbot carpet.</strong> ArF 193 nm, a 180 nm amplitude grating with ±10 orders over two Talbot lengths: the profile at the chosen z slice and the x–z section at y = 0, the same carpet the <code>deep --mode talbot</code> command computes. Made with <code>highuvlith-gui --preset arf --tab volume --dataset talbot --screenshot</code>. <em>Real capture, dark theme; nothing is mocked.</em></figcaption>
</figure>

**LIGA depth dose** (uncaptured) — for X-ray tube, betatron and bending-magnet sources: absorbed dose vs depth in PMMA with the bottom (clearing) dose and damage ceiling, exposure time from the absolute flux when enabled (tube and betatron as point sources at a given distance, bending magnet from ring current, acceptance and scan), Be/Al/Kapton filters, Au absorber and Ti membrane, Fresnel or legacy Gaussian proximity model, and a warning when the grid under-resolves the Fresnel scale √(λd). "Open the 3D dose in the volume viewer" hands the dose volume to the volume viewer. No capture of this view exists on this page; the same curves, computed by the same core function, are on the [LIGA page](processes/liga-deep-xray.md).

**Source** — the badges and derived quantities at full width, the spectral samples and the sampled pupil fill (for X-ray sources: the photon spectrum).

<!-- gui-source-ssmb -> gui.md -->
<figure markdown="span">

![The Source tab for the SSMB storage ring 1 kW design point: the Theoretical fidelity badge with its justification, the derived-quantities table (harmonic number 78, bunching factor 0.1458, coherent power 1000 W, photon rate 6.796e19 photons/s), the spectral samples and the sampled pupil fill.](assets/images/gui/gui-source-ssmb.png)

<figcaption><strong>Source view.</strong> The SSMB storage ring at its 1 kW design point through the EUV NXE optics: the fidelity badge and its justification, the derived quantities with their formulas, the spectral samples and the sampled pupil fill. Made with <code>highuvlith-gui --preset ssmb --optics nxe --tab source --screenshot</code>. <em>Real capture, dark theme; nothing is mocked.</em></figcaption>
</figure>

## Export

- **Save PNG** (aerial image and each volume view) writes the image at its native resolution with the colours shown.
- **Screenshot** (top right) saves the whole window; `--screenshot FILE.png` does the same from the command line and exits.

Both are written to the working directory as compressed RGBA PNGs (adaptive row filters plus DEFLATE via the `png` crate). A full-window capture at 2× scale (3000 × 1670 px, 20 MB of raw pixels) is about 0.4 MB.

## What the GUI does not show

- Grayscale lithography, interference lithography, ILT/OPC/SRAF, double patterning, DSA, ptychography, MNSL, stochastic/shot-noise analysis and the quantum N-photon imaging module are not in the GUI; use the [Python API](./python-api.md) (and, where a mode exists, the [CLI](./cli.md)).
- Thin-film stack reflectivity (swing curves) and resist chemistry beyond the volumetric exposure / bake / development chain are not exposed as separate views.
- Entangled-photon sources are imaged classically, like the rest of the pipeline; the λ/N figure appears only as a derived quantity.
- Two views have no capture on this page: Cross-section and LIGA depth dose (named as uncaptured above).

**Where next:** [Python API](python-api.md) · [CLI](cli.md) · [Sources](sources/index.md) · [Reference](reference/index.md)
