# CLI Reference

**Status:** ✅ Implemented — `simulate`, `sweep`, `deep`, and `materials` are live in [`crates/highuvlith-cli`](../crates/highuvlith-cli/src). Individual deep-layer modes span the honesty spectrum (see the [capability matrix](./capability-matrix.md)).

The `highuvlith` CLI is a [clap](https://docs.rs/clap)-based front end over the core engine. It reads [TOML configs](./configuration.md), runs a simulation, and writes results to a file (JSON or PNG) while printing a human-readable summary to **stderr**.

## Running

With the crate built (`cargo build -p highuvlith-cli --release`), invoke the `highuvlith` binary directly, or run through Cargo during development:

```bash
# Installed / built binary
highuvlith simulate --config examples/sim.toml

# Via cargo (note the `--` separating cargo args from CLI args)
cargo run -p highuvlith-cli -- simulate --config examples/sim.toml
```

`highuvlith --help` and `highuvlith <subcommand> --help` print usage. `highuvlith --version` prints the crate version.

## Output conventions

Two of the three subcommands follow the same I/O split:

- **stderr** carries the run summary (source, optics, mask, grid, timing, metrics, progress bars). It is always printed, even without `--output`.
- **the output file** (`--output`) carries the machine-readable result. `simulate` writes **JSON**, or a **PNG** when the output path ends in `.png`; `sweep` writes JSON only.

`materials` is the exception — it prints its table or record directly to **stdout** and writes no file.

This split means you can pipe a simulation summary to a log while redirecting nothing, or capture structured data with `--output` and still watch progress on the console.

---

## `simulate`

Compute the aerial image for a single dose/focus condition.

```
highuvlith simulate --config <FILE> [--output <FILE>] [--focus <NM>] [--dose <MJ_CM2>]
```

| Flag | Short | Type | Default | |
|------|-------|------|---------|--|
| `--config` | `-c` | path | *(required)* | TOML config file. |
| `--output` | `-o` | path | *(none)* | Result file. `.png` → image; any other extension → JSON. Omitted → summary only. |
| `--focus` | | float (nm) | `[process].focus_nm` | Override the config's focus for this run. |
| `--dose` | | float (mJ/cm²) | — | Accepted but **not consumed** by `simulate` (see note). |

**Dose note:** `simulate` computes only the aerial image, which is dose-independent, so `--dose` is currently ignored by this subcommand. Dose enters the pipeline at the resist stage; use `sweep` (which records dose in its output) or the [Python API](./python-api.md) `compute_resist_profile` / `measure_cd` for dose-dependent results.

### Output

**PNG** (`--output aerial.png`): the aerial intensity rendered with the Inferno colormap.

**JSON** (any other `--output` extension):

```json
{
  "wavelength_nm": 157.63,
  "source_type": "vuv",
  "na": 0.75,
  "cd_nm": 65.0,
  "pitch_nm": 180.0,
  "focus_nm": 0.0,
  "grid_size": 256,
  "pixel_nm": 1.0,
  "contrast": 0.87,
  "i_max": 1.02,
  "i_min": 0.07,
  "compute_ms": 112.4,
  "num_kernels": 12
}
```

### stderr summary

```
Source:  λ = 157.63 nm, σ = 0.70 (vuv)
Optics:  NA = 0.75
Mask:    CD = 65.0 nm, pitch = 180.0 nm
Grid:    256×256, pixel = 1.0 nm
Focus:   0.0 nm
Engine:  12 SOCS kernels
Compute: 112.40ms
Results:
  Contrast:  0.8712
  I_max:     1.0203
  I_min:     0.0701
Output written to aerial.png
```

The `Source` line reports the **resolved** wavelength and family label, so a synchrotron or HHG config shows its derived wavelength here.

### Examples

```bash
# F2 excimer VUV, PNG out (back-compat config with no `type` tag)
highuvlith simulate --config examples/sim.toml --output aerial.png

# LPA-FEL 25 nm EUV, JSON out
highuvlith simulate --config examples/sim_lpa_fel.toml --output fel.json

# Override focus, summary to stderr only
highuvlith simulate --config examples/sim.toml --focus 120
```

---

## `sweep`

Sweep dose and/or focus to build a process window.

```
highuvlith sweep --config <FILE> [--output <FILE>] [--focus-range <R>] [--dose-range <R>]
```

| Flag | Short | Type | Default | |
|------|-------|------|---------|--|
| `--config` | `-c` | path | *(required)* | TOML config file. |
| `--output` | `-o` | path | *(none)* | JSON result file. Omitted → summary only. |
| `--focus-range` | | `start,stop,steps` | `-200,200,11` | Focus values (nm). |
| `--dose-range` | | `start,stop,steps` | *(none)* | Dose values (mJ/cm²). Omitted → single dose from `[process].dose_mj_cm2`. |

### Range syntax

Ranges are `start,stop,steps` (three comma-separated numbers), inclusive of both endpoints:

- `-200,200,11` → 11 focus values from −200 to +200 nm.
- `steps < 2` collapses to the single value `[start]`.
- `start > stop` with `steps > 1` is an **error** (`Invalid range: start (…) > stop (…)`).
- Any format other than exactly three fields is an error.

The total point count is `focuses × doses`; each point computes an aerial image, its contrast, and its CD (via `measure_cd_2d` at threshold 0.3).

### Output

JSON only:

```json
{
  "config": { "wavelength_nm": 157.63, "source_type": "vuv", "na": 0.75, "cd_nm": 65.0, "pitch_nm": 180.0 },
  "focuses": [-200.0, -160.0, ...],
  "doses": [30.0],
  "results": [
    { "dose_mj_cm2": 30.0, "focus_nm": -200.0, "contrast": 0.42, "cd_nm": 71.3 },
    ...
  ]
}
```

### stderr summary

A live [indicatif](https://docs.rs/indicatif) progress bar plus:

```
Sweep: 21 focuses × 1 doses = 21 points
Engine: 12 SOCS kernels
 [00:00:02] [########################################] 21/21 (00:00:00)
Sweep complete
Total time: 2.31s (110.0ms per point)
Output written to pw.json
```

### Examples

```bash
# Focus-only process window
highuvlith sweep --config examples/sim.toml --focus-range="-300,300,21" --output pw.json

# Full dose × focus matrix
highuvlith sweep --config examples/sim.toml \
    --focus-range="-200,200,11" --dose-range="20,50,7" --output matrix.json
```

Note the `--focus-range="..."` form: quote and use `=` so the leading `-` in a range like `-300,...` is not parsed as another flag.

The JSON is ready to feed the [Python](./python-api.md) Bossung/ED-window plots (`viz.plot_bossung`, `viz.plot_ed_window`).

---

## `materials`

Query the built-in optical-constants database. Unlike the other subcommands, output goes to **stdout** and no file is written.

```
highuvlith materials [--wavelength <NM>] [--name <MATERIAL>]
```

| Flag | Type | Default | |
|------|------|---------|--|
| `--wavelength` | float (nm) | `157.0` | Evaluation wavelength. |
| `--name` | string | *(none)* | Single material to query. Omitted → full table. |

### Without `--name` — full table

Prints `n` and `k` for every catalog material at the wavelength:

```
Material            n        k  (at 157.0 nm)
----------------------------------------
CaF2          1.5587   0.0000
MgF2          1.4530   0.0000
...
```

Catalog: `CaF2`, `MgF2`, `LiF`, `BaF2`, `SiO2`, `Cr`, `Si`, `AlF3`, `Na3AlF6`, `LaF3`, `GdF3`, `VUV_resist`, `VUV_BARC`. Materials with no data at the wavelength show `N/A`.

### With `--name` — single record

Adds the dispersion `dn/dλ` when available:

```
CaF2 at 157.0 nm:
  n = 1.5587
  k = 0.0000
  dn/dλ = -0.001234 /nm
```

### Examples

```bash
highuvlith materials --wavelength 157          # full table at 157 nm
highuvlith materials --name CaF2 --wavelength 126   # single material + dispersion
```

---

## `deep`

Run one of the four deep-layer (3D / high-aspect-ratio) process modes: LIGA deep-X-ray shadow printing, grayscale 2.5D topography, multi-beam interference, or volumetric z-resolved exposure.

```
highuvlith deep --config <FILE> [--output <FILE>]
```

| Flag | Short | Type | Default | |
|------|-------|------|---------|--|
| `--config` | `-c` | path | *(required)* | TOML config with `[source]`/`[optics]`/`[mask]`/`[grid]` **and** a `[deep]` table. |
| `--output` | `-o` | path | *(none)* | `.png` → the mode's characteristic image; any other extension → JSON summary. Omitted → summary only. |

The mode is **not** a flag — it is `[deep] mode = "…"` in the config (`liga`, `grayscale`, `interference`, or `volumetric`), and it is required. The command reuses the standard sections through `SimConfig` (so `validate()` still runs) and parses the extra `[deep]` table for mode-specific parameters. See [Configuration → `[deep]`](./configuration.md#deep) for every key.

Each mode prints a detailed summary to stderr (led by `Deep mode: <Mode>`) and writes a mode-specific JSON summary or PNG:

| Mode | Status | stderr reports | PNG image (colormap) | JSON `mode` |
|------|--------|----------------|----------------------|-------------|
| `liga` | 🔶 | Critical energy, resist thickness, proximity gap, top/bottom dose, dose ratio, damage-ceiling margin, max aspect ratio, developed depth range | Developed-depth map (Viridis) | `"liga"` |
| `grayscale` | 🔶 | Contrast slope γ, D_th/D_clear, film thickness, exposure dose, target vs achieved height range, RMS error | Achieved height map (Viridis) | `"grayscale"` |
| `interference` | ✅/🔶 | Wavelength, air half-angle, fringe period λ/(2sinθ), kinetics, PAC range, iso-surface fill fraction | Mid-z PAC slice (Inferno) | `"interference"` |
| `volumetric` | 🔶 | Wavelength, resist thickness, dose, split-step config, top/mid/bottom CD, sidewall angle | Mid-z PAC slice (Inferno) | `"volumetric"` |

Mode selection is validated: a missing `[deep] mode` errors with the valid list, and an unknown mode is rejected (`unknown deep mode '<x>' (expected one of: liga, grayscale, interference, volumetric)`).

**LIGA spectrum sourcing:** `liga` needs a bending-magnet white beam. Give `[deep] critical_energy_kev` directly, or set `[source] type = "synchrotron"` with `beamline = "bending_magnet"` and it derives the critical energy from the machine. A synchrotron *undulator* source is rejected (quasi-monochromatic — no white-beam spectrum), with a message pointing to the fix.

### Examples

Each mode has a heavily-commented runnable config in `examples/`:

```bash
# LIGA: 500 µm PMMA, 6.23 keV bending magnet -> developed-depth PNG
highuvlith deep --config examples/liga.toml --output depth.png

# Grayscale blazed grating -> JSON summary with RMS error
highuvlith deep --config examples/grayscale.toml --output relief.json

# Two-beam interference grating at 266 nm -> mid-z PAC slice PNG
highuvlith deep --config examples/interference.toml --output fringes.png

# Volumetric exposure -> mid-z PAC slice PNG + sidewall angle on stderr
highuvlith deep --config examples/volumetric.toml --output slice.png
```

See [`examples/liga.toml`](../examples/liga.toml), [`grayscale.toml`](../examples/grayscale.toml), [`interference.toml`](../examples/interference.toml), and [`volumetric.toml`](../examples/volumetric.toml) — each documents its `[deep]` keys inline. The full `[deep]` schema is in [Configuration → `[deep]`](./configuration.md#deep).
