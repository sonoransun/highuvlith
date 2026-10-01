//! Left-panel controls: source (family, presets, machine parameters,
//! derived quantities, pupil fill), optics, imaging settings, mask, grid,
//! process, and the LIGA exposure for X-ray sources.

use std::ops::RangeInclusive;

use eframe::egui::{self, RichText};
use highuvlith_core::aerial::{DefocusModel, ImageNormalization};
use highuvlith_core::source::{HhgSource, IcsSource, SmithPurcellSource, SourceKind};
use highuvlith_core::source_models::lpp::LppDriveLaser;
use highuvlith_core::source_models::physics::HC_EV_NM;

use crate::app::LithApp;
use crate::compute::{rayleigh_dof_nm, LigaFilter};
use crate::imaging::{
    Coating, OpticsKind, OpticsPreset, Pattern, PolarizationChoice, SpectrumMode,
};
use crate::sources::{
    describe_illumination, gas_label, preset_info, Family, IllumMode, LambdaOrigin, Route,
    HHG_GASES, SXRL_SCHEMES, XRAY_ANODES,
};
use crate::widgets::{badge_chip, fmt_num, kv_table, warning_line};

fn slider(ui: &mut egui::Ui, v: &mut f64, range: RangeInclusive<f64>, text: &str) {
    ui.add(egui::Slider::new(v, range).text(text));
}

fn slider_log(ui: &mut egui::Ui, v: &mut f64, range: RangeInclusive<f64>, text: &str) {
    ui.add(
        egui::Slider::new(v, range)
            .logarithmic(true)
            .text(text)
            .custom_formatter(|x, _| fmt_num(x)),
    );
}

fn slider_int(ui: &mut egui::Ui, v: &mut usize, range: RangeInclusive<usize>, text: &str) {
    ui.add(egui::Slider::new(v, range).text(text));
}

fn note(ui: &mut egui::Ui, text: &str) {
    ui.label(RichText::new(text).small().weak());
}

impl LithApp {
    /// The whole left panel.
    pub(crate) fn draw_params(&mut self, ui: &mut egui::Ui) {
        ui.heading("highuvlith");
        note(
            ui,
            "VUV-to-X-ray lithography simulator \u{2014} every number below comes from \
             highuvlith-core.",
        );
        ui.separator();
        egui::CollapsingHeader::new(RichText::new("Source").strong())
            .default_open(true)
            .show(ui, |ui| self.source_section(ui));
        match self.source.route() {
            Route::Projection => {
                egui::CollapsingHeader::new(RichText::new("Optics").strong())
                    .default_open(true)
                    .show(ui, |ui| self.optics_section(ui));
                egui::CollapsingHeader::new(RichText::new("Imaging settings").strong())
                    .default_open(true)
                    .show(ui, |ui| self.imaging_section(ui));
                egui::CollapsingHeader::new(RichText::new("Mask").strong())
                    .default_open(true)
                    .show(ui, |ui| self.mask_section(ui));
                egui::CollapsingHeader::new(RichText::new("Grid").strong())
                    .default_open(true)
                    .show(ui, |ui| self.grid_section(ui));
                egui::CollapsingHeader::new(RichText::new("Focus & threshold").strong())
                    .default_open(true)
                    .show(ui, |ui| self.process_section(ui));
            }
            Route::Liga => {
                egui::CollapsingHeader::new(RichText::new("LIGA exposure").strong())
                    .default_open(true)
                    .show(ui, |ui| self.liga_section(ui));
            }
        }
    }

    // -----------------------------------------------------------------------
    // Source
    // -----------------------------------------------------------------------

    fn source_section(&mut self, ui: &mut egui::Ui) {
        let mut family = self.source.family;
        egui::ComboBox::from_id_salt("family_combo")
            .width(300.0)
            .selected_text(family.label())
            .show_ui(ui, |ui| {
                for f in Family::ALL {
                    ui.selectable_value(&mut family, f, f.label());
                }
            });
        if family != self.source.family {
            self.source.select_family(family);
        }

        // Honesty status of the family (mirrors its docs page).
        let status = self.source.family.status();
        let emoji_ok = self.emoji_ok.unwrap_or(false);
        ui.horizontal_wrapped(|ui| {
            for b in status.badges {
                badge_chip(ui, *b, emoji_ok);
            }
        });
        note(ui, status.summary);
        note(ui, &format!("Docs: {}", status.doc));

        // Presets.
        ui.label(RichText::new("Presets").small());
        let mut chosen = None;
        ui.horizontal_wrapped(|ui| {
            for p in self.source.family.presets() {
                let selected = self.source.preset == p.id;
                if ui
                    .selectable_label(selected, p.label)
                    .on_hover_text(p.note)
                    .clicked()
                {
                    chosen = Some(p.id);
                }
            }
        });
        if let Some(id) = chosen {
            if let Err(e) = self.source.apply_preset(id) {
                self.status = Some(format!("preset failed: {e}"));
            }
        }
        let info = preset_info(self.source.preset);
        if info.family == self.source.family {
            ui.horizontal_wrapped(|ui| {
                if let Some(b) = info.badge {
                    badge_chip(ui, b, emoji_ok);
                }
                note(ui, info.note);
            });
            if let Some(doc) = info.doc {
                note(ui, &format!("Docs: {doc}"));
            }
        }

        // Wavelength: set-point, fixed line, or derived.
        ui.separator();
        match &self.summary.built {
            Ok(src) => {
                let lambda = highuvlith_core::source::LithographySource::wavelength_nm(src);
                let origin = match self.source.family.lambda_origin() {
                    LambdaOrigin::SetPoint => "set-point".to_string(),
                    LambdaOrigin::FixedLine => "fixed by the emitting line / band".to_string(),
                    LambdaOrigin::Derived(f) => format!("DERIVED from the machine: {f}"),
                };
                ui.label(
                    RichText::new(format!(
                        "\u{3bb} = {} nm  ({} eV)",
                        fmt_num(lambda),
                        fmt_num(HC_EV_NM / lambda)
                    ))
                    .strong(),
                );
                note(ui, &origin);
            }
            Err(e) => warning_line(ui, &format!("Invalid source: {e}")),
        }
        if self.source.route() == Route::Liga {
            warning_line(
                ui,
                "Broadband, incoherent X-rays: not a projection-imaging source. Shown in the \
                 LIGA view (deep X-ray shadow printing).",
            );
        }

        ui.separator();
        self.family_controls(ui);

        egui::CollapsingHeader::new("Derived quantities")
            .default_open(false)
            .show(ui, |ui| self.derived_table(ui, "derived_left", false));
        if self.source.route() == Route::Projection {
            egui::CollapsingHeader::new("Pupil fill (illumination)")
                .default_open(false)
                .show(ui, |ui| self.illumination_controls(ui));
        }
    }

    /// Derived-quantities table (`with_notes`: notes as a column, else hover).
    pub(crate) fn derived_table(&self, ui: &mut egui::Ui, id: &str, with_notes: bool) {
        if self.summary.derived.is_empty() {
            note(ui, "No derived quantities.");
            return;
        }
        note(
            ui,
            "Physics derived from the machine parameters (informational: the imaging \
             pipeline never reads these).",
        );
        egui::Grid::new(id)
            .num_columns(if with_notes { 4 } else { 3 })
            .striped(true)
            .spacing([10.0, 3.0])
            .show(ui, |ui| {
                for q in &self.summary.derived {
                    let name = ui.label(q.name.replace('_', " "));
                    if !with_notes {
                        name.on_hover_text(&q.note);
                    }
                    ui.label(RichText::new(fmt_num(q.value)).monospace());
                    ui.label(RichText::new(&q.unit).weak());
                    if with_notes {
                        ui.label(RichText::new(&q.note).small().weak());
                    }
                    ui.end_row();
                }
            });
    }

    fn illumination_controls(&mut self, ui: &mut egui::Ui) {
        let il = &mut self.source.illumination;
        egui::ComboBox::from_id_salt("illum_mode")
            .selected_text(il.mode.label())
            .show_ui(ui, |ui| {
                for m in IllumMode::ALL {
                    ui.selectable_value(&mut il.mode, m, m.label());
                }
            });
        let uses_sigma = self.source.family.default_pupil_uses_sigma();
        match il.mode {
            IllumMode::SourceDefault if uses_sigma => {
                slider(ui, &mut il.sigma, 0.05..=1.0, "\u{3c3}");
            }
            IllumMode::SourceDefault => {
                note(
                    ui,
                    "Beam-like source: the pupil fill is the Gaussian derived from its transverse \
                     coherence.",
                );
            }
            IllumMode::Conventional | IllumMode::CoherentGaussian => {
                slider(ui, &mut il.sigma, 0.01..=1.0, "\u{3c3}");
            }
            IllumMode::Annular => {
                slider(ui, &mut il.sigma_inner, 0.0..=0.95, "\u{3c3} inner");
                slider(ui, &mut il.sigma_outer, 0.05..=1.0, "\u{3c3} outer");
            }
            IllumMode::Dipole | IllumMode::Quadrupole => {
                slider(ui, &mut il.pole_center, 0.05..=1.0, "pole centre \u{3c3}c");
                slider(ui, &mut il.pole_radius, 0.01..=0.5, "pole radius");
                let label = if il.mode == IllumMode::Dipole {
                    "orientation (\u{b0})"
                } else {
                    "opening angle (\u{b0})"
                };
                slider(ui, &mut il.angle_deg, 0.0..=90.0, label);
            }
        }
        if let Ok(src) = &self.summary.built {
            note(
                ui,
                &format!(
                    "Pupil fill used: {}",
                    describe_illumination(src.illumination())
                ),
            );
        }
    }

    fn family_controls(&mut self, ui: &mut egui::Ui) {
        let p = &mut self.source;
        match p.family {
            Family::Vuv => {
                let v = &mut p.vuv;
                slider_log(
                    ui,
                    &mut v.wavelength_nm,
                    100.0..=450.0,
                    "\u{3bb} set-point (nm)",
                );
                note(
                    ui,
                    "Presets put \u{3bb} on a physical emitter line; other values are what-ifs.",
                );
                slider_log(ui, &mut v.bandwidth_pm, 0.05..=5000.0, "FWHM (pm)");
                ui.horizontal(|ui| {
                    ui.radio_value(&mut v.gaussian_line, false, "Lorentzian");
                    ui.radio_value(&mut v.gaussian_line, true, "Gaussian line");
                });
                let mut cw = v.is_cw();
                if ui.checkbox(&mut cw, "CW lamp (no pulses)").changed() {
                    v.rep_rate_hz = if cw { 0.0 } else { 4000.0 };
                    if !cw && v.pulse_energy_mj <= 0.0 {
                        v.pulse_energy_mj = 10.0;
                    }
                }
                if !cw {
                    slider_log(ui, &mut v.pulse_energy_mj, 0.1..=50.0, "pulse energy (mJ)");
                    slider_log(ui, &mut v.rep_rate_hz, 10.0..=10_000.0, "rep rate (Hz)");
                }
            }
            Family::LpaFel => {
                let v = &mut p.lpa_fel;
                slider_log(
                    ui,
                    &mut v.electron_energy_mev,
                    20.0..=2000.0,
                    "electron energy (MeV)",
                );
                slider(ui, &mut v.undulator_period_mm, 5.0..=50.0, "\u{3bb}u (mm)");
                slider(ui, &mut v.undulator_k, 0.1..=5.0, "undulator K");
                slider_int(ui, &mut v.num_periods, 10..=2000, "periods");
                note(
                    ui,
                    &format!(
                        "Resonance \u{3bb} = {} nm (derived)",
                        fmt_num(v.resonance_nm())
                    ),
                );
                slider_log(
                    ui,
                    &mut v.pulse_energy_uj,
                    0.01..=1000.0,
                    "pulse energy (\u{b5}J)",
                );
                slider_log(ui, &mut v.rep_rate_hz, 0.1..=10_000.0, "rep rate (Hz)");
                slider_log(ui, &mut v.pulse_duration_fs, 1.0..=200.0, "pulse (fs)");
                egui::CollapsingHeader::new("Electron beam")
                    .default_open(false)
                    .show(ui, |ui| {
                        slider_log(
                            ui,
                            &mut v.peak_current_a,
                            10.0..=20_000.0,
                            "peak current (A)",
                        );
                        slider_log(
                            ui,
                            &mut v.norm_emittance_um,
                            0.05..=10.0,
                            "\u{3b5}n (\u{b5}m)",
                        );
                        slider_log(ui, &mut v.energy_spread_rel, 1e-4..=0.05, "\u{3c3}\u{3b4}");
                        slider_log(ui, &mut v.beta_m, 0.1..=20.0, "\u{3b2} (m)");
                    });
            }
            Family::Lpp => {
                let v = &mut p.lpp;
                note(
                    ui,
                    &format!("Fuel {:?} (choose the fuel with the presets).", v.fuel),
                );
                let before = v.drive_laser;
                egui::ComboBox::from_id_salt("lpp_drive")
                    .selected_text(v.drive_laser.label())
                    .show_ui(ui, |ui| {
                        for d in [
                            LppDriveLaser::Co2,
                            LppDriveLaser::SolidState1um,
                            LppDriveLaser::Thulium2um,
                        ] {
                            ui.selectable_value(&mut v.drive_laser, d, d.label());
                        }
                    });
                if v.drive_laser != before && v.fuel == highuvlith_core::source::LppFuel::Sn {
                    v.conversion_efficiency = v.drive_laser.sn_conversion_efficiency().0;
                }
                slider_log(
                    ui,
                    &mut v.drive_power_w,
                    500.0..=100_000.0,
                    "drive power (W)",
                );
                slider_log(
                    ui,
                    &mut v.conversion_efficiency,
                    1e-3..=0.1,
                    "CE (into 2\u{3c0})",
                );
                slider(
                    ui,
                    &mut v.transport_efficiency,
                    0.01..=1.0,
                    "2\u{3c0}-to-IF efficiency",
                );
                slider_log(ui, &mut v.rep_rate_hz, 1000.0..=200_000.0, "rep rate (Hz)");
            }
            Family::Synchrotron => {
                let v = &mut p.synchrotron;
                ui.horizontal(|ui| {
                    ui.radio_value(&mut v.bending_magnet, false, "Undulator");
                    ui.radio_value(&mut v.bending_magnet, true, "Bending magnet (LIGA)");
                });
                if v.bending_magnet {
                    slider(ui, &mut v.energy_gev, 0.3..=8.0, "ring energy (GeV)");
                    slider(ui, &mut v.field_t, 0.2..=10.0, "bending field (T)");
                } else {
                    slider(ui, &mut v.energy_gev, 0.1..=8.0, "ring energy (GeV)");
                    slider(ui, &mut v.period_mm, 5.0..=100.0, "\u{3bb}u (mm)");
                    slider(ui, &mut v.k, 0.1..=5.0, "K");
                    slider_int(ui, &mut v.num_periods, 10..=1000, "periods");
                    ui.horizontal(|ui| {
                        ui.label("harmonic");
                        for h in [1usize, 3, 5, 7, 9] {
                            ui.selectable_value(&mut v.harmonic, h, h.to_string());
                        }
                    });
                    ui.checkbox(&mut v.use_ring_beam, "coherence from the ring emittances");
                    if v.use_ring_beam {
                        slider_log(
                            ui,
                            &mut v.emittance_x_nm_rad,
                            0.01..=100.0,
                            "\u{3b5}x (nm\u{b7}rad)",
                        );
                        slider_log(
                            ui,
                            &mut v.emittance_y_nm_rad,
                            0.001..=10.0,
                            "\u{3b5}y (nm\u{b7}rad)",
                        );
                    }
                }
                slider(ui, &mut v.ring_current_ma, 1.0..=500.0, "ring current (mA)");
            }
            Family::Hhg => {
                let v = &mut p.hhg;
                slider(
                    ui,
                    &mut v.driver_wavelength_nm,
                    400.0..=2000.0,
                    "driver \u{3bb} (nm)",
                );
                egui::ComboBox::from_id_salt("hhg_gas")
                    .selected_text(gas_label(v.gas))
                    .show_ui(ui, |ui| {
                        for g in HHG_GASES {
                            ui.selectable_value(&mut v.gas, g, gas_label(g));
                        }
                    });
                slider_log(
                    ui,
                    &mut v.intensity_w_cm2,
                    1e13..=3e15,
                    "intensity (W/cm\u{b2})",
                );
                let cutoff = HhgSource::cutoff_energy_ev_for(
                    v.gas,
                    v.intensity_w_cm2,
                    v.driver_wavelength_nm,
                );
                let q_max = (cutoff / (HC_EV_NM / v.driver_wavelength_nm)).floor() as usize;
                let q_max_odd = if q_max.is_multiple_of(2) {
                    q_max.saturating_sub(1)
                } else {
                    q_max
                };
                let mut q = v.harmonic as f64;
                if ui
                    .add(
                        egui::Slider::new(&mut q, 3.0..=201.0)
                            .step_by(2.0)
                            .text("harmonic q"),
                    )
                    .changed()
                {
                    let q = q.round() as usize;
                    v.harmonic = if q.is_multiple_of(2) { q + 1 } else { q };
                }
                note(
                    ui,
                    &format!(
                        "cutoff Ip + 3.17 Up = {} eV, so q \u{2264} {q_max_odd}",
                        fmt_num(cutoff)
                    ),
                );
                slider_log(
                    ui,
                    &mut v.mono_bandwidth_pm,
                    1.0..=1000.0,
                    "mono bandwidth (pm)",
                );
                let mut derive = v.driver_power_w.is_some();
                ui.checkbox(
                    &mut derive,
                    "derive power from the driver (P \u{d7} \u{3b7}q)",
                );
                if derive {
                    let mut pw = v.driver_power_w.unwrap_or(5.0);
                    slider_log(ui, &mut pw, 0.01..=1000.0, "driver average power (W)");
                    v.driver_power_w = Some(pw);
                } else {
                    v.driver_power_w = None;
                    slider_log(
                        ui,
                        &mut v.pulse_energy_nj,
                        1e-3..=100.0,
                        "stored pulse energy (nJ)",
                    );
                }
                slider_log(ui, &mut v.rep_rate_hz, 100.0..=1e7, "rep rate (Hz)");
                ui.checkbox(&mut v.full_comb, "full harmonic comb (no monochromator)");
                if v.full_comb {
                    let mut band = v.passband_nm.is_some();
                    ui.checkbox(&mut band, "band-pass filter");
                    if band {
                        let [mut lo, mut hi] = v.passband_nm.unwrap_or([12.5, 14.5]);
                        slider_log(ui, &mut lo, 1.0..=200.0, "band min (nm)");
                        slider_log(ui, &mut hi, 1.0..=200.0, "band max (nm)");
                        v.passband_nm = Some([lo.min(hi), lo.max(hi)]);
                    } else {
                        v.passband_nm = None;
                    }
                    note(
                        ui,
                        "Comb imaging is honest only with the per-wavelength spectrum mode (one \
                         TCC per harmonic).",
                    );
                }
            }
            Family::Xfel => {
                let v = &mut p.xfel;
                ui.horizontal(|ui| {
                    ui.radio_value(&mut v.seeded, false, "SASE");
                    ui.radio_value(&mut v.seeded, true, "Seeded");
                });
                if v.seeded {
                    slider_log(
                        ui,
                        &mut v.seeded_rel_bandwidth,
                        1e-6..=1e-3,
                        "\u{394}\u{3bb}/\u{3bb}",
                    );
                }
                slider_log(
                    ui,
                    &mut v.wavelength_nm,
                    0.1..=100.0,
                    "\u{3bb} set-point (nm)",
                );
                if let Ok(SourceKind::Xfel(s)) = &self.summary.built {
                    if let Some(k) = s.undulator_k() {
                        note(
                            ui,
                            &format!("Gap-tuned undulator K = {} (derived)", fmt_num(k)),
                        );
                    }
                }
                egui::CollapsingHeader::new("Machine")
                    .default_open(true)
                    .show(ui, |ui| {
                        slider_log(
                            ui,
                            &mut v.electron_energy_mev,
                            100.0..=20_000.0,
                            "electron energy (MeV)",
                        );
                        slider(ui, &mut v.undulator_period_mm, 10.0..=80.0, "\u{3bb}u (mm)");
                        slider_log(
                            ui,
                            &mut v.undulator_length_m,
                            5.0..=200.0,
                            "undulator length (m)",
                        );
                        slider_log(
                            ui,
                            &mut v.peak_current_a,
                            50.0..=10_000.0,
                            "peak current (A)",
                        );
                        slider_log(
                            ui,
                            &mut v.norm_emittance_um,
                            0.1..=5.0,
                            "\u{3b5}n (\u{b5}m)",
                        );
                        slider_log(ui, &mut v.energy_spread_rel, 1e-5..=1e-2, "\u{3c3}\u{3b4}");
                        slider_log(ui, &mut v.beta_m, 1.0..=50.0, "\u{3b2} (m)");
                    });
                slider_log(
                    ui,
                    &mut v.pulse_energy_uj,
                    0.1..=5000.0,
                    "pulse energy (\u{b5}J)",
                );
                slider_log(ui, &mut v.pulse_duration_fs, 1.0..=500.0, "pulse (fs)");
                slider_log(ui, &mut v.rep_rate_hz, 1.0..=1e9, "rep rate (Hz)");
            }
            Family::Ics => {
                let v = &mut p.ics;
                slider_log(
                    ui,
                    &mut v.electron_energy_mev,
                    0.5..=100.0,
                    "electron energy (MeV)",
                );
                slider_log(
                    ui,
                    &mut v.laser_wavelength_nm,
                    200.0..=10_600.0,
                    "laser \u{3bb} (nm)",
                );
                slider(ui, &mut v.laser_a0, 0.0..=1.0, "a\u{2080}");
                if ui.button("Tune the electron energy to 13.5 nm").clicked() {
                    if let Ok(s) =
                        IcsSource::for_wavelength(13.5, v.laser_wavelength_nm, v.laser_a0)
                    {
                        v.electron_energy_mev = s.electron_energy_mev;
                    }
                }
                slider_log(
                    ui,
                    &mut v.collection_half_angle_mrad,
                    0.1..=100.0,
                    "collection (mrad)",
                );
                slider_log(ui, &mut v.energy_spread_rel, 1e-4..=0.05, "\u{3c3}E");
                slider_log(ui, &mut v.rep_rate_hz, 1.0..=1e9, "collision rate (Hz)");
                egui::CollapsingHeader::new("Collision")
                    .default_open(false)
                    .show(ui, |ui| {
                        slider_log(
                            ui,
                            &mut v.bunch_charge_pc,
                            1.0..=1000.0,
                            "bunch charge (pC)",
                        );
                        slider_log(
                            ui,
                            &mut v.laser_pulse_energy_mj,
                            0.01..=100.0,
                            "laser pulse (mJ)",
                        );
                        slider_log(
                            ui,
                            &mut v.electron_spot_um,
                            1.0..=100.0,
                            "electron spot (\u{b5}m)",
                        );
                        slider_log(
                            ui,
                            &mut v.laser_spot_um,
                            1.0..=100.0,
                            "laser spot (\u{b5}m)",
                        );
                    });
            }
            Family::Ssmb => {
                let v = &mut p.ssmb;
                slider(
                    ui,
                    &mut v.ring_energy_mev,
                    100.0..=3000.0,
                    "ring energy (MeV)",
                );
                slider(
                    ui,
                    &mut v.modulation_wavelength_nm,
                    200.0..=2000.0,
                    "modulation \u{3bb} (nm)",
                );
                slider_int(ui, &mut v.harmonic, 1..=200, "harmonic h");
                note(
                    ui,
                    &format!(
                        "\u{3bb} = \u{3bb}mod / h = {} nm (derived)",
                        fmt_num(v.wavelength_nm())
                    ),
                );
                slider_log(
                    ui,
                    &mut v.average_current_a,
                    0.01..=10.0,
                    "average current (A)",
                );
                slider_log(ui, &mut v.peak_current_a, 0.01..=100.0, "peak current (A)");
                slider_log(ui, &mut v.bunching_factor, 1e-4..=1.0, "bunching factor b");
                slider_int(ui, &mut v.radiator_periods, 10..=1000, "radiator periods");
                slider(ui, &mut v.radiator_k, 0.1..=5.0, "radiator K");
            }
            Family::Entangled => {
                let v = &mut p.entangled;
                slider_log(
                    ui,
                    &mut v.wavelength_nm,
                    100.0..=1000.0,
                    "photon \u{3bb} (nm)",
                );
                slider_int(ui, &mut v.n_photons, 2..=10, "N (NOON)");
                slider(ui, &mut v.fidelity, 0.0..=1.0, "fidelity");
                slider_log(ui, &mut v.pair_rate_hz, 1.0..=1e15, "pair rate (Hz)");
            }
            Family::XrayTube => {
                let v = &mut p.xray_tube;
                egui::ComboBox::from_id_salt("anode")
                    .selected_text(format!("{} anode", v.anode.symbol()))
                    .show_ui(ui, |ui| {
                        for a in XRAY_ANODES {
                            ui.selectable_value(&mut v.anode, a, format!("{} anode", a.symbol()));
                        }
                    });
                slider(ui, &mut v.kvp, 5.0..=300.0, "tube voltage (kV)");
                slider_log(ui, &mut v.current_ma, 0.1..=100.0, "tube current (mA)");
                slider(ui, &mut v.be_window_um, 0.0..=1000.0, "Be window (\u{b5}m)");
            }
            Family::Dpp => {
                let v = &mut p.dpp;
                note(ui, &format!("Fuel {:?} (choose with the presets).", v.fuel));
                slider_log(
                    ui,
                    &mut v.electrical_power_w,
                    100.0..=100_000.0,
                    "electrical power (W)",
                );
                slider_log(
                    ui,
                    &mut v.conversion_efficiency,
                    5e-4..=0.1,
                    "CE (into 2\u{3c0})",
                );
                slider(
                    ui,
                    &mut v.collector_solid_angle_sr,
                    0.1..=std::f64::consts::TAU,
                    "collector \u{3a9} (sr)",
                );
                slider(
                    ui,
                    &mut v.collector_efficiency,
                    0.05..=1.0,
                    "collector efficiency",
                );
                slider_log(
                    ui,
                    &mut v.source_diameter_mm,
                    0.05..=5.0,
                    "pinch \u{d8} (mm)",
                );
                slider_log(ui, &mut v.source_length_mm, 0.1..=10.0, "pinch length (mm)");
                slider_log(
                    ui,
                    &mut v.illuminator_etendue_mm2_sr,
                    0.1..=100.0,
                    "illuminator \u{e9}tendue (mm\u{b2}sr)",
                );
                slider_log(ui, &mut v.rep_rate_hz, 100.0..=100_000.0, "rep rate (Hz)");
            }
            Family::Sxrl => {
                let v = &mut p.sxrl;
                egui::ComboBox::from_id_salt("sxrl_scheme")
                    .selected_text(format!(
                        "{} ({} nm)",
                        v.scheme.transition(),
                        v.scheme.wavelength_nm()
                    ))
                    .show_ui(ui, |ui| {
                        for s in SXRL_SCHEMES {
                            ui.selectable_value(
                                &mut v.scheme,
                                s,
                                format!("{} nm \u{2014} {}", s.wavelength_nm(), s.transition()),
                            );
                        }
                    });
                slider_log(
                    ui,
                    &mut v.pulse_energy_uj,
                    0.01..=1000.0,
                    "pulse energy (\u{b5}J)",
                );
                slider_log(ui, &mut v.rep_rate_hz, 0.1..=1000.0, "rep rate (Hz)");
                slider_log(ui, &mut v.pulse_duration_ps, 0.1..=5000.0, "pulse (ps)");
                slider(ui, &mut v.coherence, 0.01..=1.0, "coherent fraction");
                slider_log(
                    ui,
                    &mut v.rel_linewidth,
                    1e-5..=1e-2,
                    "\u{394}\u{3bb}/\u{3bb}",
                );
            }
            Family::Betatron => {
                let v = &mut p.betatron;
                slider_log(
                    ui,
                    &mut v.electron_energy_mev,
                    10.0..=2000.0,
                    "electron energy (MeV)",
                );
                slider_log(
                    ui,
                    &mut v.plasma_density_cm3,
                    1e17..=1.5e21,
                    "plasma density (1/cm\u{b3})",
                );
                slider_log(
                    ui,
                    &mut v.betatron_amplitude_um,
                    0.1..=10.0,
                    "r\u{3b2} (\u{b5}m)",
                );
                slider_log(ui, &mut v.interaction_length_mm, 0.5..=30.0, "length (mm)");
                slider_log(ui, &mut v.bunch_charge_pc, 1.0..=1000.0, "charge (pC)");
                slider_log(ui, &mut v.rep_rate_hz, 0.1..=10_000.0, "rep rate (Hz)");
            }
            Family::SmithPurcell => {
                let v = &mut p.smith_purcell;
                slider_log(
                    ui,
                    &mut v.electron_energy_kev,
                    1.0..=300.0,
                    "electron energy (keV)",
                );
                slider_log(
                    ui,
                    &mut v.grating_period_nm,
                    1.0..=1000.0,
                    "grating period (nm)",
                );
                slider_int(ui, &mut v.diffraction_order, 1..=5, "order m");
                slider(
                    ui,
                    &mut v.observation_angle_deg,
                    1.0..=179.0,
                    "observation angle (\u{b0})",
                );
                if ui.button("Set period for 13.5 nm").clicked() {
                    if let Ok(s) = SmithPurcellSource::for_wavelength(
                        13.5,
                        v.electron_energy_kev,
                        v.diffraction_order,
                        v.observation_angle_deg,
                    ) {
                        v.grating_period_nm = s.grating_period_nm;
                    }
                }
                slider_log(ui, &mut v.beam_current_na, 0.1..=10_000.0, "current (nA)");
                slider_int(ui, &mut v.num_periods, 10..=10_000, "periods");
                slider(
                    ui,
                    &mut v.impact_height_nm,
                    0.0..=100.0,
                    "impact height (nm)",
                );
                slider_log(
                    ui,
                    &mut v.coupling_efficiency,
                    1e-6..=1.0,
                    "coupling \u{3b5} (assumed)",
                );
            }
        }
    }

    // -----------------------------------------------------------------------
    // Optics and imaging settings
    // -----------------------------------------------------------------------

    fn optics_section(&mut self, ui: &mut egui::Ui) {
        let lambda = self.summary.wavelength_nm().unwrap_or(157.63);
        let o = &mut self.optics;
        egui::ComboBox::from_id_salt("optics_kind")
            .width(260.0)
            .selected_text(o.kind.label())
            .show_ui(ui, |ui| {
                for k in OpticsKind::ALL {
                    ui.selectable_value(&mut o.kind, k, k.label());
                }
            });
        let mut preset = None;
        ui.horizontal_wrapped(|ui| {
            for p in OpticsPreset::ALL {
                if ui.button(p.label()).clicked() {
                    preset = Some(p);
                }
            }
        });
        if let Some(p) = preset {
            self.apply_optics_preset(p);
        }
        let o = &mut self.optics;
        let kind = o.kind.resolve(lambda);
        let (lo, hi) = kind.na_range(o.immersion_index);
        slider(ui, &mut o.na, lo..=hi, "NA");
        if kind == OpticsKind::Immersion {
            slider(ui, &mut o.immersion_index, 1.0..=1.8, "immersion index n");
            note(
                ui,
                "Water at 193 nm: n = 1.437. NA may exceed 1, up to 0.95\u{b7}n.",
            );
        }
        if kind.has_obscuration() {
            slider(
                ui,
                &mut o.central_obscuration,
                0.0..=0.6,
                "central obscuration (\u{b7}NA)",
            );
            note(
                ui,
                "High-NA designs are centrally obscured; the 0.2\u{b7}NA preset value is an \
                 assumption.",
            );
        }
        slider(ui, &mut o.flare, 0.0..=0.1, "flare fraction");
        if kind.has_obscuration() {
            let m = &mut o.multilayer;
            ui.checkbox(&mut m.enabled, "angle-dependent multilayer pupil")
                .on_hover_text(
                    "Each ray reflects off every coated mirror at its own incidence angle \
                     (Parratt multilayer response): pupil apodization and phase. With the \
                     clear-field normalization it changes the image shape, not the dose.",
                );
            if m.enabled {
                let mut coating = m.coating;
                egui::ComboBox::from_label("coating")
                    .selected_text(coating.label())
                    .show_ui(ui, |ui| {
                        for c in Coating::ALL {
                            ui.selectable_value(&mut coating, c, c.label());
                        }
                    });
                if coating != m.coating {
                    m.set_coating(coating);
                }
                slider(ui, &mut m.period_nm, 2.0..=10.0, "period (nm)");
                let mut mirrors = m.mirrors as f64;
                ui.add(
                    egui::Slider::new(&mut mirrors, 1.0..=10.0)
                        .step_by(1.0)
                        .text("coated mirrors"),
                );
                m.mirrors = mirrors.round() as usize;
                slider(
                    ui,
                    &mut m.center_deg,
                    0.0..=25.0,
                    "\u{3b8} at pupil centre (\u{b0})",
                );
                slider(
                    ui,
                    &mut m.radial_deg,
                    -20.0..=20.0,
                    "radial \u{394}\u{3b8} at rim (\u{b0})",
                );
                slider(
                    ui,
                    &mut m.tilt_deg,
                    -20.0..=20.0,
                    "linear tilt at rim (\u{b0})",
                );
                if m.tilt_deg != 0.0 {
                    slider(ui, &mut m.azimuth_deg, 0.0..=180.0, "tilt azimuth (\u{b0})");
                }
                match m.center_reflectance(lambda) {
                    Ok(r) => note(
                        ui,
                        &format!(
                            "Single-mirror reflectance at {} nm, pupil centre: {:.1} %",
                            fmt_num(lambda),
                            100.0 * r
                        ),
                    ),
                    Err(e) => warning_line(ui, &e),
                }
                note(
                    ui,
                    "The incidence-angle map is an ASSUMPTION (same map on every mirror), not a \
                     ray trace of a real design (status: Simplified).",
                );
            }
        }
        match &self.scene_cache.resolved {
            Ok(r) => {
                note(ui, &format!("Using: {}", r.optics.label));
                for w in &r.optics.warnings {
                    warning_line(ui, w);
                }
                if kind == OpticsKind::EuvProjection {
                    note(
                        ui,
                        "Isotropic wafer-side pupil: anamorphic 4\u{d7}/8\u{d7} magnification \
                         and mask-3D are not modeled; multilayer apodization only through the \
                         optional angle map above (status: Simplified).",
                    );
                }
            }
            Err(e) => warning_line(ui, e),
        }
    }

    fn imaging_section(&mut self, ui: &mut egui::Ui) {
        let im = &mut self.imaging;
        ui.label("Defocus model");
        ui.horizontal(|ui| {
            ui.radio_value(&mut im.defocus_model, DefocusModel::Exact, "Exact")
                .on_hover_text("Defocus inside the pupil at every source point (exact).");
            ui.radio_value(
                &mut im.defocus_model,
                DefocusModel::KernelPhase,
                "Legacy kernel phase",
            )
            .on_hover_text(
                "In-focus kernels times the on-axis paraxial phase: wrong through focus for \
                     off-axis illumination (kept for comparison).",
            );
        });
        ui.label("Imaging model");
        ui.horizontal(|ui| {
            ui.radio_value(&mut im.vector, false, "Scalar");
            ui.radio_value(&mut im.vector, true, "Vector (polarized)");
        });
        if im.vector {
            egui::ComboBox::from_id_salt("polarization")
                .selected_text(im.polarization.label())
                .show_ui(ui, |ui| {
                    for p in PolarizationChoice::ALL {
                        ui.selectable_value(&mut im.polarization, p, p.label());
                    }
                });
            ui.checkbox(&mut im.obliquity, "radiometric (obliquity) factor");
            note(
                ui,
                "TE/TM pupil per diffraction order; thin mask, ideal lens (no Jones pupil). The \
                 image medium is the optics' immersion index.",
            );
        }
        ui.label("Intensity scale");
        ui.horizontal(|ui| {
            ui.radio_value(
                &mut im.normalization,
                ImageNormalization::ClearField,
                "Relative (clear field = 1)",
            );
            ui.radio_value(
                &mut im.normalization,
                ImageNormalization::Absolute,
                "Absolute",
            );
        });
        ui.label("Spectrum");
        egui::ComboBox::from_id_salt("spectrum_mode")
            .width(260.0)
            .selected_text(im.spectrum.label())
            .show_ui(ui, |ui| {
                for m in SpectrumMode::ALL {
                    ui.selectable_value(&mut im.spectrum, m, m.label());
                }
            });
        let mut override_samples = im.spectral_samples.is_some();
        ui.checkbox(&mut override_samples, "override spectral samples");
        if override_samples {
            let mut n = im.spectral_samples.unwrap_or(5);
            slider_int(ui, &mut n, 1..=21, "samples");
            im.spectral_samples = Some(n);
        } else {
            im.spectral_samples = None;
            if let Ok(src) = &self.summary.built {
                note(
                    ui,
                    &format!(
                        "Source sampling: {} spectral samples.",
                        src.spectral_samples()
                    ),
                );
            }
        }
        slider_int(
            ui,
            &mut self.imaging.max_kernels,
            1..=96,
            "max SOCS kernels",
        );
    }

    // -----------------------------------------------------------------------
    // Mask, grid, focus
    // -----------------------------------------------------------------------

    fn mask_section(&mut self, ui: &mut egui::Ui) {
        let m = &mut self.mask;
        ui.horizontal(|ui| {
            ui.radio_value(&mut m.pattern, Pattern::LineSpace, "Lines / spaces");
            ui.radio_value(&mut m.pattern, Pattern::ContactArray, "Contact array");
        });
        slider_log(ui, &mut m.pitch_nm, 5.0..=5000.0, "pitch (nm)");
        let cd_label = match m.pattern {
            Pattern::LineSpace => "line CD (nm)",
            Pattern::ContactArray => "hole size (nm)",
        };
        let max_cd = 0.95 * m.pitch_nm;
        m.cd_nm = m.cd_nm.min(max_cd);
        slider_log(ui, &mut m.cd_nm, 1.0..=max_cd.max(1.0), cd_label);
        ui.checkbox(&mut m.reverse_tone, "reverse field tone");
        ui.checkbox(&mut m.att_psm, "attenuated PSM (6 %, 180\u{b0})");
        note(
            ui,
            "Thin (Kirchhoff) mask, exact Fourier-series spectrum; no mask-3D.",
        );
    }

    fn grid_section(&mut self, ui: &mut egui::Ui) {
        let g = &mut self.grid;
        ui.horizontal(|ui| {
            ui.label("pixels");
            for s in [64usize, 128, 256, 512] {
                ui.selectable_value(&mut g.size, s, s.to_string());
            }
        });
        ui.checkbox(&mut g.auto_pixel, "automatic pixel (Nyquist-safe)");
        if g.auto_pixel {
            slider_int(ui, &mut g.periods, 1..=8, "pitches in the field");
        } else {
            slider_log(ui, &mut g.pixel_nm, 0.05..=200.0, "target pixel (nm)");
        }
        match &self.scene_cache.resolved {
            Ok(r) => {
                let c = &r.grid;
                kv_table(
                    ui,
                    "grid_choice",
                    &[
                        (
                            "Field".into(),
                            format!("{} nm = {} \u{d7} pitch", fmt_num(c.field_nm), c.periods),
                        ),
                        (
                            "Pixel".into(),
                            format!("{} nm ({}\u{b2})", fmt_num(c.pixel_nm), c.size),
                        ),
                        (
                            "Nyquist-safe pixel".into(),
                            format!("\u{2264} {} nm", fmt_num(c.nyquist_pixel_nm)),
                        ),
                    ],
                );
                if c.resolves_band {
                    note(ui, "The field holds whole pitches (commensurate) and the pixel represents the full band (1+\u{3c3})NA/\u{3bb}.");
                } else {
                    warning_line(
                        ui,
                        "One pitch does not fit below the Nyquist-safe pixel: part of the band is \
                         lost. Increase the pixel count.",
                    );
                }
            }
            Err(e) => warning_line(ui, e),
        }
    }

    fn process_section(&mut self, ui: &mut egui::Ui) {
        let lambda = self.summary.wavelength_nm().unwrap_or(157.63);
        let medium = if self.optics.kind.resolve(lambda) == OpticsKind::Immersion {
            self.optics.immersion_index
        } else {
            1.0
        };
        let rdof = rayleigh_dof_nm(lambda, self.optics.na.max(1e-3), medium);
        let span = (6.0 * rdof).clamp(20.0, 5000.0);
        self.focus_nm = self.focus_nm.clamp(-span, span);
        slider(ui, &mut self.focus_nm, -span..=span, "focus (nm)");
        note(
            ui,
            &format!("Rayleigh DOF n\u{3bb}/(2NA\u{b2}) = {} nm", fmt_num(rdof)),
        );
        slider(ui, &mut self.threshold, 0.02..=0.98, "print threshold (I)");
        note(
            ui,
            "Constant-threshold resist: the printed edge is the intensity contour at this \
             threshold (CD and NILS readouts).",
        );
    }

    // -----------------------------------------------------------------------
    // LIGA exposure (X-ray tube, betatron, bending magnet)
    // -----------------------------------------------------------------------

    fn liga_section(&mut self, ui: &mut egui::Ui) {
        let l = &mut self.liga;
        let family = self.source.family;
        slider_log(
            ui,
            &mut l.resist_thickness_um,
            20.0..=3000.0,
            "PMMA thickness (\u{b5}m)",
        );
        slider(ui, &mut l.gap_um, 0.0..=1000.0, "proximity gap (\u{b5}m)");
        slider(ui, &mut l.absorber_um, 1.0..=60.0, "Au absorber (\u{b5}m)");
        slider(ui, &mut l.membrane_um, 0.5..=20.0, "Ti membrane (\u{b5}m)");
        ui.horizontal(|ui| {
            ui.label("filter");
            for f in LigaFilter::ALL {
                ui.selectable_value(&mut l.filter, f, f.label());
            }
        });
        if l.filter != LigaFilter::None {
            slider_log(
                ui,
                &mut l.filter_um,
                1.0..=1000.0,
                "filter thickness (\u{b5}m)",
            );
        }
        slider(
            ui,
            &mut l.target_bottom_dose,
            0.5..=10.0,
            "bottom dose (kJ/cm\u{b3})",
        );
        slider(
            ui,
            &mut l.damage_dose,
            5.0..=60.0,
            "damage ceiling (kJ/cm\u{b3})",
        );
        slider(
            ui,
            &mut l.develop_threshold,
            0.5..=10.0,
            "developer threshold (kJ/cm\u{b3})",
        );
        ui.horizontal(|ui| {
            ui.radio_value(&mut l.fresnel, true, "Fresnel diffraction");
            ui.radio_value(&mut l.fresnel, false, "Gaussian blur (legacy)");
        });
        slider_int(ui, &mut l.energy_bins, 10..=200, "energy bins");
        ui.checkbox(&mut l.absolute, "absolute flux (gives the exposure time)");
        if l.absolute {
            if family == Family::Synchrotron {
                slider(ui, &mut l.bm_distance_m, 1.0..=50.0, "source distance (m)");
                slider(
                    ui,
                    &mut l.bm_acceptance_mrad,
                    0.5..=20.0,
                    "horizontal acceptance (mrad)",
                );
                slider(ui, &mut l.bm_scan_mm, 5.0..=200.0, "vertical scan (mm)");
            } else {
                slider_log(
                    ui,
                    &mut l.distance_mm,
                    10.0..=10_000.0,
                    "source distance (mm)",
                );
            }
        }
        ui.separator();
        ui.label("Mask (Au bars) and grid");
        slider_log(ui, &mut l.pitch_um, 2.0..=500.0, "pitch (\u{b5}m)");
        l.cd_um = l.cd_um.min(0.95 * l.pitch_um);
        slider_log(
            ui,
            &mut l.cd_um,
            0.5..=(0.95 * l.pitch_um).max(0.5),
            "bar width (\u{b5}m)",
        );
        ui.horizontal(|ui| {
            ui.label("pixels");
            for s in [32usize, 64, 128] {
                ui.selectable_value(&mut l.grid_size, s, s.to_string());
            }
        });
        slider_int(ui, &mut l.nz, 8..=96, "depth slices");
        note(
            ui,
            "Same model as `highuvlith deep` LIGA mode (NIST \u{3bc}/\u{3bc}en, Henke \u{3b4}). \
             Status: Simplified, see docs/processes/liga-deep-xray.md.",
        );
    }

    /// Apply an optics preset and keep its NA (no automatic NA reset).
    pub(crate) fn apply_optics_preset(&mut self, preset: OpticsPreset) {
        self.optics.apply_preset(preset);
        self.mark_optics_kind_current();
    }
}
