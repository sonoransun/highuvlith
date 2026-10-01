//! `highuvlith` command-line interface.
//!
//! A clap-based CLI over the `highuvlith-core` engine. Subcommands:
//!
//! - `simulate` — aerial image, contrast and printed CD for one dose/focus;
//! - `sweep` — dose × focus process window (Bossung curves, ED window);
//! - `deep` — deep-layer 3D / high-aspect-ratio process modes (LIGA,
//!   grayscale, interference, volumetric, Talbot);
//! - `optimize` — mask optimization (ILT, fragment OPC, SRAF insertion) and
//!   spacer-patterning (SADP / SAQP) geometry;
//! - `throughput` — dose-limited wafers per hour for a configured source;
//! - `sources` — every source family and preset with its derived physics;
//! - `materials` — optical constants (VUV tables, Henke X-ray/EUV) and
//!   EUV/BEUV multilayer mirrors.
//!
//! Runs are configured from TOML files (`examples/*.toml`, reference in
//! `docs/configuration.md`); results serialize to JSON (or PNG images).

mod commands;
mod config;
mod keys;

use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(
    name = "highuvlith",
    version,
    about = "Lithography simulation from VUV to X-ray (CLI over highuvlith-core)",
    long_about = "Lithography simulation from VUV to X-ray (CLI over highuvlith-core).\n\n\
        Config-driven commands (simulate, sweep, deep, optimize, throughput) read a TOML file \
        with strict keys: an unknown key, or one the selected type / mode / method does not \
        read, is an error. Fourteen [source] families: vuv, lpa_fel, lpp, synchrotron, hhg, \
        xfel, ics, ssmb, entangled, xray_tube, dpp, sxrl, betatron, smith_purcell. Summaries \
        go to stderr; --output writes JSON (or a PNG image for simulate / deep / optimize). \
        Reference: docs/cli.md and docs/configuration.md."
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Aerial image, contrast and printed CD for one dose/focus condition
    ///
    /// Reads [source] [optics] [mask] (required) and [illumination] [grid]
    /// [process] [imaging]; scalar or vector Hopkins imaging, exact defocus,
    /// polychromatic spectra and a constant-threshold resist. A [deep],
    /// [optimize] or [throughput] table is noted on stderr and ignored.
    Simulate {
        /// Path to TOML configuration file
        #[arg(short, long)]
        config: std::path::PathBuf,

        /// Output file path (.png writes the aerial image; otherwise JSON)
        #[arg(short, long)]
        output: Option<std::path::PathBuf>,

        /// Override focus value (nm)
        #[arg(long, allow_hyphen_values = true)]
        focus: Option<f64>,

        /// Exposure dose (mJ/cm²) relative to [process] dose_mj_cm2: the
        /// printed contour moves to I = threshold · dose_mj_cm2 / dose
        #[arg(long)]
        dose: Option<f64>,
    },

    /// Sweep dose and/or focus to compute the process window
    ///
    /// Same tables as `simulate`. Computes the CD at every dose × focus point
    /// with a constant-threshold resist, the Bossung curves and the
    /// exposure-defocus window against the [process] CD target and
    /// tolerance. A [deep], [optimize] or [throughput] table is noted on
    /// stderr and ignored.
    Sweep {
        /// Path to TOML configuration file
        #[arg(short, long)]
        config: std::path::PathBuf,

        /// Output file path (.json)
        #[arg(short, long)]
        output: Option<std::path::PathBuf>,

        /// Focus range: start,stop,steps (e.g. "-200,200,21")
        #[arg(long, default_value = "-200,200,11", allow_hyphen_values = true)]
        focus_range: String,

        /// Dose range: start,stop,steps (e.g. "20,50,15"); default: the
        /// [process] dose only
        #[arg(long)]
        dose_range: Option<String>,
    },

    /// Deep-layer 3D / high-aspect-ratio process modes (LIGA, grayscale,
    /// interference, volumetric, Talbot). Mode is selected by `[deep] mode`.
    ///
    /// Modes: `liga` (X-ray depth dose and developed depth, optional absolute
    /// exposure time), `grayscale` (height map from a transmittance mask),
    /// `interference` (two-beam volumetric PAC), `volumetric` (3D resist
    /// exposure, PEB and threshold / FMM / level-set development), `talbot`
    /// (Talbot carpet, DTL / ATL images, two-grating EUV interference). Each
    /// mode reads only the tables it needs and notes the others as ignored.
    Deep {
        /// Path to TOML configuration file
        #[arg(short, long)]
        config: std::path::PathBuf,

        /// Output file path (.png writes the mode's image; otherwise JSON)
        #[arg(short, long)]
        output: Option<std::path::PathBuf>,
    },

    /// Mask optimization (ILT, fragment OPC, SRAF) and SADP/SAQP geometry.
    /// Method is selected by `[optimize] method`.
    ///
    /// Methods: `ilt` (adjoint or proxy gradient inverse lithography), `opc`
    /// (fragment model-based OPC), `sraf` (rule-deck assists with a model
    /// print check, optional DOF comparison), `sadp` / `saqp` (spacer
    /// patterning geometry; read only [optimize]). The imaging methods also
    /// read the `simulate` tables.
    Optimize {
        /// Path to TOML configuration file
        #[arg(short, long)]
        config: std::path::PathBuf,

        /// Output file path (.png writes the optimized mask; otherwise JSON)
        #[arg(short, long)]
        output: Option<std::path::PathBuf>,
    },

    /// Dose-limited wafers per hour for the configured [source]
    ///
    /// Simplified scanner model (illustrative presets, not vendor data):
    /// source power × optics × mask delivers the dose; fields are scanned at
    /// the dose-limited speed plus lumped overheads. The preset follows the
    /// wavelength (refractive ≥ 100 nm, euv_hvm 12.4–15 nm, beuv_la_b
    /// 6–7.5 nm); outside those bands (X-ray tubes, betatron, 25–47 nm
    /// lines) set [throughput] preset or optics_transmission.
    Throughput {
        /// Path to TOML configuration file ([source], optional [throughput])
        #[arg(short, long)]
        config: std::path::PathBuf,

        /// Output file path (.json)
        #[arg(short, long)]
        output: Option<std::path::PathBuf>,

        /// Resist dose-to-size (mJ/cm²); overrides [throughput] / [process]
        #[arg(long)]
        dose: Option<f64>,

        /// Print the result as JSON on stdout
        #[arg(long)]
        json: bool,
    },

    /// List every source family and preset with wavelength, power and
    /// derived physics
    ///
    /// 44 presets over the fourteen [source] families, each with its honesty
    /// badge and the [source] TOML that selects it.
    Sources {
        /// Only this family (`type` tag, e.g. lpp, xfel, xray_tube)
        #[arg(long)]
        family: Option<String>,

        /// Print JSON on stdout instead of the table
        #[arg(long)]
        json: bool,
    },

    /// Query optical constants and EUV/BEUV multilayer mirrors
    ///
    /// n, k (and dn/dλ) from the VUV tables within their data ranges, Henke
    /// δ/β for any compound (`henke:<formula>[@density]`), or the
    /// reflectance, peak, bandwidth and angular acceptance of an ideal Mo/Si,
    /// La/B4C or La/B multilayer.
    Materials {
        /// Evaluate at this wavelength (nm); default 157 nm, or 13.5 nm with
        /// --multilayer
        #[arg(long)]
        wavelength: Option<f64>,

        /// Specific material (a table name, or `henke:<formula>[@density]`)
        #[arg(long)]
        name: Option<String>,

        /// Multilayer mirror preset: mo_si, la_b4c, or la_b
        #[arg(long)]
        multilayer: Option<String>,

        /// Multilayer bilayer count (default: preset)
        #[arg(long)]
        periods: Option<usize>,

        /// Multilayer period in nm (default: preset)
        #[arg(long)]
        period_nm: Option<f64>,

        /// Multilayer absorber fraction Γ (default: preset)
        #[arg(long)]
        gamma: Option<f64>,

        /// Incidence angle from the normal in degrees (default 0)
        #[arg(long)]
        angle_deg: Option<f64>,

        /// Print JSON on stdout
        #[arg(long)]
        json: bool,
    },
}

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Simulate {
            config,
            output,
            focus,
            dose,
        } => commands::simulate::run(&config, output.as_deref(), focus, dose),
        Commands::Sweep {
            config,
            output,
            focus_range,
            dose_range,
        } => commands::sweep::run(
            &config,
            output.as_deref(),
            &focus_range,
            dose_range.as_deref(),
        ),
        Commands::Deep { config, output } => commands::deep::run(&config, output.as_deref()),
        Commands::Optimize { config, output } => {
            commands::optimize::run(&config, output.as_deref())
        }
        Commands::Throughput {
            config,
            output,
            dose,
            json,
        } => commands::throughput::run(&config, output.as_deref(), dose, json),
        Commands::Sources { family, json } => commands::sources::run(family.as_deref(), json),
        Commands::Materials {
            wavelength,
            name,
            multilayer,
            periods,
            period_nm,
            gamma,
            angle_deg,
            json,
        } => commands::materials::run(&commands::materials::MaterialsArgs {
            wavelength,
            name,
            multilayer,
            periods,
            period_nm,
            gamma,
            angle_deg,
            json,
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::CommandFactory;

    fn long_help(sub: Option<&str>) -> String {
        let mut cmd = Cli::command();
        let cmd = match sub {
            Some(name) => cmd.find_subcommand_mut(name).expect("subcommand"),
            None => &mut cmd,
        };
        cmd.get_long_about().expect("long help text").to_string()
    }

    /// Drift guard: the `--help` text names every family, mode, method and
    /// preset the config code accepts.
    #[test]
    fn help_lists_current_families_modes_methods_and_presets() {
        let top = long_help(None);
        assert!(top.contains("Fourteen [source] families"));
        assert_eq!(config::SOURCE_TYPES.len(), 14);
        for t in config::SOURCE_TYPES {
            assert!(top.contains(t), "top-level help misses family {t}");
        }
        let deep = long_help(Some("deep"));
        for m in commands::deep::DEEP_MODES {
            assert!(
                deep.contains(&format!("`{m}`")),
                "deep help misses mode {m}"
            );
        }
        let optimize = long_help(Some("optimize"));
        for m in commands::optimize::OPTIMIZE_METHODS {
            assert!(
                optimize.contains(&format!("`{m}`")),
                "optimize help misses {m}"
            );
        }
        let throughput = long_help(Some("throughput"));
        for p in commands::throughput::THROUGHPUT_PRESETS {
            assert!(throughput.contains(p), "throughput help misses preset {p}");
        }
        let n_presets = commands::sources::PRESETS.len();
        assert!(long_help(Some("sources")).contains(&format!("{n_presets} presets")));
        Cli::command().debug_assert();
    }
}
