//! End-to-end: every `examples/*.toml` runs through the `highuvlith` binary
//! (the subcommand is chosen by the config's tables: `[deep]` → `deep`,
//! `[optimize]` → `optimize`, otherwise `simulate`), plus the commands that
//! `docs/cli.md` documents without a config (`sources`, `materials`) and the
//! config-driven `sweep` / `throughput`.
//!
//! Every example runs with its own settings except `volumetric.toml`, whose
//! 128² × 32 level-set volume takes ~90 s in a debug build: the test runs a
//! copy with a 32² × 8 volume (same field, same physics keys).

use std::path::{Path, PathBuf};
use std::process::Command;

fn examples_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples")
}

fn scratch_dir() -> PathBuf {
    let dir = std::env::temp_dir().join(format!("highuvlith-cli-examples-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

/// Run the binary; panic with its stderr unless it succeeds. Returns (stdout, stderr).
fn run(args: &[&str]) -> (String, String) {
    let out = Command::new(env!("CARGO_BIN_EXE_highuvlith"))
        .args(args)
        .output()
        .expect("failed to start highuvlith");
    let stdout = String::from_utf8_lossy(&out.stdout).to_string();
    let stderr = String::from_utf8_lossy(&out.stderr).to_string();
    assert!(
        out.status.success(),
        "highuvlith {} failed:\n{stderr}",
        args.join(" ")
    );
    (stdout, stderr)
}

/// The subcommand an example is meant for.
fn subcommand(text: &str) -> &'static str {
    let has = |table: &str| text.lines().any(|l| l.trim() == format!("[{table}]"));
    if has("deep") {
        "deep"
    } else if has("optimize") {
        "optimize"
    } else {
        "simulate"
    }
}

/// Reduced-size copies for the examples too slow for a debug test build.
fn test_copy(name: &str, text: &str) -> Option<String> {
    if name != "volumetric.toml" {
        return None;
    }
    let mut doc: toml::Table = text.parse().unwrap();
    let grid = doc["grid"].as_table_mut().unwrap();
    // 32 × 18.75 nm = 600 nm: the example's field (two 300 nm pitches).
    grid.insert("size".into(), toml::Value::Integer(32));
    grid.insert("pixel_nm".into(), toml::Value::Float(18.75));
    doc["deep"]
        .as_table_mut()
        .unwrap()
        .insert("nz".into(), toml::Value::Integer(8));
    Some(toml::to_string(&doc).unwrap())
}

fn run_example(name: &str) {
    let path = examples_dir().join(name);
    let text = std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{name}: {e}"));
    let cmd = subcommand(&text);
    let dir = scratch_dir();
    let config = match test_copy(name, &text) {
        Some(reduced) => {
            let p = dir.join(name);
            std::fs::write(&p, reduced).unwrap();
            p
        }
        None => path,
    };
    let json_path = dir.join(name.replace(".toml", ".json"));
    let (_, stderr) = run(&[
        cmd,
        "--config",
        config.to_str().unwrap(),
        "--output",
        json_path.to_str().unwrap(),
    ]);
    let json: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&json_path).unwrap())
            .unwrap_or_else(|e| panic!("{name}: output is not JSON: {e}"));
    assert!(json.is_object(), "{name}: JSON output is not an object");
    if cmd == "simulate" {
        // Bright-field sanity: no dark-field warning, and the image is
        // normalized to the clear field (a mostly blocked zero order shows up
        // as I_max far above the coherent-ringing ~1.3 of a 1:1 grating).
        assert!(!stderr.contains("dark-field"), "{name}:\n{stderr}");
        assert_eq!(json["dark_field_imaging"], false, "{name}");
        let i_max = json["i_max"].as_f64().unwrap();
        assert!(i_max < 1.5, "{name}: I_max {i_max} (zero order blocked?)");
    }
}

macro_rules! example_tests {
    ($($test:ident => $file:literal,)*) => {
        $(
            #[test]
            fn $test() {
                run_example($file);
            }
        )*
        const EXAMPLES: &[&str] = &[$($file),*];
    };
}

example_tests! {
    grayscale => "grayscale.toml",
    interference => "interference.toml",
    liga => "liga.toml",
    optimize_ilt => "optimize_ilt.toml",
    optimize_opc => "optimize_opc.toml",
    optimize_sadp => "optimize_sadp.toml",
    optimize_sraf => "optimize_sraf.toml",
    sim => "sim.toml",
    sim_arf => "sim_arf.toml",
    sim_betatron => "sim_betatron.toml",
    sim_dpp => "sim_dpp.toml",
    sim_entangled => "sim_entangled.toml",
    sim_hg_gline => "sim_hg_gline.toml",
    sim_hg_iline => "sim_hg_iline.toml",
    sim_hhg => "sim_hhg.toml",
    sim_ics => "sim_ics.toml",
    sim_krf => "sim_krf.toml",
    sim_lpa_fel => "sim_lpa_fel.toml",
    sim_lpp_gd => "sim_lpp_gd.toml",
    sim_lpp_sn => "sim_lpp_sn.toml",
    sim_smith_purcell => "sim_smith_purcell.toml",
    sim_ssmb => "sim_ssmb.toml",
    sim_sxrl => "sim_sxrl.toml",
    sim_synchrotron => "sim_synchrotron.toml",
    sim_xfel => "sim_xfel.toml",
    sim_xfel_cw_sc => "sim_xfel_cw_sc.toml",
    sim_xfel_erl => "sim_xfel_erl.toml",
    sim_xray_tube => "sim_xray_tube.toml",
    talbot => "talbot.toml",
    volumetric => "volumetric.toml",
}

/// A new example file must be added to the list above.
#[test]
fn every_example_is_covered() {
    let mut on_disk: Vec<String> = std::fs::read_dir(examples_dir())
        .unwrap()
        .filter_map(|e| {
            let name = e.unwrap().file_name().to_string_lossy().to_string();
            name.ends_with(".toml").then_some(name)
        })
        .collect();
    on_disk.sort();
    let mut listed: Vec<String> = EXAMPLES.iter().map(|s| s.to_string()).collect();
    listed.sort();
    assert_eq!(on_disk, listed);
}

#[test]
fn sweep_writes_bossung_and_ed_window() {
    let dir = scratch_dir();
    let out = dir.join("sweep.json");
    let config = examples_dir().join("sim.toml");
    run(&[
        "sweep",
        "--config",
        config.to_str().unwrap(),
        "--focus-range",
        "-100,100,3",
        "--dose-range",
        "25,35,3",
        "--output",
        out.to_str().unwrap(),
    ]);
    let json: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&out).unwrap()).unwrap();
    assert!(json.is_object());
}

#[test]
fn simulate_and_sweep_note_unread_subcommand_tables() {
    let dir = scratch_dir();
    let text = std::fs::read_to_string(examples_dir().join("sim.toml")).unwrap();
    let config = dir.join("sim_with_throughput.toml");
    std::fs::write(
        &config,
        format!("{text}\n[throughput]\npreset = \"refractive\"\n"),
    )
    .unwrap();
    let (_, stderr) = run(&["simulate", "--config", config.to_str().unwrap()]);
    assert!(
        stderr.contains("note: [throughput] is not read by simulate (ignored"),
        "{stderr}"
    );
    let (_, stderr) = run(&[
        "sweep",
        "--config",
        config.to_str().unwrap(),
        "--focus-range",
        "0,0,1",
    ]);
    assert!(
        stderr.contains("note: [throughput] is not read by sweep"),
        "{stderr}"
    );
    let (_, stderr) = run(&[
        "simulate",
        "--config",
        examples_dir().join("sim.toml").to_str().unwrap(),
    ]);
    assert!(!stderr.contains("is not read by"), "{stderr}");
}

#[test]
fn throughput_json() {
    let config = examples_dir().join("sim_lpp_sn.toml");
    let (stdout, _) = run(&["throughput", "--config", config.to_str().unwrap(), "--json"]);
    let json: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    assert!(json.is_object());
}

#[test]
fn sources_and_materials_commands() {
    let (stdout, _) = run(&["sources", "--json"]);
    let json: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    assert!(json.as_array().is_some_and(|a| a.len() >= 40));
    run(&["sources"]);
    run(&["sources", "--family", "xray_tube"]);
    run(&["materials"]);
    run(&["materials", "--name", "CaF2", "--wavelength", "157.63"]);
    let (stdout, _) = run(&["materials", "--multilayer", "mo_si", "--json"]);
    let json: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    assert!(json.is_object());
}
