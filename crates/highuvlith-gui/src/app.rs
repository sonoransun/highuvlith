//! The eframe application: panel state, background jobs, tab bar, status
//! bar, PNG export and window screenshots. The left-panel controls live in
//! `panels`, the central views in `views`.

use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Instant, SystemTime, UNIX_EPOCH};

use eframe::egui::{self, RichText};
use highuvlith_core::source::{DerivedQuantity, LithographySource, SourceKind};

use crate::colormap::Colormap;
use crate::compute::{
    self, AerialRequest, AerialResult, EngineCache, LigaParams, LigaRequest, LigaResult, PwParams,
    PwRequest, PwResult, Resolved, Scene, TalbotParams, TalbotRequest, TalbotResult,
    VolumetricParams, VolumetricRequest, VolumetricResult,
};
use crate::imaging::{GridParams, ImagingParams, MaskParams, OpticsKind, OpticsParams};
use crate::jobs::Job;
use crate::sources::{Route, SourceParams};
use crate::ui_state::{conform_dataset, conform_tab, needs, Need, Tab};
use crate::volume::{DatasetKind, ViewerState};
use crate::widgets::TextureSlot;

/// Start-up options (from the command line).
#[derive(Default)]
pub struct StartOptions {
    /// Source preset to load.
    pub preset: Option<crate::sources::PresetId>,
    /// Optics preset to load.
    pub optics: Option<crate::imaging::OpticsPreset>,
    /// View to open.
    pub tab: Option<Tab>,
    /// Mask CD / pitch overrides (nm).
    pub cd_nm: Option<f64>,
    pub pitch_nm: Option<f64>,
    /// Volume dataset to open.
    pub dataset: Option<DatasetKind>,
    /// Save a screenshot of the window to this path once the view is
    /// computed, then exit.
    pub screenshot: Option<PathBuf>,
}

/// UI-thread cache of the built source (cheap, no imaging).
pub struct SourceSummary {
    key: Option<(SourceParams, Option<usize>)>,
    pub built: Result<SourceKind, String>,
    pub derived: Vec<DerivedQuantity>,
}

impl SourceSummary {
    fn refresh(&mut self, params: &SourceParams, samples: Option<usize>) {
        let key = (params.clone(), samples);
        if self.key.as_ref() == Some(&key) {
            return;
        }
        self.built = params.build(samples);
        self.derived = self
            .built
            .as_ref()
            .map(|s| s.derived_quantities())
            .unwrap_or_default();
        self.key = Some(key);
    }

    /// Wavelength of the built source.
    pub fn wavelength_nm(&self) -> Option<f64> {
        self.built.as_ref().ok().map(|s| s.wavelength_nm())
    }
}

/// UI-thread cache of the resolved scene (optics label, grid choice).
pub struct SceneCache {
    key: Option<Scene>,
    pub resolved: Result<Resolved, String>,
}

impl SceneCache {
    fn refresh(&mut self, scene: &Scene) {
        if self.key.as_ref() == Some(scene) {
            return;
        }
        self.resolved = compute::resolve(scene);
        self.key = Some(scene.clone());
    }
}

/// A pending window screenshot.
pub struct ScreenshotPlan {
    pub path: PathBuf,
    /// Exit after saving (command-line mode).
    pub exit_after: bool,
    /// Frames to wait once the view is computed (lets the layout settle).
    settle: u32,
    requested: bool,
    started: Instant,
}

impl ScreenshotPlan {
    fn new(path: PathBuf, exit_after: bool) -> Self {
        Self {
            path,
            exit_after,
            settle: 8,
            requested: false,
            started: Instant::now(),
        }
    }
}

/// The application.
pub struct LithApp {
    pub(crate) source: SourceParams,
    pub(crate) optics: OpticsParams,
    pub(crate) imaging: ImagingParams,
    pub(crate) mask: MaskParams,
    pub(crate) grid: GridParams,
    pub(crate) focus_nm: f64,
    /// Printed-edge intensity threshold for the CD / NILS readouts.
    pub(crate) threshold: f64,
    pub(crate) pw: PwParams,
    pub(crate) vol: VolumetricParams,
    pub(crate) liga: LigaParams,
    pub(crate) talbot: TalbotParams,
    pub(crate) viewer: ViewerState,
    pub(crate) tab: Tab,
    pub(crate) aerial_colormap: Colormap,
    /// Recompute the heavy 3D views (volumetric, LIGA, Talbot) on every change.
    pub(crate) live_heavy: bool,
    /// One-shot request from a "Compute" button.
    pub(crate) run_heavy_once: bool,

    pub(crate) summary: SourceSummary,
    pub(crate) scene_cache: SceneCache,

    pub(crate) aerial: Job<AerialRequest, AerialResult>,
    pub(crate) pw_job: Job<PwRequest, PwResult>,
    pub(crate) vol_job: Job<VolumetricRequest, VolumetricResult>,
    pub(crate) liga_job: Job<LigaRequest, LigaResult>,
    pub(crate) talbot_job: Job<TalbotRequest, TalbotResult>,
    /// Bumped whenever a result arrives (texture cache keys).
    pub(crate) generation: u64,

    pub(crate) tex_aerial: TextureSlot,
    pub(crate) tex_xy: TextureSlot,
    pub(crate) tex_xz: TextureSlot,
    /// Whether the UI font has the badge emoji (checked once).
    pub(crate) emoji_ok: Option<bool>,
    pub(crate) export_dir: String,
    pub(crate) status: Option<String>,
    pub(crate) screenshot: Option<ScreenshotPlan>,
    last_route: Route,
    last_optics_kind: Option<OpticsKind>,
}

impl LithApp {
    /// New application state (with optional start-up presets).
    pub fn new(opts: StartOptions) -> Self {
        let cache = Arc::new(EngineCache::default());
        let mut app = Self {
            source: SourceParams::default(),
            optics: OpticsParams::default(),
            imaging: ImagingParams::default(),
            mask: MaskParams::default(),
            grid: GridParams::default(),
            focus_nm: 0.0,
            threshold: 0.3,
            pw: PwParams::default(),
            vol: VolumetricParams::default(),
            liga: LigaParams::default(),
            talbot: TalbotParams::default(),
            viewer: ViewerState::default(),
            tab: Tab::Aerial,
            aerial_colormap: Colormap::Inferno,
            live_heavy: true,
            run_heavy_once: false,
            summary: SourceSummary {
                key: None,
                built: Err("not built".into()),
                derived: Vec::new(),
            },
            scene_cache: SceneCache {
                key: None,
                resolved: Err("not resolved".into()),
            },
            aerial: {
                let c = Arc::clone(&cache);
                Job::new(move |r| compute::run_aerial(&c, r))
            },
            pw_job: {
                let c = Arc::clone(&cache);
                Job::new(move |r| compute::run_process_window(&c, r))
            },
            vol_job: {
                let c = Arc::clone(&cache);
                Job::new(move |r| compute::run_volumetric(&c, r))
            },
            liga_job: Job::new(compute::run_liga),
            talbot_job: Job::new(compute::run_talbot),
            generation: 0,
            tex_aerial: TextureSlot::default(),
            tex_xy: TextureSlot::default(),
            tex_xz: TextureSlot::default(),
            emoji_ok: None,
            export_dir: std::env::current_dir()
                .map(|p| p.display().to_string())
                .unwrap_or_else(|_| ".".into()),
            status: None,
            screenshot: None,
            last_route: Route::Projection,
            last_optics_kind: None,
        };
        if let Some(id) = opts.preset {
            if let Err(e) = app.source.apply_preset(id) {
                app.status = Some(format!("preset failed: {e}"));
            }
        }
        if let Some(p) = opts.optics {
            app.optics.apply_preset(p);
            // A preset fixes the kind; keep its NA.
            app.last_optics_kind = Some(app.optics.kind);
        }
        if let Some(cd) = opts.cd_nm {
            app.mask.cd_nm = cd;
        }
        if let Some(pitch) = opts.pitch_nm {
            app.mask.pitch_nm = pitch;
        }
        if let Some(tab) = opts.tab {
            app.tab = tab;
        }
        if let Some(d) = opts.dataset {
            app.viewer.dataset = d;
        }
        if let Some(path) = opts.screenshot {
            app.screenshot = Some(ScreenshotPlan::new(path, true));
        }
        app.sync_route();
        app
    }

    /// The projection-imaging scene of the panels.
    pub(crate) fn scene(&self) -> Scene {
        Scene {
            source: self.source.clone(),
            optics: self.optics.clone(),
            imaging: self.imaging.clone(),
            mask: self.mask.clone(),
            grid: self.grid.clone(),
        }
    }

    pub(crate) fn aerial_request(&self) -> AerialRequest {
        AerialRequest {
            scene: self.scene(),
            focus_nm: self.focus_nm,
            threshold: self.threshold,
        }
    }

    pub(crate) fn pw_request(&self) -> PwRequest {
        PwRequest {
            scene: self.scene(),
            pw: self.pw.clone(),
        }
    }

    pub(crate) fn vol_request(&self) -> VolumetricRequest {
        VolumetricRequest {
            scene: self.scene(),
            focus_nm: self.focus_nm,
            vol: self.vol.clone(),
        }
    }

    pub(crate) fn liga_request(&self) -> LigaRequest {
        LigaRequest {
            source: self.source.clone(),
            liga: self.liga.clone(),
        }
    }

    pub(crate) fn talbot_request(&self) -> TalbotRequest {
        TalbotRequest {
            source: self.source.clone(),
            mask: self.mask.clone(),
            talbot: self.talbot.clone(),
        }
    }

    /// Keep the view and the optics consistent with the selected source.
    fn sync_route(&mut self) {
        self.summary
            .refresh(&self.source, self.imaging.spectral_samples);
        let route = self.source.route();
        if route != self.last_route {
            self.tab = conform_tab(self.tab, route);
            self.last_route = route;
        }
        self.viewer.dataset = conform_dataset(self.viewer.dataset, route);
        if let Some(lambda) = self.summary.wavelength_nm() {
            let kind = self.optics.kind.resolve(lambda);
            if self.last_optics_kind.is_some_and(|k| k != kind) {
                // E.g. VUV → EUV under "Auto": start from the new kind's NA.
                self.optics.na = kind.default_na();
            }
            self.last_optics_kind = Some(kind);
            self.optics.conform_to(lambda);
        }
        if route == Route::Projection {
            self.scene_cache.refresh(&self.scene());
        }
    }

    /// Record the current optics kind as seen (no automatic NA reset), e.g.
    /// after a preset set both the kind and its NA.
    pub(crate) fn mark_optics_kind_current(&mut self) {
        if let Some(lambda) = self.summary.wavelength_nm() {
            self.last_optics_kind = Some(self.optics.kind.resolve(lambda));
        }
    }

    /// Submit the visible view's computations and collect results.
    fn run_jobs(&mut self, ctx: &egui::Context) {
        let route = self.source.route();
        let heavy = self.live_heavy || self.run_heavy_once;
        for need in needs(self.tab, self.viewer.dataset, route) {
            match need {
                Need::Aerial => {
                    let req = self.aerial_request();
                    self.aerial.submit(req);
                }
                Need::ProcessWindow => {
                    let req = self.pw_request();
                    self.pw_job.submit(req);
                }
                Need::Volumetric if heavy => {
                    let req = self.vol_request();
                    self.vol_job.submit(req);
                }
                Need::Liga if heavy => {
                    let req = self.liga_request();
                    self.liga_job.submit(req);
                }
                Need::Talbot if heavy => {
                    let req = self.talbot_request();
                    self.talbot_job.submit(req);
                }
                _ => {}
            }
        }
        self.run_heavy_once = false;
        let notify = |ctx: &egui::Context| {
            let ctx = ctx.clone();
            move || ctx.request_repaint()
        };
        let mut arrived = false;
        arrived |= self.aerial.poll(notify(ctx));
        arrived |= self.pw_job.poll(notify(ctx));
        arrived |= self.vol_job.poll(notify(ctx));
        arrived |= self.liga_job.poll(notify(ctx));
        arrived |= self.talbot_job.poll(notify(ctx));
        if arrived {
            self.generation += 1;
        }
    }

    /// Whether any computation the visible view needs is still running.
    pub(crate) fn view_busy(&self) -> bool {
        needs(self.tab, self.viewer.dataset, self.source.route())
            .into_iter()
            .any(|n| match n {
                Need::Aerial => self.aerial.is_busy(),
                Need::ProcessWindow => self.pw_job.is_busy(),
                Need::Volumetric => self.vol_job.is_busy(),
                Need::Liga => self.liga_job.is_busy(),
                Need::Talbot => self.talbot_job.is_busy(),
            })
    }

    /// Whether the visible view has (any) result to show.
    fn view_has_result(&self) -> bool {
        needs(self.tab, self.viewer.dataset, self.source.route())
            .into_iter()
            .all(|n| match n {
                Need::Aerial => self.aerial.latest.is_some(),
                Need::ProcessWindow => self.pw_job.latest.is_some(),
                Need::Volumetric => self.vol_job.latest.is_some(),
                Need::Liga => self.liga_job.latest.is_some(),
                Need::Talbot => self.talbot_job.latest.is_some(),
            })
    }

    /// Directory for exports.
    pub(crate) fn export_path(&self, stem: &str) -> PathBuf {
        let secs = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
        PathBuf::from(&self.export_dir).join(format!("highuvlith-{stem}-{secs}.png"))
    }

    /// Ask the renderer for a window screenshot (saved by `handle_screenshot`).
    pub(crate) fn request_window_screenshot(&mut self) {
        let path = self.export_path("window");
        self.screenshot = Some(ScreenshotPlan::new(path, false));
    }

    fn handle_screenshot(&mut self, ctx: &egui::Context) {
        if self.screenshot.is_none() {
            return;
        }
        let view_ready = self.view_has_result() && !self.view_busy();
        let Some(plan) = &mut self.screenshot else {
            return;
        };
        // Deliver a captured frame.
        let image = ctx.input(|i| {
            i.raw.events.iter().find_map(|e| match e {
                egui::Event::Screenshot { image, .. } => Some(Arc::clone(image)),
                _ => None,
            })
        });
        if let Some(image) = image {
            let [w, h] = image.size;
            let rgba: Vec<u8> = image.pixels.iter().flat_map(|c| c.to_array()).collect();
            let msg = match crate::png::write_rgba(&plan.path, w as u32, h as u32, &rgba) {
                Ok(()) => format!("saved window screenshot {}", plan.path.display()),
                Err(e) => format!("screenshot failed: {e}"),
            };
            eprintln!("{msg}");
            let exit = plan.exit_after;
            self.status = Some(msg);
            self.screenshot = None;
            if exit {
                ctx.send_viewport_cmd(egui::ViewportCommand::Close);
            }
            return;
        }
        if plan.requested {
            ctx.request_repaint();
            return;
        }
        let ready = !plan.exit_after || view_ready;
        let timed_out = plan.started.elapsed().as_secs() > 300;
        if ready || timed_out {
            if plan.settle > 0 {
                plan.settle -= 1;
            } else {
                plan.requested = true;
                ctx.send_viewport_cmd(egui::ViewportCommand::Screenshot(egui::UserData::default()));
            }
        }
        ctx.request_repaint();
    }

    fn draw_tab_bar(&mut self, ui: &mut egui::Ui) {
        let route = self.source.route();
        ui.horizontal_wrapped(|ui| {
            for tab in Tab::ALL {
                let enabled = tab.available(route);
                let resp = ui.add_enabled(
                    enabled,
                    egui::SelectableLabel::new(self.tab == tab, tab.label()),
                );
                let resp = if enabled {
                    resp
                } else {
                    resp.on_disabled_hover_text(match route {
                        Route::Liga => {
                            "Broadband X-ray source: no projection imaging (see the LIGA view)"
                        }
                        Route::Projection => "Select an X-ray tube, betatron or bending magnet",
                    })
                };
                if resp.clicked() {
                    self.tab = tab;
                }
            }
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui
                    .button("Screenshot")
                    .on_hover_text("Save a PNG of the whole window to the export folder")
                    .clicked()
                {
                    self.request_window_screenshot();
                }
                if self.view_busy() {
                    ui.spinner();
                    ui.label(RichText::new("computing\u{2026}").weak());
                }
            });
        });
    }

    fn draw_status(&self, ui: &mut egui::Ui) {
        ui.horizontal_wrapped(|ui| {
            match self.source.route() {
                Route::Projection => {
                    if let Some(r) = self.aerial.result() {
                        ui.label(format!(
                            "\u{3bb} {:.3} nm \u{b7} NA {:.3} \u{b7} {}\u{b2} px of {:.3} nm, \
                             field {:.1} nm \u{b7} {} kernels ({:.4} of the TCC) \u{b7} engine \
                             {:.0} ms{} \u{b7} image {:.0} ms \u{b7} contrast {:.4}",
                            r.wavelength_nm,
                            r.na,
                            r.grid.size,
                            r.grid.pixel_nm,
                            r.grid.field_nm,
                            r.num_kernels,
                            r.captured_energy,
                            r.engine_ms,
                            if r.engine_reused { " (cached)" } else { "" },
                            r.image_ms,
                            r.contrast
                        ));
                    }
                }
                Route::Liga => {
                    ui.label(format!(
                        "{} \u{2014} LIGA / proximity source",
                        self.source.family.label()
                    ));
                }
            }
            if let Some(msg) = &self.status {
                ui.separator();
                ui.label(RichText::new(msg).weak());
            }
        });
    }
}

impl eframe::App for LithApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        if self.emoji_ok.is_none() {
            let probe = format!(
                "{}{}{}",
                crate::sources::Badge::Implemented.emoji(),
                crate::sources::Badge::Simplified.emoji(),
                crate::sources::Badge::Theoretical.emoji()
            );
            self.emoji_ok =
                Some(ctx.fonts(|f| f.has_glyphs(&egui::FontId::proportional(14.0), &probe)));
        }
        self.sync_route();
        self.run_jobs(ctx);

        egui::TopBottomPanel::bottom("status_panel").show(ctx, |ui| {
            self.draw_status(ui);
        });
        egui::SidePanel::left("params_panel")
            .resizable(true)
            .default_width(340.0)
            .min_width(280.0)
            .show(ctx, |ui| {
                egui::ScrollArea::vertical()
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        self.draw_params(ui);
                    });
            });
        egui::CentralPanel::default().show(ctx, |ui| {
            self.draw_tab_bar(ui);
            ui.separator();
            egui::ScrollArea::vertical()
                .auto_shrink([false, false])
                .show(ui, |ui| match self.tab {
                    Tab::Aerial => self.view_aerial(ui),
                    Tab::CrossSection => self.view_cross_section(ui),
                    Tab::ProcessWindow => self.view_process_window(ui),
                    Tab::Volume => self.view_volume(ui),
                    Tab::Liga => self.view_liga(ui),
                    Tab::Source => self.view_source(ui),
                });
        });
        self.handle_screenshot(ctx);
    }
}
