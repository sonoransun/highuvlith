//! `highuvlith` desktop GUI.
//!
//! An egui/eframe application over `highuvlith-core`: every registered
//! light-source family with its machine parameters and derived physics,
//! projection imaging (dry / immersion refractive, EUV and other reflective
//! optics; scalar or vector; one wavelength, narrow band or an exact TCC per
//! wavelength) on an automatically commensurate grid, the dose-aware process
//! window, and a volume viewer for the volumetric resist, LIGA and Talbot
//! outputs. All physics runs on background threads; the UI never blocks.
//!
//! ```text
//! cargo run -p highuvlith-gui --release -- [options]
//!   --preset NAME        source preset (e.g. arf, lpp-sn, xfel-flash, tube-w)
//!   --optics NAME        optics preset: 193i | nxe | high-na
//!   --tab NAME           aerial | cross-section | process-window | volume | liga | source
//!   --dataset NAME       volume dataset: latent | developed | liga | talbot
//!   --cd NM, --pitch NM  mask line CD and pitch
//!   --size WxH           window size in points (default 1500x950)
//!   --screenshot FILE    save a PNG of the window once the view is computed, then exit
//! ```

mod app;
mod colormap;
mod compute;
mod imaging;
mod jobs;
mod panels;
mod png;
mod sources;
mod ui_state;
mod views;
mod volume;
mod widgets;

use eframe::egui;

const USAGE: &str = "usage: highuvlith-gui [--preset NAME] [--optics 193i|nxe|high-na] \
[--tab aerial|cross-section|process-window|volume|liga|source] \
[--dataset latent|developed|liga|talbot] [--cd NM] [--pitch NM] [--size WxH] \
[--screenshot FILE.png]";

/// Parsed command line.
struct Cli {
    start: app::StartOptions,
    size: [f32; 2],
}

fn parse_args(args: &[String]) -> Result<Cli, String> {
    let mut start = app::StartOptions::default();
    let mut size = [1500.0, 950.0];
    let mut it = args.iter();
    while let Some(flag) = it.next() {
        let mut value = || {
            it.next()
                .cloned()
                .ok_or_else(|| format!("{flag} needs a value"))
        };
        match flag.as_str() {
            "--preset" => {
                let v = value()?;
                start.preset = Some(
                    sources::PresetId::from_cli_name(&v)
                        .ok_or_else(|| format!("unknown preset '{v}'"))?,
                );
            }
            "--optics" => {
                let v = value()?;
                start.optics = Some(
                    imaging::OpticsPreset::from_cli_name(&v)
                        .ok_or_else(|| format!("unknown optics preset '{v}'"))?,
                );
            }
            "--tab" => {
                let v = value()?;
                start.tab = Some(
                    ui_state::Tab::from_cli_name(&v).ok_or_else(|| format!("unknown tab '{v}'"))?,
                );
            }
            "--dataset" => {
                let v = value()?;
                start.dataset = Some(
                    volume::DatasetKind::from_cli_name(&v)
                        .ok_or_else(|| format!("unknown dataset '{v}'"))?,
                );
            }
            "--cd" => start.cd_nm = Some(parse_num(&value()?)?),
            "--pitch" => start.pitch_nm = Some(parse_num(&value()?)?),
            "--size" => {
                let v = value()?;
                let (w, h) = v
                    .split_once('x')
                    .ok_or_else(|| format!("--size expects WxH, got '{v}'"))?;
                size = [parse_num(w)? as f32, parse_num(h)? as f32];
            }
            "--screenshot" => start.screenshot = Some(value()?.into()),
            "-h" | "--help" => return Err(USAGE.to_string()),
            other => return Err(format!("unknown argument '{other}'\n{USAGE}")),
        }
    }
    Ok(Cli { start, size })
}

fn parse_num(s: &str) -> Result<f64, String> {
    s.parse::<f64>()
        .ok()
        .filter(|v| v.is_finite() && *v > 0.0)
        .ok_or_else(|| format!("expected a positive number, got '{s}'"))
}

fn main() -> eframe::Result {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let cli = match parse_args(&args) {
        Ok(cli) => cli,
        Err(msg) => {
            eprintln!("{msg}");
            std::process::exit(if args.iter().any(|a| a == "-h" || a == "--help") {
                0
            } else {
                2
            });
        }
    };
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size(cli.size)
            .with_title("highuvlith \u{2014} VUV to X-ray lithography simulator"),
        ..Default::default()
    };
    let start = cli.start;
    eframe::run_native(
        "highuvlith",
        options,
        Box::new(move |_cc| Ok(Box::new(app::LithApp::new(start)))),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn args(s: &str) -> Vec<String> {
        s.split_whitespace().map(String::from).collect()
    }

    #[test]
    fn parses_presets_views_and_screenshot() {
        let cli = parse_args(&args(
            "--preset arf --optics 193i --tab process-window --cd 45 --pitch 90 \
             --size 1200x800 --screenshot out.png --dataset talbot",
        ))
        .unwrap();
        assert_eq!(cli.start.preset, Some(sources::PresetId::ArF));
        assert_eq!(cli.start.optics, Some(imaging::OpticsPreset::Immersion193i));
        assert_eq!(cli.start.tab, Some(ui_state::Tab::ProcessWindow));
        assert_eq!(cli.start.dataset, Some(volume::DatasetKind::TalbotCarpet));
        assert_eq!(
            (cli.start.cd_nm, cli.start.pitch_nm),
            (Some(45.0), Some(90.0))
        );
        assert_eq!(cli.size, [1200.0, 800.0]);
        assert_eq!(cli.start.screenshot, Some("out.png".into()));
        assert!(parse_args(&args("")).unwrap().start.preset.is_none());
    }

    #[test]
    fn rejects_bad_arguments() {
        assert!(parse_args(&args("--preset nonsense")).is_err());
        assert!(parse_args(&args("--tab")).is_err());
        assert!(parse_args(&args("--cd -3")).is_err());
        assert!(parse_args(&args("--size 12")).is_err());
        assert!(parse_args(&args("--frobnicate")).is_err());
    }
}
