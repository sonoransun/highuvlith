//! `highuvlith` command-line interface.
//!
//! A clap-based CLI over the `highuvlith-core` engine with four subcommands:
//! `simulate` (aerial image for one dose/focus condition), `sweep` (dose/focus
//! process-window analysis), `deep` (deep-layer 3D / high-aspect-ratio process
//! modes: LIGA, grayscale, interference, volumetric), and `materials` (query
//! the built-in optical constants). Runs are configured from TOML files (see
//! `examples/sim.toml`); results serialize to JSON.

mod commands;
mod config;

use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "highuvlith", version, about = "VUV lithography simulation CLI")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Compute aerial image for a single condition
    Simulate {
        /// Path to TOML configuration file
        #[arg(short, long)]
        config: std::path::PathBuf,

        /// Output file path (.json)
        #[arg(short, long)]
        output: Option<std::path::PathBuf>,

        /// Override focus value (nm)
        #[arg(long)]
        focus: Option<f64>,

        /// Override dose value (mJ/cm²)
        #[arg(long)]
        dose: Option<f64>,
    },

    /// Sweep dose and/or focus to compute process window
    Sweep {
        /// Path to TOML configuration file
        #[arg(short, long)]
        config: std::path::PathBuf,

        /// Output file path (.json)
        #[arg(short, long)]
        output: Option<std::path::PathBuf>,

        /// Focus range: start,stop,steps (e.g., "-200,200,21")
        #[arg(long, default_value = "-200,200,11")]
        focus_range: String,

        /// Dose range: start,stop,steps (e.g., "20,50,15")
        #[arg(long)]
        dose_range: Option<String>,
    },

    /// Deep-layer 3D / high-aspect-ratio process modes (LIGA, grayscale,
    /// interference, volumetric). Mode is selected by `[deep] mode` in the TOML.
    Deep {
        /// Path to TOML configuration file
        #[arg(short, long)]
        config: std::path::PathBuf,

        /// Output file path (.png writes the mode's image; otherwise .json)
        #[arg(short, long)]
        output: Option<std::path::PathBuf>,
    },

    /// Query the VUV materials database
    Materials {
        /// Evaluate at this wavelength (nm)
        #[arg(long)]
        wavelength: Option<f64>,

        /// Specific material to query
        #[arg(long)]
        name: Option<String>,
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
        Commands::Materials { wavelength, name } => {
            commands::materials::run(wavelength, name.as_deref())
        }
    }
}
