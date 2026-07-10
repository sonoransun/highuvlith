# Desktop GUI Reference

**Status:** ✅ Implemented — the [egui](https://github.com/emilk/egui) app in [`crates/highuvlith-gui`](../crates/highuvlith-gui/src) runs the real core engine on a background thread. The volumetric 3D viewer is 🗺️ **planned** (see [below](#volumetric-viewer--planned)).

The GUI is a real-time aerial-image explorer: adjust source, optics, mask, and process parameters with sliders and watch the aerial image and its cross-section recompute live. It calls the same `AerialImageEngine` as the CLI and Python API, so its physics matches theirs exactly.

## Running

```bash
cargo run -p highuvlith-gui
```

Build with `--release` for smoother interaction at large grids:

```bash
cargo run -p highuvlith-gui --release
```

The window opens with an initial simulation already computed (VUV F₂ at 157.63 nm, 128×128 grid).

## Layout

- **Left panel** — collapsible parameter groups (Source, Optics, Mask, Process) plus a grid-size selector.
- **Central panel** — the visualization, with two tabs: **Aerial Image** (Inferno heatmap) and **Cross-Section** (intensity vs x line plot through the center row).
- **Bottom bar** — live status: grid size, SOCS kernel count, compute time, contrast, and the intensity range `[I_min, I_max]`. While a recompute is in flight it shows a spinner and `Computing...`.

Every parameter change marks the state dirty and kicks off a recompute on a **background thread**, so the UI never blocks. To keep interaction fluid the GUI builds the engine with a single spectral sample and up to 15 SOCS kernels; the CLI and Python paths use more samples for higher fidelity.

## Source

A dropdown selects among **six source families** (the CLI/Python-only families — ICS, SSMB, entangled — are deliberately excluded to keep the panel usable):

| Dropdown label | Family | Default λ on select |
|----------------|--------|---------------------|
| VUV excimer | `VuvSource` | 157.63 nm |
| LPA-FEL (EUV) | `LpaFelSource` | 25.0 nm |
| LPP plasma | `LppSource` | 13.5 nm |
| Synchrotron undulator | `SynchrotronSource` | 13.5 nm (derived) |
| HHG (table-top) | `HhgSource` | 13.56 nm (derived) |
| XFEL | `XfelSource` | 13.5 nm |

Selecting a family resets the wavelength to its default and σ to 0.7.

### Preset buttons

A row of one-click presets: **F2 (157)**, **Ar2 (126)**, **FEL 25**, **EUV 13.5** (Sn LPP), and **BEUV 6.7** (Gd LPP). These set both the family and the wavelength together.

### Wavelength control — set vs. derived (live physics)

The wavelength widget depends on whether the selected family sets its wavelength directly or derives it from machine parameters:

- **Set-point families** (VUV, LPA-FEL, LPP, XFEL) show a **logarithmic slider from 1 to 170 nm** (0.01 nm steps).
- **Derived families** (Synchrotron undulator, HHG) show a **read-only readout** instead:
  `λ = 13.482 nm (derived from machine parameters)`.
  This value updates live from the physics — the synchrotron readout is the undulator resonance `λ_u(1 + K²/2)/(2γ²)` computed from the electron energy, period, and K sliders; the HHG readout is `800 / harmonic`. Moving those sliders moves the wavelength, which is the point: it is proof the machine model is live, not a cosmetic label.

### Per-family controls

Beyond the σ slider (0.1–1.0), each family exposes its own physically meaningful knobs:

| Family | Controls |
|--------|----------|
| VUV | *(none beyond λ and σ)* |
| LPA-FEL | E_e (MeV) 100–600; τ (fs) 5–50 |
| LPP | Fuel toggle: Sn (13.5 nm) / Gd (6.7 nm) — sets λ |
| Synchrotron | E_ring (GeV) 0.2–3.0; λ_u (mm) 5–50; K 0.3–3.0 — all feed the derived λ |
| HHG | Harmonic q 11–71 (kept **odd**); fixed Ne gas, 800 nm driver, 4×10¹⁴ W/cm² |
| XFEL | Mode toggle: SASE / Seeded |

## Optics

A single **NA slider (0.3–0.95)**. The optical system itself is chosen automatically to stay physical:

> When the effective wavelength drops **below 50 nm**, the Source panel shows
> `Optics: Schwarzschild reflective (auto — no lens below 50 nm)`
> and the engine switches from the refractive CaF₂ lens to a Schwarzschild reflective objective (BEUV preset below 10 nm, EUV standard otherwise), with NA capped at 0.6.

This mirrors the CLI's [optics honesty guard](./configuration.md#optics) — but where the CLI *warns* and lets you proceed with unphysical refractive optics, the GUI silently does the right thing and tells you it did. Above 50 nm it uses the refractive projection lens at the slider's NA.

## Mask

- **CD (nm)** slider, 20–200.
- **Pitch (nm)** slider, 40–500.

The GUI always simulates a line/space pattern.

## Process

- **Focus (nm)** slider, −500 to +500.

Dose is not a control here — the GUI shows the aerial image, which is dose-independent.

## Grid

A row of selectable sizes: **64, 128, 256, 512, 1024**. Larger grids give finer sampling at higher compute cost; the pixel pitch is fixed at 2 nm, so the field of view scales with the grid size. Default is 128.

## Errors

If the engine or a source/optics/mask constructor rejects the current parameters (e.g. an HHG harmonic beyond the cutoff), the central panel shows the error message in red instead of an image, and clears once the parameters become valid again.

## Volumetric viewer 🗺️ Planned

The GUI currently visualizes only the 2D aerial image and its cross-section. A viewer for the z-resolved deep-layer results (volumetric PAC/dose fields, LIGA depth dose, grayscale height maps) is planned but not yet implemented; those pipelines are available through the [Python API](./python-api.md) today.
