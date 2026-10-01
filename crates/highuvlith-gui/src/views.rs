//! Central-panel views.

use std::sync::Arc;

use eframe::egui::{self, RichText};
use egui_plot::{HLine, Legend, Line, LineStyle, Plot, PlotPoints, Points, Polygon, VLine};
use highuvlith_core::metrics;
use highuvlith_core::source::LithographySource;
use ndarray::Array2;

use crate::app::LithApp;
use crate::colormap::Colormap;
use crate::compute::{xray_source_spectrum, DevelopChoice, PebChoice};
use crate::sources::Route;
use crate::ui_state::{dataset_available, Tab};
use crate::volume::{DatasetKind, RangeMode, VolumeData};
use crate::widgets::{
    self, badge_chip, colorbar, colorize, fmt_num, heatmap, key_of, kv_table, muted, ordinal,
    series, warning_line, Heatmap,
};

fn note(ui: &mut egui::Ui, text: &str) {
    ui.label(RichText::new(text).small().weak());
}

/// "computed in … ms" plus a stale marker while newer inputs compute.
fn freshness(ui: &mut egui::Ui, elapsed: std::time::Duration, current: bool) {
    let text = format!("computed in {:.0} ms", elapsed.as_secs_f64() * 1e3);
    if current {
        note(ui, &text);
    } else {
        ui.horizontal(|ui| {
            ui.spinner();
            note(
                ui,
                &format!("{text} \u{2014} showing the previous inputs while updating"),
            );
        });
    }
}

fn waiting(ui: &mut egui::Ui, what: &str) {
    ui.horizontal(|ui| {
        ui.spinner();
        ui.label(format!("Computing the {what}\u{2026}"));
    });
}

fn colormap_combo(ui: &mut egui::Ui, id: &str, cmap: &mut Colormap) {
    egui::ComboBox::from_id_salt(id)
        .selected_text(cmap.label())
        .show_ui(ui, |ui| {
            for c in Colormap::ALL {
                ui.selectable_value(cmap, c, c.label());
            }
        });
}

/// Save a heatmap exactly as displayed (native resolution, same colours).
fn save_heatmap_png(
    path: &std::path::Path,
    data: &Array2<f64>,
    cmap: Colormap,
    range: (f64, f64),
    dark: bool,
    flip_rows: bool,
) -> String {
    let img = colorize(data, cmap, range, dark, flip_rows);
    let [w, h] = img.size;
    let rgba: Vec<u8> = img.pixels.iter().flat_map(|c| c.to_array()).collect();
    match crate::png::write_rgba(path, w as u32, h as u32, &rgba) {
        Ok(()) => format!("saved {}", path.display()),
        Err(e) => format!("export failed: {e}"),
    }
}

/// The dataset a view shows, its summary rows and its notes.
type VolumeSelection<'a> = (
    Option<&'a Arc<VolumeData>>,
    Vec<(String, String)>,
    Vec<String>,
);

/// Split `(x, y)` samples at non-finite y into drawable segments.
fn finite_segments(points: impl IntoIterator<Item = (f64, f64)>) -> Vec<Vec<[f64; 2]>> {
    let mut out = Vec::new();
    let mut cur: Vec<[f64; 2]> = Vec::new();
    for (x, y) in points {
        if y.is_finite() {
            cur.push([x, y]);
        } else if !cur.is_empty() {
            out.push(std::mem::take(&mut cur));
        }
    }
    if !cur.is_empty() {
        out.push(cur);
    }
    out
}

/// Closed outline of a process rectangle (focus × dose).
fn rect_outline(r: &highuvlith_core::process::ProcessRectangle) -> Vec<[f64; 2]> {
    vec![
        [r.focus_min_nm, r.dose_min_mj_cm2],
        [r.focus_max_nm, r.dose_min_mj_cm2],
        [r.focus_max_nm, r.dose_max_mj_cm2],
        [r.focus_min_nm, r.dose_max_mj_cm2],
        [r.focus_min_nm, r.dose_min_mj_cm2],
    ]
}

impl LithApp {
    fn projection_only(&self, ui: &mut egui::Ui) -> bool {
        if self.source.route() == Route::Liga {
            warning_line(
                ui,
                "The selected source is a broadband X-ray source for LIGA / proximity printing; \
                 projection imaging does not apply. Open the LIGA view.",
            );
            return false;
        }
        true
    }

    // -----------------------------------------------------------------------
    // Aerial image
    // -----------------------------------------------------------------------

    pub(crate) fn view_aerial(&mut self, ui: &mut egui::Ui) {
        if !self.projection_only(ui) {
            return;
        }
        let dark = ui.visuals().dark_mode;
        let Some(fin) = &self.aerial.latest else {
            waiting(ui, "aerial image");
            return;
        };
        let r = match &fin.result {
            Ok(r) => r,
            Err(e) => {
                warning_line(ui, e);
                return;
            }
        };
        let mut save = false;
        ui.horizontal(|ui| {
            ui.label("Colormap");
            colormap_combo(ui, "aerial_cmap", &mut self.aerial_colormap);
            save = ui
                .button("Save PNG")
                .on_hover_text("The image at its native resolution, with the colours shown")
                .clicked();
            ui.label(
                RichText::new("saved to the working directory")
                    .weak()
                    .small(),
            )
            .on_hover_text(&self.export_dir);
        });
        let cmap = self.aerial_colormap;
        let range = (r.i_min, r.i_max);
        let half = r.half_field_nm;
        let size = (ui.available_width() - 90.0)
            .min(ui.available_height() - 140.0)
            .max(240.0);
        let mut hover = None;
        ui.horizontal(|ui| {
            ui.vertical(|ui| {
                ui.set_width(size);
                hover = heatmap(
                    ui,
                    &mut self.tex_aerial,
                    &Heatmap {
                        id: "aerial_plot",
                        data: &r.image,
                        x_range: (-half, half),
                        v_range: (-half, half),
                        depth_down: false,
                        colormap: cmap,
                        value_range: range,
                        x_label: "x (nm)".into(),
                        v_label: "y (nm)".into(),
                        equal_aspect: true,
                        key: key_of(("aerial", self.generation, cmap as u8, dark)),
                        height: size,
                    },
                );
            });
            let label = match r.normalized {
                true => "I / I_clear",
                false => "I (absolute)",
            };
            colorbar(ui, cmap, range, label, size * 0.7);
        });
        match hover {
            Some(h) => {
                ui.label(format!(
                    "x = {} nm, y = {} nm (pixel row {}, column {}):  I = {}",
                    fmt_num(h.x),
                    fmt_num(h.v),
                    h.row,
                    h.col,
                    fmt_num(h.value)
                ));
            }
            None => note(
                ui,
                "Hover the image for values; drag to pan, double-click to reset.",
            ),
        }
        if save {
            let path = self.export_path("aerial");
            self.status = Some(save_heatmap_png(&path, &r.image, cmap, range, dark, true));
        }
        let current = self.aerial.is_current(&self.aerial_request());
        if let Some(fin) = &self.aerial.latest {
            freshness(ui, fin.elapsed, current);
        }
        ui.separator();
        self.aerial_metrics(ui);
    }

    fn aerial_metrics(&self, ui: &mut egui::Ui) {
        let Some(fin) = &self.aerial.latest else {
            return;
        };
        let Ok(r) = &fin.result else {
            return;
        };
        let thr = fin.request.threshold;
        let rows = vec![
            (
                "Contrast (I_max\u{2212}I_min)/(I_max+I_min)".to_string(),
                fmt_num(r.contrast),
            ),
            (
                "I_min / I_max".to_string(),
                format!("{} / {}", fmt_num(r.i_min), fmt_num(r.i_max)),
            ),
            (
                format!("Printed CD at I = {}", fmt_num(thr)),
                r.cd_nm.map_or("does not print".to_string(), |v| {
                    format!("{} nm", fmt_num(v))
                }),
            ),
            (
                "NILS".to_string(),
                r.nils.map_or("n/a".to_string(), fmt_num),
            ),
            (
                "k\u{2081} (half-pitch)".to_string(),
                format!(
                    "{}   (pitch cutoff \u{3bb}/(NA(1+\u{3c3})) = {} nm)",
                    fmt_num(r.k1_half_pitch),
                    fmt_num(r.cutoff_pitch_nm)
                ),
            ),
            ("Optics".to_string(), r.optics_label.clone()),
            (
                "Pupil fill".to_string(),
                format!(
                    "{} (support |\u{3c3}| \u{2264} {})",
                    r.illumination,
                    fmt_num(r.sigma_extent)
                ),
            ),
            (
                "SOCS kernels".to_string(),
                format!(
                    "{} (captured {:.4} of the TCC trace)",
                    r.num_kernels, r.captured_energy
                ),
            ),
            (
                "Clear field (absolute)".to_string(),
                format!(
                    "{}{}",
                    fmt_num(r.clear_field),
                    if r.normalized {
                        ""
                    } else {
                        " \u{2014} dark field: not normalized"
                    }
                ),
            ),
            (
                "Spectrum".to_string(),
                format!(
                    "{} ({} samples)",
                    fin.request.scene.imaging.spectrum.label(),
                    r.spectral_samples.len()
                ),
            ),
        ];
        kv_table(ui, "aerial_metrics", &rows);
        for w in &r.warnings {
            warning_line(ui, w);
        }
    }

    // -----------------------------------------------------------------------
    // Cross-section
    // -----------------------------------------------------------------------

    pub(crate) fn view_cross_section(&mut self, ui: &mut egui::Ui) {
        if !self.projection_only(ui) {
            return;
        }
        let dark = ui.visuals().dark_mode;
        let Some(fin) = &self.aerial.latest else {
            waiting(ui, "aerial image");
            return;
        };
        let r = match &fin.result {
            Ok(r) => r,
            Err(e) => {
                warning_line(ui, e);
                return;
            }
        };
        let thr = fin.request.threshold;
        let features = metrics::periodic_features(&r.profile, &r.profile_x, thr, r.tone);
        let points: Vec<[f64; 2]> = r
            .profile_x
            .iter()
            .zip(&r.profile)
            .map(|(&x, &i)| [x, i])
            .collect();
        let half = r.half_field_nm;
        Plot::new("cross_section")
            .height((ui.available_height() - 160.0).max(260.0))
            .x_axis_label("x (nm), y = 0")
            .y_axis_label(if r.normalized {
                "I / I_clear"
            } else {
                "I (absolute)"
            })
            .include_y(0.0)
            .include_x(-half)
            .include_x(half)
            .legend(Legend::default())
            .show(ui, |plot_ui| {
                plot_ui.line(
                    Line::new(PlotPoints::from(points))
                        .color(series(0, dark))
                        .width(2.0)
                        .name("aerial intensity"),
                );
                plot_ui.hline(
                    HLine::new(thr)
                        .color(muted(dark))
                        .style(LineStyle::Dashed { length: 8.0 })
                        .name(format!("threshold {}", fmt_num(thr))),
                );
                for f in &features {
                    for x in [f.left_nm, f.right_nm] {
                        // Wrap edges back into the field for display.
                        let w = 2.0 * half;
                        let xd = (x + half).rem_euclid(w) - half;
                        plot_ui.vline(VLine::new(xd).color(series(1, dark)).width(1.0));
                    }
                }
            });
        let cd = r
            .cd_nm
            .map_or("does not print".into(), |v| format!("{} nm", fmt_num(v)));
        ui.label(format!(
            "Printed feature ({:?} tone) at I = {}: CD {} \u{b7} NILS {} \u{b7} {} features in the field (edges marked)",
            r.tone,
            fmt_num(thr),
            cd,
            r.nils.map_or("n/a".into(), fmt_num),
            features.len()
        ));
        egui::CollapsingHeader::new("Data table")
            .default_open(false)
            .show(ui, |ui| {
                egui::ScrollArea::vertical()
                    .max_height(220.0)
                    .show(ui, |ui| {
                        egui::Grid::new("xs_table").striped(true).show(ui, |ui| {
                            ui.label(RichText::new("x (nm)").strong());
                            ui.label(RichText::new("I").strong());
                            ui.end_row();
                            for (x, i) in r.profile_x.iter().zip(&r.profile) {
                                ui.label(fmt_num(*x));
                                ui.label(fmt_num(*i));
                                ui.end_row();
                            }
                        });
                    });
            });
    }

    // -----------------------------------------------------------------------
    // Process window
    // -----------------------------------------------------------------------

    pub(crate) fn view_process_window(&mut self, ui: &mut egui::Ui) {
        if !self.projection_only(ui) {
            return;
        }
        let dark = ui.visuals().dark_mode;
        egui::CollapsingHeader::new("Sweep and spec")
            .default_open(true)
            .show(ui, |ui| {
                let p = &mut self.pw;
                ui.horizontal_wrapped(|ui| {
                    ui.add(egui::Slider::new(&mut p.dose_nominal, 1.0..=200.0).logarithmic(true).text("nominal dose (mJ/cm\u{b2})"));
                    ui.add(egui::Slider::new(&mut p.dose_span_pct, 2.0..=60.0).text("\u{b1} dose (%)"));
                    ui.add(egui::Slider::new(&mut p.n_doses, 3..=15).text("doses"));
                });
                ui.horizontal_wrapped(|ui| {
                    ui.checkbox(&mut p.focus_auto, "focus \u{b1}2 Rayleigh DOF");
                    if !p.focus_auto {
                        ui.add(egui::Slider::new(&mut p.focus_span_nm, 5.0..=3000.0).logarithmic(true).text("\u{b1} focus (nm)"));
                    }
                    ui.add(egui::Slider::new(&mut p.n_focus, 5..=31).text("focus planes"));
                });
                ui.horizontal_wrapped(|ui| {
                    ui.add(egui::Slider::new(&mut p.threshold, 0.05..=0.95).text("threshold I at nominal dose"));
                    let mut custom = p.target_cd_nm.is_some();
                    ui.checkbox(&mut custom, "target CD \u{2260} mask CD");
                    if custom {
                        let mut t = p.target_cd_nm.unwrap_or(self.mask.cd_nm);
                        ui.add(egui::Slider::new(&mut t, 1.0..=2000.0).logarithmic(true).text("target CD (nm)"));
                        p.target_cd_nm = Some(t);
                    } else {
                        p.target_cd_nm = None;
                    }
                    ui.add(egui::Slider::new(&mut p.tolerance_pct, 1.0..=30.0).text("\u{b1} CD tolerance (%)"));
                });
                note(
                    ui,
                    "Constant-threshold resist (status: Simplified): at dose d the printed edge is \
                     the contour I = E_th/d, E_th = threshold \u{d7} nominal dose. One aerial image \
                     per focus plane (exact defocus); CD on the y = 0 cut.",
                );
            });
        let Some(fin) = &self.pw_job.latest else {
            waiting(ui, "process window");
            return;
        };
        let r = match &fin.result {
            Ok(r) => r,
            Err(e) => {
                warning_line(ui, e);
                return;
            }
        };
        freshness(ui, fin.elapsed, self.pw_job.is_current(&self.pw_request()));
        let w = &r.window;
        let (spec_lo, spec_hi) = w.spec_limits_nm();
        let mut rows = vec![
            (
                "Best focus".to_string(),
                // Sub-picometre residue of the focus fit is numerically zero.
                format!(
                    "{} nm",
                    fmt_num(if r.best_focus_nm.abs() < 1e-6 {
                        0.0
                    } else {
                        r.best_focus_nm
                    })
                ),
            ),
            (
                "DOF at dose-to-size".to_string(),
                format!("{} nm", fmt_num(r.dof_nm)),
            ),
            (
                "Exposure latitude at best focus".to_string(),
                format!("{} %", fmt_num(r.el_pct)),
            ),
            (
                "Dose-to-size".to_string(),
                r.dose_to_size
                    .map_or("n/a".into(), |d| format!("{} mJ/cm\u{b2}", fmt_num(d))),
            ),
            (
                "DOF at 5 % EL".to_string(),
                r.dof_at_5pct
                    .map_or("n/a".into(), |x| format!("{} nm", fmt_num(x.dof_nm()))),
            ),
            (
                "Max DOF\u{d7}EL window".to_string(),
                r.max_area.map_or("none".into(), |x| {
                    format!(
                        "{} nm \u{d7} {} %",
                        fmt_num(x.dof_nm()),
                        fmt_num(x.exposure_latitude_pct())
                    )
                }),
            ),
            (
                "CD spec".to_string(),
                format!("{} \u{2013} {} nm", fmt_num(spec_lo), fmt_num(spec_hi)),
            ),
            (
                "Rayleigh DOF n\u{3bb}/(2NA\u{b2})".to_string(),
                format!("{} nm", fmt_num(r.rayleigh_dof_nm)),
            ),
        ];
        if let Some(t) = w.resist {
            rows.push((
                "Resist E_th (open-frame dose to clear)".to_string(),
                format!("{} mJ/cm\u{b2}", fmt_num(t.dose_to_clear_mj_cm2)),
            ));
        }
        kv_table(ui, "pw_summary", &rows);
        for msg in &r.warnings {
            warning_line(ui, msg);
        }
        let plot_h = 300.0;
        let n_doses = w.doses.len();
        ui.columns(2, |cols| {
            // Bossung curves.
            cols[0].label(RichText::new("Bossung curves: printed CD vs focus").strong());
            Plot::new("bossung")
                .height(plot_h)
                .x_axis_label("focus (nm)")
                .y_axis_label("CD (nm)")
                .legend(Legend::default())
                .show(&mut cols[0], |plot_ui| {
                    for (i, curve) in w.bossung_curves().iter().enumerate() {
                        let color = ordinal(i, n_doses, dark);
                        let name = format!("{} mJ/cm\u{b2}", fmt_num(curve[0].dose_mj_cm2));
                        for seg in finite_segments(curve.iter().map(|p| (p.focus_nm, p.cd_nm))) {
                            plot_ui.line(
                                Line::new(PlotPoints::from(seg))
                                    .color(color)
                                    .width(2.0)
                                    .name(&name),
                            );
                        }
                    }
                    plot_ui.hline(
                        HLine::new(w.cd_target_nm)
                            .color(muted(dark))
                            .name("target CD"),
                    );
                    for y in [spec_lo, spec_hi] {
                        plot_ui.hline(
                            HLine::new(y)
                                .color(muted(dark))
                                .style(LineStyle::Dashed { length: 8.0 })
                                .name("CD spec"),
                        );
                    }
                });
            // Exposure–defocus window.
            cols[1].label(RichText::new("Exposure\u{2013}defocus window: in-spec doses").strong());
            Plot::new("ed_window")
                .height(plot_h)
                .x_axis_label("focus (nm)")
                .y_axis_label("dose (mJ/cm\u{b2})")
                .legend(Legend::default())
                .show(&mut cols[1], |plot_ui| {
                    let mut order: Vec<usize> = (0..w.focuses.len()).collect();
                    order.sort_by(|&a, &b| w.focuses[a].total_cmp(&w.focuses[b]));
                    let lo = finite_segments(
                        order
                            .iter()
                            .map(|&j| (w.focuses[j], w.dose_limits[j].map_or(f64::NAN, |d| d.0))),
                    );
                    let hi = finite_segments(
                        order
                            .iter()
                            .map(|&j| (w.focuses[j], w.dose_limits[j].map_or(f64::NAN, |d| d.1))),
                    );
                    let fill = series(0, dark).gamma_multiply(0.35);
                    for (a, b) in lo.iter().zip(&hi) {
                        let mut poly = a.clone();
                        poly.extend(b.iter().rev());
                        plot_ui.polygon(
                            Polygon::new(PlotPoints::from(poly))
                                .fill_color(fill)
                                .stroke(egui::Stroke::new(1.5, series(0, dark)))
                                .name("in-spec region"),
                        );
                    }
                    if let Some(rect) = &r.max_area {
                        plot_ui.line(
                            Line::new(PlotPoints::from(rect_outline(rect)))
                                .color(series(1, dark))
                                .width(2.0)
                                .name("max DOF\u{d7}EL rectangle"),
                        );
                    }
                    if let Some(rect) = &r.dof_at_5pct {
                        plot_ui.line(
                            Line::new(PlotPoints::from(rect_outline(rect)))
                                .color(series(2, dark))
                                .width(2.0)
                                .name("DOF at 5 % EL"),
                        );
                    }
                });
        });
        ui.label(RichText::new("Exposure latitude vs depth of focus").strong());
        Plot::new("el_dof")
            .height(200.0)
            .x_axis_label("DOF (nm)")
            .y_axis_label("EL (%)")
            .include_y(0.0)
            .show(ui, |plot_ui| {
                let pts: Vec<[f64; 2]> = r.el_vs_dof.iter().map(|&(d, e)| [d, e]).collect();
                plot_ui.line(
                    Line::new(PlotPoints::from(pts))
                        .color(series(0, dark))
                        .width(2.0),
                );
            });
        egui::CollapsingHeader::new("CD matrix (nm; blank = does not print)")
            .default_open(false)
            .show(ui, |ui| {
                egui::ScrollArea::horizontal().show(ui, |ui| {
                    egui::Grid::new("cd_matrix").striped(true).show(ui, |ui| {
                        ui.label(RichText::new("dose \\ focus").strong());
                        for z in &w.focuses {
                            ui.label(RichText::new(fmt_num(*z)).strong());
                        }
                        ui.end_row();
                        for (i, d) in w.doses.iter().enumerate() {
                            ui.label(RichText::new(fmt_num(*d)).strong());
                            for j in 0..w.focuses.len() {
                                let cd = w.cd_matrix[[i, j]];
                                ui.label(if cd.is_finite() {
                                    fmt_num(cd)
                                } else {
                                    String::new()
                                });
                            }
                            ui.end_row();
                        }
                    });
                });
            });
    }

    // -----------------------------------------------------------------------
    // Volume viewer
    // -----------------------------------------------------------------------

    pub(crate) fn view_volume(&mut self, ui: &mut egui::Ui) {
        let route = self.source.route();
        ui.horizontal_wrapped(|ui| {
            ui.label("Dataset");
            egui::ComboBox::from_id_salt("dataset")
                .selected_text(self.viewer.dataset.label())
                .show_ui(ui, |ui| {
                    for d in DatasetKind::ALL {
                        if dataset_available(d, route) {
                            ui.selectable_value(&mut self.viewer.dataset, d, d.label());
                        }
                    }
                });
            ui.checkbox(&mut self.live_heavy, "live update");
            if !self.live_heavy && ui.button("Compute").clicked() {
                self.run_heavy_once = true;
            }
        });
        match self.viewer.dataset {
            DatasetKind::Latent | DatasetKind::Developed => self.volumetric_controls(ui),
            DatasetKind::TalbotCarpet => self.talbot_controls(ui),
            DatasetKind::LigaDose => note(ui, "LIGA exposure settings are in the left panel."),
        }
        ui.separator();

        // Pick the dataset and its extra rows.
        let (data, summary, notes): VolumeSelection<'_> = match self.viewer.dataset {
            DatasetKind::Latent | DatasetKind::Developed => match &self.vol_job.latest {
                None => (None, vec![], vec![]),
                Some(fin) => match &fin.result {
                    Err(e) => {
                        warning_line(ui, e);
                        return;
                    }
                    Ok(r) => {
                        let d = if self.viewer.dataset == DatasetKind::Developed {
                            r.developed.as_ref()
                        } else {
                            Some(&r.latent)
                        };
                        if d.is_none() {
                            warning_line(
                                ui,
                                "No development computed: choose fast marching or level set.",
                            );
                        }
                        (d, r.summary.clone(), r.notes.clone())
                    }
                },
            },
            DatasetKind::LigaDose => match &self.liga_job.latest {
                None => (None, vec![], vec![]),
                Some(fin) => match &fin.result {
                    Err(e) => {
                        warning_line(ui, e);
                        return;
                    }
                    Ok(r) => (Some(&r.volume), r.summary.clone(), vec![]),
                },
            },
            DatasetKind::TalbotCarpet => match &self.talbot_job.latest {
                None => (None, vec![], vec![]),
                Some(fin) => match &fin.result {
                    Err(e) => {
                        warning_line(ui, e);
                        return;
                    }
                    Ok(r) => (
                        Some(&r.carpet),
                        vec![
                            (
                                "Wavelength".into(),
                                format!("{} nm", fmt_num(r.wavelength_nm)),
                            ),
                            (
                                "Grating period".into(),
                                format!("{} nm", fmt_num(r.period_nm)),
                            ),
                            (
                                "Talbot length 2p\u{b2}/\u{3bb}".into(),
                                format!("{} nm", fmt_num(r.talbot_length_nm)),
                            ),
                            (
                                "Exact Talbot length".into(),
                                r.talbot_length_exact_nm
                                    .map_or("n/a".into(), |z| format!("{} nm", fmt_num(z))),
                            ),
                        ],
                        vec![
                            "Scalar thin-mask grating, coherent normal illumination, exact \
                                 angular-spectrum propagation (status: Simplified, see \
                                 docs/processes/talbot.md)."
                                .into(),
                        ],
                    ),
                },
            },
        };
        let fresh = match self.viewer.dataset {
            DatasetKind::Latent | DatasetKind::Developed => self
                .vol_job
                .latest
                .as_ref()
                .map(|f| (f.elapsed, self.vol_job.is_current(&self.vol_request()))),
            DatasetKind::LigaDose => self
                .liga_job
                .latest
                .as_ref()
                .map(|f| (f.elapsed, self.liga_job.is_current(&self.liga_request()))),
            DatasetKind::TalbotCarpet => self.talbot_job.latest.as_ref().map(|f| {
                (
                    f.elapsed,
                    self.talbot_job.is_current(&self.talbot_request()),
                )
            }),
        };
        if let Some((elapsed, current)) = fresh {
            freshness(ui, elapsed, current);
        }
        let Some(v) = data else {
            if !self.live_heavy {
                note(ui, "Press Compute to run this dataset.");
            } else {
                waiting(ui, "volume");
            }
            return;
        };
        // Share the dataset so `self` stays free for the textures.
        let v = Arc::clone(v);
        let shape_key = key_of((v.kind as u8, v.dims()));
        let shape_changed = ui.ctx().data_mut(|d| {
            let id = egui::Id::new("volume_shape");
            let prev: Option<u64> = d.get_temp(id);
            d.insert_temp(id, shape_key);
            prev != Some(shape_key)
        });
        self.viewer.conform(&v, shape_changed);
        self.draw_volume(ui, &v);
        ui.separator();
        kv_table(ui, "volume_summary", &summary);
        for n in &notes {
            note(ui, n);
        }
    }

    fn volumetric_controls(&mut self, ui: &mut egui::Ui) {
        let v = &mut self.vol;
        ui.horizontal_wrapped(|ui| {
            ui.add(
                egui::Slider::new(&mut v.resist_thickness_nm, 20.0..=2000.0)
                    .logarithmic(true)
                    .text("resist (nm)"),
            );
            ui.add(
                egui::Slider::new(&mut v.dose_mj_cm2, 1.0..=300.0)
                    .logarithmic(true)
                    .text("dose (mJ/cm\u{b2})"),
            );
            ui.add(egui::Slider::new(&mut v.nz, 4..=96).text("z slices"));
            ui.add(egui::Slider::new(&mut v.n_planes, 1..=32).text("focus planes"));
        });
        ui.horizontal_wrapped(|ui| {
            ui.label("Bake");
            ui.radio_value(&mut v.peb, PebChoice::None, "none");
            ui.radio_value(&mut v.peb, PebChoice::Gaussian, "Gaussian");
            ui.radio_value(&mut v.peb, PebChoice::Car, "CAR reaction\u{2013}diffusion");
            if v.peb == PebChoice::Gaussian {
                ui.add(egui::Slider::new(&mut v.peb_lateral_nm, 0.0..=50.0).text("\u{3c3}xy (nm)"));
                ui.add(egui::Slider::new(&mut v.peb_vertical_nm, 0.0..=50.0).text("\u{3c3}z (nm)"));
            }
        });
        ui.horizontal_wrapped(|ui| {
            ui.label("Develop");
            ui.radio_value(&mut v.develop, DevelopChoice::None, "latent only");
            ui.radio_value(&mut v.develop, DevelopChoice::FastMarching, "fast marching");
            ui.radio_value(&mut v.develop, DevelopChoice::LevelSet, "level set");
            if v.develop != DevelopChoice::None {
                ui.add(
                    egui::Slider::new(&mut v.dev_time_s, 1.0..=300.0)
                        .logarithmic(true)
                        .text("time (s)"),
                );
            }
        });
        note(
            ui,
            "Uses the projection scene of the left panel (source, optics, mask, grid, focus). \
             Large grids take seconds: release builds recommended.",
        );
        note(
            ui,
            "Resist = the core's illustrative default (Mack n = 3, low contrast) on bare Si: \
             the developed window is narrow (default 15 mJ/cm\u{b2}, 30 s). Without a bake \
             (or with \u{3c3}z below about 10 nm) the standing waves (period \u{3bb}/2n, \u{2248} 48 nm \
             at 157 nm) leave nodes that stall the dissolution front.",
        );
    }

    fn talbot_controls(&mut self, ui: &mut egui::Ui) {
        let t = &mut self.talbot;
        ui.horizontal_wrapped(|ui| {
            ui.radio_value(&mut t.phase_grating, false, "amplitude grating");
            ui.radio_value(&mut t.phase_grating, true, "\u{3c0} phase grating");
            ui.add(egui::Slider::new(&mut t.max_order, 1..=40).text("orders \u{b1}"));
            ui.add(
                egui::Slider::new(&mut t.z_max_talbot, 0.25..=6.0).text("z range (Talbot lengths)"),
            );
            ui.add(egui::Slider::new(&mut t.periods, 1..=6).text("periods across"));
            ui.add(egui::Slider::new(&mut t.nz, 32..=512).text("z rows"));
        });
        note(
            ui,
            "Grating period and line CD from the Mask section; wavelength from the source.",
        );
    }

    fn draw_volume(&mut self, ui: &mut egui::Ui, v: &VolumeData) {
        let dark = ui.visuals().dark_mode;
        let (nz, ny, _nx) = v.dims();
        let scale = v.display_scale_nm;
        let unit = v.display_unit;
        ui.horizontal_wrapped(|ui| {
            ui.label("Colormap");
            colormap_combo(ui, "volume_cmap", &mut self.viewer.colormap);
            ui.radio_value(
                &mut self.viewer.range_mode,
                RangeMode::Global,
                "global range",
            );
            ui.radio_value(
                &mut self.viewer.range_mode,
                RangeMode::PerSlice,
                "per-view range",
            );
        });
        let z_text = format!(
            "z slice ({} = {} {unit})",
            v.z_label,
            fmt_num(v.z_at(self.viewer.slice_k) / scale)
        );
        ui.add(egui::Slider::new(&mut self.viewer.slice_k, 0..=nz.saturating_sub(1)).text(z_text));
        if !v.is_xz_plane() {
            let y_text = format!(
                "x\u{2013}z section at y = {} {unit}",
                fmt_num(v.y_at(self.viewer.row_i) / scale)
            );
            ui.add(
                egui::Slider::new(&mut self.viewer.row_i, 0..=ny.saturating_sub(1)).text(y_text),
            );
        }
        let cmap = self.viewer.colormap;
        let label = if v.unit.is_empty() {
            v.quantity.clone()
        } else {
            format!("{} ({})", v.quantity, v.unit)
        };
        let to_disp = |r: (f64, f64)| (r.0 / scale, r.1 / scale);
        let width = ui.available_width();
        let pane = ((width - 110.0) / 2.0).clamp(220.0, 640.0);
        let mut save_xy = false;
        let mut save_xz = false;
        let xz = v.xz_slice(self.viewer.row_i);
        let xz_range = self.viewer.color_range(v, &xz);
        let xy = v.xy_slice(self.viewer.slice_k);
        let xy_range = self.viewer.color_range(v, &xy);
        let gen = self.generation;
        let mut hover_text = None;
        ui.horizontal(|ui| {
            // Lateral slice (or the profile at depth z for x–z planes).
            ui.vertical(|ui| {
                ui.set_width(pane);
                if v.is_xz_plane() {
                    ui.label(
                        RichText::new(format!(
                            "Profile at z = {} {unit}",
                            fmt_num(v.z_at(self.viewer.slice_k) / scale)
                        ))
                        .strong(),
                    );
                    let pts: Vec<[f64; 2]> = (0..xy.ncols())
                        .map(|j| [v.x_at(j) / scale, xy[[0, j]]])
                        .collect();
                    Plot::new("volume_profile")
                        .height(pane)
                        .x_axis_label(format!("x ({unit})"))
                        .y_axis_label(label.clone())
                        .include_y(0.0)
                        .show(ui, |plot_ui| {
                            plot_ui.line(
                                Line::new(PlotPoints::from(pts))
                                    .color(series(0, dark))
                                    .width(2.0),
                            );
                        });
                } else {
                    ui.label(
                        RichText::new(format!(
                            "x\u{2013}y slice at depth {} {unit}",
                            fmt_num(v.z_at(self.viewer.slice_k) / scale)
                        ))
                        .strong(),
                    );
                    let hover = heatmap(
                        ui,
                        &mut self.tex_xy,
                        &Heatmap {
                            id: "volume_xy",
                            data: &xy,
                            x_range: to_disp(v.x_nm),
                            v_range: to_disp(v.y_nm),
                            depth_down: false,
                            colormap: cmap,
                            value_range: xy_range,
                            x_label: format!("x ({unit})"),
                            v_label: format!("y ({unit})"),
                            equal_aspect: true,
                            key: key_of((
                                "xy",
                                gen,
                                v.kind as u8,
                                self.viewer.slice_k,
                                cmap as u8,
                                xy_range.0.to_bits(),
                                xy_range.1.to_bits(),
                                dark,
                            )),
                            height: pane,
                        },
                    );
                    if let Some(h) = hover {
                        hover_text = Some(format!(
                            "x = {} {unit}, y = {} {unit}, z = {} {unit}:  {} {}",
                            fmt_num(h.x),
                            fmt_num(h.v),
                            fmt_num(v.z_at(self.viewer.slice_k) / scale),
                            fmt_num(h.value),
                            v.unit
                        ));
                    }
                    save_xy = ui.button("Save x\u{2013}y PNG").clicked();
                }
            });
            // x–z cross-section, depth downward.
            ui.vertical(|ui| {
                ui.set_width(pane);
                ui.label(
                    RichText::new(format!(
                        "x\u{2013}z section at y = {} {unit}",
                        fmt_num(v.y_at(self.viewer.row_i) / scale)
                    ))
                    .strong(),
                );
                let hover = heatmap(
                    ui,
                    &mut self.tex_xz,
                    &Heatmap {
                        id: "volume_xz",
                        data: &xz,
                        x_range: to_disp(v.x_nm),
                        v_range: to_disp(v.z_nm),
                        depth_down: true,
                        colormap: cmap,
                        value_range: xz_range,
                        x_label: format!("x ({unit})"),
                        v_label: format!("{} ({unit})", v.z_label),
                        equal_aspect: false,
                        key: key_of((
                            "xz",
                            gen,
                            v.kind as u8,
                            self.viewer.row_i,
                            cmap as u8,
                            xz_range.0.to_bits(),
                            xz_range.1.to_bits(),
                            dark,
                        )),
                        height: pane,
                    },
                );
                if let Some(h) = hover {
                    hover_text = Some(format!(
                        "x = {} {unit}, z = {} {unit}:  {} {}",
                        fmt_num(h.x),
                        fmt_num(h.v),
                        fmt_num(h.value),
                        v.unit
                    ));
                }
                save_xz = ui.button("Save x\u{2013}z PNG").clicked();
            });
            colorbar(ui, cmap, xz_range, &label, pane * 0.7);
        });
        match hover_text {
            Some(t) => {
                ui.label(t);
            }
            None => note(ui, "Hover a map for the value under the cursor."),
        }
        if save_xy {
            let path = self.export_path("volume-xy");
            self.status = Some(save_heatmap_png(&path, &xy, cmap, xy_range, dark, true));
        }
        if save_xz {
            let path = self.export_path("volume-xz");
            self.status = Some(save_heatmap_png(&path, &xz, cmap, xz_range, dark, false));
        }
    }

    // -----------------------------------------------------------------------
    // LIGA
    // -----------------------------------------------------------------------

    pub(crate) fn view_liga(&mut self, ui: &mut egui::Ui) {
        let dark = ui.visuals().dark_mode;
        if self.source.route() != Route::Liga {
            note(
                ui,
                "Select an X-ray tube, a betatron or a bending magnet for the LIGA view.",
            );
            return;
        }
        ui.horizontal(|ui| {
            ui.checkbox(&mut self.live_heavy, "live update");
            if !self.live_heavy && ui.button("Compute").clicked() {
                self.run_heavy_once = true;
            }
            if ui.button("Open the 3D dose in the volume viewer").clicked() {
                self.viewer.dataset = DatasetKind::LigaDose;
                self.tab = Tab::Volume;
            }
        });
        let Some(fin) = &self.liga_job.latest else {
            waiting(ui, "LIGA exposure");
            return;
        };
        let r = match &fin.result {
            Ok(r) => r,
            Err(e) => {
                warning_line(ui, e);
                return;
            }
        };
        freshness(
            ui,
            fin.elapsed,
            self.liga_job.is_current(&self.liga_request()),
        );
        ui.label(RichText::new("Absorbed dose vs depth (open areas)").strong());
        let pts: Vec<[f64; 2]> = r
            .z_um
            .iter()
            .zip(&r.dose_kj_cm3)
            .map(|(&z, &d)| [z, d])
            .collect();
        Plot::new("liga_depth")
            .height(300.0)
            .x_axis_label("depth below the PMMA top (\u{b5}m)")
            .y_axis_label("dose (kJ/cm\u{b3})")
            .include_y(0.0)
            .legend(Legend::default())
            .show(ui, |plot_ui| {
                plot_ui.line(
                    Line::new(PlotPoints::from(pts))
                        .color(series(0, dark))
                        .width(2.0)
                        .name("absorbed dose"),
                );
                plot_ui.hline(
                    HLine::new(r.target_bottom_dose)
                        .color(muted(dark))
                        .style(LineStyle::Dashed { length: 8.0 })
                        .name("bottom (clearing) dose"),
                );
                plot_ui.hline(
                    HLine::new(r.damage_dose)
                        .color(widgets::critical_color())
                        .style(LineStyle::Dashed { length: 8.0 })
                        .name("damage ceiling"),
                );
            });
        kv_table(ui, "liga_summary", &r.summary);
        for w in &r.warnings {
            warning_line(ui, w);
        }
        egui::CollapsingHeader::new("Data table")
            .default_open(false)
            .show(ui, |ui| {
                egui::ScrollArea::vertical()
                    .max_height(220.0)
                    .show(ui, |ui| {
                        egui::Grid::new("liga_table").striped(true).show(ui, |ui| {
                            ui.label(RichText::new("depth (\u{b5}m)").strong());
                            ui.label(RichText::new("dose (kJ/cm\u{b3})").strong());
                            ui.end_row();
                            for (z, d) in r.z_um.iter().zip(&r.dose_kj_cm3) {
                                ui.label(fmt_num(*z));
                                ui.label(fmt_num(*d));
                                ui.end_row();
                            }
                        });
                    });
            });
    }

    // -----------------------------------------------------------------------
    // Source
    // -----------------------------------------------------------------------

    pub(crate) fn view_source(&mut self, ui: &mut egui::Ui) {
        let dark = ui.visuals().dark_mode;
        let emoji_ok = self.emoji_ok.unwrap_or(false);
        let status = self.source.family.status();
        ui.heading(self.source.family.label());
        note(
            ui,
            &format!(
                "TOML / Python type tag: \"{}\"",
                self.source.family.kind_label()
            ),
        );
        ui.horizontal_wrapped(|ui| {
            for b in status.badges {
                badge_chip(ui, *b, emoji_ok);
            }
            if let Some(b) = self.source.preset_badge() {
                ui.label("preset:");
                badge_chip(ui, b, emoji_ok);
            }
        });
        note(ui, status.summary);
        let src = match &self.summary.built {
            Ok(s) => s.clone(),
            Err(e) => {
                warning_line(ui, e);
                return;
            }
        };
        ui.label(format!(
            "\u{3bb} = {} nm, bandwidth {} pm, average power {}",
            fmt_num(src.wavelength_nm()),
            fmt_num(src.bandwidth_pm()),
            src.average_power_w()
                .map_or("not modeled".into(), |p| format!("{} W", fmt_num(p)))
        ));
        egui::CollapsingHeader::new(RichText::new("Derived quantities").strong())
            .default_open(true)
            .show(ui, |ui| self.derived_table(ui, "derived_full", true));

        if let Some((pts, label)) = xray_source_spectrum(&src) {
            ui.label(RichText::new("X-ray photon spectrum").strong());
            let line: Vec<[f64; 2]> = pts.iter().map(|&(e, v)| [e, v]).collect();
            Plot::new("xray_spectrum")
                .height(260.0)
                .x_axis_label("photon energy (keV)")
                .y_axis_label(label)
                .include_y(0.0)
                .show(ui, |plot_ui| {
                    plot_ui.line(
                        Line::new(PlotPoints::from(line))
                            .color(series(0, dark))
                            .width(1.5),
                    );
                });
            return;
        }
        let weights = src.spectral_weights();
        let plot_h = 260.0;
        let aerial = self.aerial.result();
        ui.columns(2, |cols| {
            cols[0].label(RichText::new("Spectral samples").strong());
            Plot::new("spectral_samples")
                .height(plot_h)
                .x_axis_label("wavelength (nm)")
                .y_axis_label("weight")
                .include_y(0.0)
                .show(&mut cols[0], |plot_ui| {
                    let pts: Vec<[f64; 2]> = weights.iter().map(|&(l, w)| [l, w]).collect();
                    plot_ui.points(
                        Points::new(PlotPoints::from(pts))
                            .stems(0.0)
                            .radius(4.0)
                            .color(series(0, dark)),
                    );
                });
            cols[1].label(RichText::new("Pupil fill (sampled source points)").strong());
            match aerial {
                Some(r) => {
                    Plot::new("pupil_fill")
                        .height(plot_h)
                        .data_aspect(1.0)
                        .x_axis_label("\u{3c3}x")
                        .y_axis_label("\u{3c3}y")
                        .include_x(-1.05)
                        .include_x(1.05)
                        .include_y(-1.05)
                        .include_y(1.05)
                        .show(&mut cols[1], |plot_ui| {
                            let circle = |rad: f64| -> Vec<[f64; 2]> {
                                (0..=128)
                                    .map(|i| {
                                        let t = i as f64 / 128.0 * std::f64::consts::TAU;
                                        [rad * t.cos(), rad * t.sin()]
                                    })
                                    .collect()
                            };
                            plot_ui.line(
                                Line::new(PlotPoints::from(circle(1.0)))
                                    .color(muted(dark))
                                    .name("pupil edge (NA)"),
                            );
                            if let Some(o) = r.central_obscuration.filter(|o| *o > 0.0) {
                                plot_ui.line(
                                    Line::new(PlotPoints::from(circle(o)))
                                        .color(widgets::critical_color())
                                        .name("central obscuration"),
                                );
                            }
                            let w_max =
                                r.source_points.iter().map(|p| p.weight).fold(0.0, f64::max);
                            let pts: Vec<[f64; 2]> = r
                                .source_points
                                .iter()
                                .filter(|p| p.weight > 1e-3 * w_max)
                                .map(|p| [p.sx, p.sy])
                                .collect();
                            plot_ui.points(
                                Points::new(PlotPoints::from(pts))
                                    .radius(1.5)
                                    .color(series(0, dark)),
                            );
                        });
                    note(
                        &mut cols[1],
                        &format!(
                            "{} \u{2014} {} source points (weights > 1e-3 of the peak shown)",
                            r.illumination,
                            r.source_points.len()
                        ),
                    );
                }
                None => waiting(&mut cols[1], "source sampling"),
            }
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn segments_split_at_missing_values() {
        let segs = finite_segments(vec![
            (0.0, 1.0),
            (1.0, f64::NAN),
            (2.0, 2.0),
            (3.0, 3.0),
            (4.0, f64::INFINITY),
        ]);
        assert_eq!(segs, vec![vec![[0.0, 1.0]], vec![[2.0, 2.0], [3.0, 3.0]]]);
        assert!(finite_segments(vec![(0.0, f64::NAN)]).is_empty());
    }

    #[test]
    fn rectangle_outline_is_closed() {
        let r = highuvlith_core::process::ProcessRectangle {
            focus_min_nm: -50.0,
            focus_max_nm: 50.0,
            dose_min_mj_cm2: 28.0,
            dose_max_mj_cm2: 32.0,
        };
        let o = rect_outline(&r);
        assert_eq!(o.len(), 5);
        assert_eq!(o[0], o[4]);
    }
}
