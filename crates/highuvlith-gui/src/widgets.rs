//! Shared drawing helpers: palette colours, honesty-badge chips, the
//! heatmap view (axes, colorbar, hover readout), key–value tables and number
//! formatting.
//!
//! Colours follow the docs' data-viz palette: categorical series in a fixed
//! slot order with light/dark steps, a one-hue ordinal ramp for ordered
//! series (doses), reserved status colours paired with a label, and text in
//! ink colours rather than series colours.

use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

use eframe::egui::{self, Color32, RichText};
use egui_plot::{Plot, PlotImage, PlotPoint};
use ndarray::Array2;

use crate::colormap::{normalize, Colormap};
use crate::sources::Badge;

/// Categorical series colour, slot `i` (0-based, fixed order, never cycled
/// past 8: callers fold extra series instead).
pub fn series(i: usize, dark: bool) -> Color32 {
    const LIGHT: [u32; 8] = [
        0x2a78d6, 0xeb6834, 0x1baf7a, 0xeda100, 0xe87ba4, 0x008300, 0x4a3aa7, 0xe34948,
    ];
    const DARK: [u32; 8] = [
        0x3987e5, 0xd95926, 0x199e70, 0xc98500, 0xd55181, 0x008300, 0x9085e9, 0xe66767,
    ];
    hex(if dark {
        DARK[i.min(7)]
    } else {
        LIGHT[i.min(7)]
    })
}

/// One-hue ordinal ramp for `n` ordered series (e.g. doses), position `i`.
/// Light theme: blue steps 250 → 700; dark theme: 600 → 100 (the end next
/// to the surface still clears 2:1 contrast).
pub fn ordinal(i: usize, n: usize, dark: bool) -> Color32 {
    let t = if n <= 1 {
        0.5
    } else {
        i as f64 / (n - 1) as f64
    };
    // Steps 100 … 700 of the sequential blue ramp map to t' ∈ [0, 1].
    let (lo, hi) = if dark {
        (10.0 / 12.0, 0.0)
    } else {
        (3.0 / 12.0, 1.0)
    };
    let rgb = Colormap::Blues.rgb(lo + (hi - lo) * t, false);
    Color32::from_rgb(rgb[0], rgb[1], rgb[2])
}

/// Muted ink for axes, reference lines and secondary text.
pub fn muted(dark: bool) -> Color32 {
    let _ = dark;
    hex(0x898781)
}

/// Status colour of an honesty badge (always shown with its label).
pub fn badge_color(badge: Badge) -> Color32 {
    match badge {
        Badge::Implemented => hex(0x0ca30c),
        Badge::Simplified => hex(0xfab219),
        Badge::Theoretical => hex(0xec835a),
    }
}

/// Status colour for warnings (paired with a ⚠ label).
pub fn warning_color() -> Color32 {
    hex(0xfab219)
}

/// Status colour for errors / limits that mean "bad" (paired with a label).
pub fn critical_color() -> Color32 {
    hex(0xd03b3b)
}

fn hex(v: u32) -> Color32 {
    Color32::from_rgb((v >> 16) as u8, (v >> 8) as u8, v as u8)
}

/// A badge chip: coloured dot + "✅ Implemented" (emoji only when the font
/// has the glyph).
pub fn badge_chip(ui: &mut egui::Ui, badge: Badge, emoji_ok: bool) {
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 4.0;
        let (rect, _) = ui.allocate_exact_size(egui::vec2(10.0, 10.0), egui::Sense::hover());
        ui.painter()
            .circle_filled(rect.center(), 4.5, badge_color(badge));
        let text = if emoji_ok {
            format!("{} {}", badge.emoji(), badge.word())
        } else {
            badge.word().to_string()
        };
        ui.label(RichText::new(text).strong());
    });
}

/// A wrapped warning line: "⚠ text".
pub fn warning_line(ui: &mut egui::Ui, text: &str) {
    ui.horizontal_wrapped(|ui| {
        ui.label(RichText::new("\u{26a0}").color(warning_color()).strong());
        ui.label(text);
    });
}

/// Striped two-column table of `(label, value)` rows.
pub fn kv_table(ui: &mut egui::Ui, id: &str, rows: &[(String, String)]) {
    egui::Grid::new(id)
        .num_columns(2)
        .striped(true)
        .spacing([12.0, 4.0])
        .show(ui, |ui| {
            for (k, v) in rows {
                ui.label(RichText::new(k).weak());
                ui.label(v);
                ui.end_row();
            }
        });
}

/// Compact number formatting: fixed for moderate magnitudes, scientific
/// otherwise.
pub fn fmt_num(v: f64) -> String {
    if !v.is_finite() {
        return if v.is_nan() {
            "n/a".to_string()
        } else if v > 0.0 {
            "\u{221e}".to_string()
        } else {
            "\u{2212}\u{221e}".to_string()
        };
    }
    let a = v.abs();
    if a == 0.0 {
        "0".to_string()
    } else if (1e-3..1e5).contains(&a) {
        let digits = if a >= 100.0 {
            1
        } else if a >= 1.0 {
            3
        } else {
            4
        };
        format!("{v:.digits$}")
    } else {
        format!("{v:.3e}")
    }
}

/// Texture slot that rebuilds only when its content key changes.
#[derive(Default)]
pub struct TextureSlot {
    key: Option<u64>,
    handle: Option<egui::TextureHandle>,
}

impl TextureSlot {
    /// The texture for `key`, built with `build` when the key changed.
    pub fn get(
        &mut self,
        ctx: &egui::Context,
        name: &str,
        key: u64,
        build: impl FnOnce() -> egui::ColorImage,
    ) -> egui::TextureId {
        if self.key != Some(key) || self.handle.is_none() {
            let image = build();
            match &mut self.handle {
                Some(h) => h.set(image, egui::TextureOptions::NEAREST),
                None => {
                    self.handle = Some(ctx.load_texture(name, image, egui::TextureOptions::NEAREST))
                }
            }
            self.key = Some(key);
        }
        self.handle.as_ref().expect("texture built").id()
    }
}

/// Hash of anything hashable (texture keys).
pub fn key_of(parts: impl Hash) -> u64 {
    let mut h = DefaultHasher::new();
    parts.hash(&mut h);
    h.finish()
}

/// RGBA pixels of `data` (`rows × cols`) through `colormap` over `range`.
/// With `flip_rows` the last data row becomes the top image row (data whose
/// row index grows upward, like y).
pub fn colorize(
    data: &Array2<f64>,
    colormap: Colormap,
    range: (f64, f64),
    dark: bool,
    flip_rows: bool,
) -> egui::ColorImage {
    let (rows, cols) = data.dim();
    let mut rgba = Vec::with_capacity(rows * cols * 4);
    for r in 0..rows {
        let src = if flip_rows { rows - 1 - r } else { r };
        for c in 0..cols {
            let [red, g, b] = colormap.rgb(normalize(data[[src, c]], range), dark);
            rgba.extend_from_slice(&[red, g, b, 255]);
        }
    }
    egui::ColorImage::from_rgba_unmultiplied([cols, rows], &rgba)
}

/// Heatmap description. Coordinates are in display units; the vertical
/// plot axis is `y` (upward) or `-depth` (depth views, labelled positive).
pub struct Heatmap<'a> {
    pub id: &'a str,
    /// Values `[row][col]`.
    pub data: &'a Array2<f64>,
    /// Horizontal cell-edge extent.
    pub x_range: (f64, f64),
    /// Vertical cell-edge extent (y, or depth for `depth_down`).
    pub v_range: (f64, f64),
    /// Row 0 is the top (depth views) instead of the bottom (y views).
    pub depth_down: bool,
    pub colormap: Colormap,
    pub value_range: (f64, f64),
    pub x_label: String,
    pub v_label: String,
    /// Square data units (x–y maps).
    pub equal_aspect: bool,
    /// Content key for the texture cache.
    pub key: u64,
    pub height: f32,
}

/// Hovered cell of a heatmap.
#[derive(Clone, Copy, Debug)]
pub struct HeatmapHover {
    /// Horizontal coordinate (display units).
    pub x: f64,
    /// Vertical coordinate (y, or depth for depth views; display units).
    pub v: f64,
    pub row: usize,
    pub col: usize,
    pub value: f64,
}

/// Draw a heatmap with axes; returns the hovered cell.
pub fn heatmap(ui: &mut egui::Ui, slot: &mut TextureSlot, h: &Heatmap<'_>) -> Option<HeatmapHover> {
    let dark = ui.visuals().dark_mode;
    let (rows, cols) = h.data.dim();
    if rows == 0 || cols == 0 {
        ui.label("(empty)");
        return None;
    }
    let tex = slot.get(ui.ctx(), h.id, h.key, || {
        colorize(h.data, h.colormap, h.value_range, dark, !h.depth_down)
    });
    let (x0, x1) = h.x_range;
    let (v0, v1) = h.v_range;
    // Plot coordinates: y up, so depth views plot −depth.
    let (p0, p1) = if h.depth_down { (-v1, -v0) } else { (v0, v1) };
    let center = PlotPoint::new(0.5 * (x0 + x1), 0.5 * (p0 + p1));
    let size = egui::vec2((x1 - x0) as f32, (p1 - p0) as f32);
    let depth_down = h.depth_down;
    let mut plot = Plot::new(h.id)
        .height(h.height)
        .x_axis_label(h.x_label.clone())
        .y_axis_label(h.v_label.clone())
        .allow_scroll(false)
        .show_grid(false)
        .include_x(x0)
        .include_x(x1)
        .include_y(p0)
        .include_y(p1)
        .label_formatter(|_, _| String::new());
    if h.equal_aspect {
        plot = plot.data_aspect(1.0);
    }
    if depth_down {
        plot = plot.y_axis_formatter(|mark, _| fmt_num(-mark.value));
    }
    let response = plot.show(ui, |plot_ui| {
        plot_ui.image(PlotImage::new(tex, center, size));
        plot_ui.pointer_coordinate()
    });
    let pointer = response.inner?;
    let v = if depth_down { -pointer.y } else { pointer.y };
    let col = crate::volume::cell_index(pointer.x, h.x_range, cols)?;
    let r = crate::volume::cell_index(v, h.v_range, rows)?;
    Some(HeatmapHover {
        x: pointer.x,
        v,
        row: r,
        col,
        value: h.data[[r, col]],
    })
}

/// Vertical colorbar with min / mid / max labels.
pub fn colorbar(
    ui: &mut egui::Ui,
    colormap: Colormap,
    range: (f64, f64),
    label: &str,
    height: f32,
) {
    let dark = ui.visuals().dark_mode;
    ui.vertical(|ui| {
        ui.label(RichText::new(label).small());
        ui.label(RichText::new(fmt_num(range.1)).small());
        let (rect, _) = ui.allocate_exact_size(egui::vec2(18.0, height), egui::Sense::hover());
        let mut mesh = egui::Mesh::default();
        let steps = 64;
        for s in 0..=steps {
            let t = s as f64 / steps as f64;
            let y = rect.bottom() - rect.height() * t as f32;
            let [r, g, b] = colormap.rgb(t, dark);
            let c = Color32::from_rgb(r, g, b);
            mesh.colored_vertex(egui::pos2(rect.left(), y), c);
            mesh.colored_vertex(egui::pos2(rect.right(), y), c);
        }
        for s in 0..steps {
            let i = 2 * s as u32;
            mesh.add_triangle(i, i + 1, i + 2);
            mesh.add_triangle(i + 1, i + 3, i + 2);
        }
        ui.painter().add(egui::Shape::mesh(mesh));
        ui.painter().rect_stroke(
            rect,
            0.0,
            egui::Stroke::new(1.0, ui.visuals().widgets.noninteractive.bg_stroke.color),
        );
        ui.label(RichText::new(fmt_num(range.0)).small());
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn number_formatting() {
        assert_eq!(fmt_num(0.0), "0");
        assert_eq!(fmt_num(157.63), "157.6");
        assert_eq!(fmt_num(13.5), "13.500");
        assert_eq!(fmt_num(0.4555), "0.4555");
        assert_eq!(fmt_num(2.5e6), "2.500e6");
        assert_eq!(fmt_num(-3.2e-7), "-3.200e-7");
        assert_eq!(fmt_num(f64::NAN), "n/a");
        assert_eq!(fmt_num(f64::INFINITY), "\u{221e}");
    }

    #[test]
    fn colorize_flips_rows_for_upward_axes() {
        let data = Array2::from_shape_vec((2, 1), vec![0.0, 1.0]).unwrap();
        let up = colorize(&data, Colormap::Grayscale, (0.0, 1.0), false, true);
        // Upward axis: the larger y (row 1, value 1 → white) is drawn on top.
        assert_eq!(up.pixels[0], Color32::WHITE);
        assert_eq!(up.pixels[1], Color32::BLACK);
        let down = colorize(&data, Colormap::Grayscale, (0.0, 1.0), false, false);
        assert_eq!(down.pixels[0], Color32::BLACK);
        assert_eq!(down.size, [1, 2]);
    }

    #[test]
    fn ordinal_ramp_is_monotone_and_theme_aware() {
        let lum =
            |c: Color32| 0.2126 * c.r() as f64 + 0.7152 * c.g() as f64 + 0.0722 * c.b() as f64;
        for dark in [false, true] {
            let cols: Vec<f64> = (0..5).map(|i| lum(ordinal(i, 5, dark))).collect();
            for w in cols.windows(2) {
                if dark {
                    assert!(w[1] > w[0], "dark ramp should brighten");
                } else {
                    assert!(w[1] < w[0], "light ramp should darken");
                }
            }
        }
        assert_ne!(series(0, false), series(1, false));
        assert_eq!(series(12, true), series(7, true));
    }

    /// Every non-ASCII character the UI sources put in text (escaped or
    /// literal, outside comments) must exist in egui's default fonts, or it
    /// renders as a box. The badge emoji are exempt: `badge_chip` checks
    /// them at run time and falls back to the badge word.
    #[test]
    fn ui_text_glyphs_exist_in_the_default_fonts() {
        let files = [
            include_str!("app.rs"),
            include_str!("compute.rs"),
            include_str!("imaging.rs"),
            include_str!("panels.rs"),
            include_str!("sources.rs"),
            include_str!("ui_state.rs"),
            include_str!("views.rs"),
            include_str!("volume.rs"),
            include_str!("widgets.rs"),
        ];
        let mut chars = std::collections::BTreeSet::new();
        for src in files {
            for line in src.lines() {
                if line.trim_start().starts_with("//") {
                    continue;
                }
                let mut rest = line;
                while let Some(pos) = rest.find("\\u{") {
                    let after = &rest[pos + 3..];
                    let Some(end) = after.find('}') else { break };
                    if let Some(c) = u32::from_str_radix(&after[..end], 16)
                        .ok()
                        .and_then(char::from_u32)
                    {
                        chars.insert(c);
                    }
                    rest = &after[end..];
                }
                chars.extend(line.chars().filter(|c| !c.is_ascii()));
            }
        }
        let ctx = egui::Context::default();
        let _ = ctx.run(egui::RawInput::default(), |_| {});
        let font = egui::FontId::proportional(14.0);
        let exempt = [
            Badge::Implemented.emoji(),
            Badge::Simplified.emoji(),
            Badge::Theoretical.emoji(),
        ]
        .concat();
        let missing: Vec<char> = chars
            .into_iter()
            .filter(|c| !exempt.contains(*c))
            .filter(|c| !ctx.fonts(|f| f.has_glyph(&font, *c)))
            .collect();
        assert!(
            missing.is_empty(),
            "glyphs missing from egui's default fonts: {missing:?}"
        );
    }

    #[test]
    fn texture_keys_differ_by_content() {
        assert_ne!(key_of((1u64, 2usize)), key_of((1u64, 3usize)));
        assert_eq!(
            key_of(("a", 1.5f64.to_bits())),
            key_of(("a", 1.5f64.to_bits()))
        );
    }
}
