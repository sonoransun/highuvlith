//! Colormaps for the heatmap views: a normalized value in [0, 1] → sRGB.
//!
//! - `Inferno` reproduces `highuvlith_core::io::image_export`'s inferno-like
//!   map exactly, so the GUI shows the same colours as
//!   `highuvlith simulate --output image.png`.
//! - `Blues` is the one-hue sequential ramp of the docs' data-viz palette
//!   (steps 100 → 700). Its low end sits next to the chart surface: light
//!   in the light theme, dark in the dark theme.
//! - `Grayscale` is black → white.
//!
//! Non-finite values map to a neutral mid-gray ("no data").

/// A heatmap colormap.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum Colormap {
    /// Black → purple → orange → yellow (same formula as the CLI PNG export).
    #[default]
    Inferno,
    /// One-hue sequential blue ramp.
    Blues,
    /// Black → white.
    Grayscale,
}

/// Sequential blue ramp, steps 100 … 700 (light → dark).
const BLUES: [[u8; 3]; 13] = [
    [0xcd, 0xe2, 0xfb],
    [0xb7, 0xd3, 0xf6],
    [0x9e, 0xc5, 0xf4],
    [0x86, 0xb6, 0xef],
    [0x6d, 0xa7, 0xec],
    [0x55, 0x98, 0xe7],
    [0x39, 0x87, 0xe5],
    [0x2a, 0x78, 0xd6],
    [0x25, 0x6a, 0xbf],
    [0x1c, 0x5c, 0xab],
    [0x18, 0x4f, 0x95],
    [0x10, 0x42, 0x81],
    [0x0d, 0x36, 0x6b],
];

/// Colour used for NaN / infinite samples.
pub const NO_DATA: [u8; 3] = [128, 128, 128];

impl Colormap {
    /// Every colormap, in menu order.
    pub const ALL: [Colormap; 3] = [Colormap::Inferno, Colormap::Blues, Colormap::Grayscale];

    /// Menu label.
    pub fn label(self) -> &'static str {
        match self {
            Colormap::Inferno => "Inferno",
            Colormap::Blues => "Blues (sequential)",
            Colormap::Grayscale => "Grayscale",
        }
    }

    /// sRGB colour of the normalized value `t` (clamped to [0, 1]).
    /// `dark_mode` flips the `Blues` ramp so its low end stays next to the
    /// dark surface; the other maps do not depend on the theme.
    pub fn rgb(self, t: f64, dark_mode: bool) -> [u8; 3] {
        if !t.is_finite() {
            return NO_DATA;
        }
        let t = t.clamp(0.0, 1.0);
        match self {
            Colormap::Inferno => inferno(t),
            Colormap::Grayscale => {
                let v = (t * 255.0) as u8;
                [v, v, v]
            }
            Colormap::Blues => {
                let u = if dark_mode { 1.0 - t } else { t };
                ramp(&BLUES, u)
            }
        }
    }
}

/// Same arithmetic as `highuvlith_core::io::image_export`'s `Colormap::Inferno`
/// (kept in sync by hand: that function is private to the core crate).
fn inferno(t: f64) -> [u8; 3] {
    let t = t as f32;
    if t < 0.25 {
        let s = t / 0.25;
        [(s * 80.0) as u8, 0, (s * 120.0) as u8]
    } else if t < 0.5 {
        let s = (t - 0.25) / 0.25;
        [
            (80.0 + s * 140.0) as u8,
            (s * 30.0) as u8,
            (120.0 - s * 70.0) as u8,
        ]
    } else if t < 0.75 {
        let s = (t - 0.5) / 0.25;
        [
            (220.0 + s * 35.0) as u8,
            (30.0 + s * 130.0) as u8,
            (50.0 - s * 50.0) as u8,
        ]
    } else {
        let s = (t - 0.75) / 0.25;
        [255, (160.0 + s * 95.0) as u8, (s * 100.0) as u8]
    }
}

/// Piecewise-linear interpolation through equally spaced colour stops.
fn ramp(stops: &[[u8; 3]], t: f64) -> [u8; 3] {
    let n = stops.len();
    let x = t * (n - 1) as f64;
    let i = (x.floor() as usize).min(n - 2);
    let f = x - i as f64;
    let (a, b) = (stops[i], stops[i + 1]);
    let mix = |p: u8, q: u8| (p as f64 + (q as f64 - p as f64) * f).round() as u8;
    [mix(a[0], b[0]), mix(a[1], b[1]), mix(a[2], b[2])]
}

/// Linear normalization of `v` into [0, 1] over `range = (lo, hi)`; values
/// outside the range are clamped, a degenerate range maps every finite value
/// to 0.5, and non-finite values stay non-finite (→ [`NO_DATA`]).
pub fn normalize(v: f64, range: (f64, f64)) -> f64 {
    if !v.is_finite() {
        return f64::NAN;
    }
    let (lo, hi) = range;
    let span = hi - lo;
    if !(span.is_finite() && span > 0.0) {
        return 0.5;
    }
    ((v - lo) / span).clamp(0.0, 1.0)
}

/// Finite `(min, max)` of the samples; `None` when there is no finite sample.
pub fn finite_range<'a>(values: impl IntoIterator<Item = &'a f64>) -> Option<(f64, f64)> {
    let mut range: Option<(f64, f64)> = None;
    for &v in values {
        if v.is_finite() {
            range = Some(match range {
                None => (v, v),
                Some((lo, hi)) => (lo.min(v), hi.max(v)),
            });
        }
    }
    range
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn inferno_matches_core_export_formula() {
        // Hand-evaluated from highuvlith_core::io::image_export::apply_colormap.
        assert_eq!(Colormap::Inferno.rgb(0.0, false), [0, 0, 0]);
        assert_eq!(Colormap::Inferno.rgb(0.25, false), [80, 0, 120]);
        assert_eq!(Colormap::Inferno.rgb(0.5, false), [220, 30, 50]);
        assert_eq!(Colormap::Inferno.rgb(0.75, false), [255, 160, 0]);
        assert_eq!(Colormap::Inferno.rgb(1.0, false), [255, 255, 100]);
        // Theme independent.
        assert_eq!(
            Colormap::Inferno.rgb(0.6, true),
            Colormap::Inferno.rgb(0.6, false)
        );
    }

    #[test]
    fn blues_runs_light_to_dark_and_flips_in_dark_mode() {
        assert_eq!(Colormap::Blues.rgb(0.0, false), BLUES[0]);
        assert_eq!(Colormap::Blues.rgb(1.0, false), BLUES[12]);
        assert_eq!(Colormap::Blues.rgb(0.0, true), BLUES[12]);
        assert_eq!(Colormap::Blues.rgb(1.0, true), BLUES[0]);
        // Stops are hit exactly (13 stops → every 1/12).
        assert_eq!(Colormap::Blues.rgb(0.5, false), BLUES[6]);
        // Luminance decreases monotonically along the light-theme ramp.
        let lum = |c: [u8; 3]| 0.2126 * c[0] as f64 + 0.7152 * c[1] as f64 + 0.0722 * c[2] as f64;
        let mut prev = f64::INFINITY;
        for k in 0..=100 {
            let l = lum(Colormap::Blues.rgb(k as f64 / 100.0, false));
            assert!(l <= prev + 1e-9, "not monotone at {k}");
            prev = l;
        }
    }

    #[test]
    fn grayscale_endpoints_and_clamping() {
        assert_eq!(Colormap::Grayscale.rgb(0.0, false), [0, 0, 0]);
        assert_eq!(Colormap::Grayscale.rgb(1.0, true), [255, 255, 255]);
        assert_eq!(Colormap::Grayscale.rgb(-3.0, false), [0, 0, 0]);
        assert_eq!(Colormap::Grayscale.rgb(7.0, false), [255, 255, 255]);
        for cmap in Colormap::ALL {
            assert_eq!(cmap.rgb(f64::NAN, false), NO_DATA);
        }
    }

    #[test]
    fn normalize_and_range() {
        assert_eq!(normalize(5.0, (0.0, 10.0)), 0.5);
        assert_eq!(normalize(-1.0, (0.0, 10.0)), 0.0);
        assert_eq!(normalize(11.0, (0.0, 10.0)), 1.0);
        assert_eq!(normalize(3.0, (2.0, 2.0)), 0.5);
        assert!(normalize(f64::INFINITY, (0.0, 1.0)).is_nan());
        let v = [f64::NAN, 2.0, -1.0, f64::INFINITY, 4.0];
        assert_eq!(finite_range(v.iter()), Some((-1.0, 4.0)));
        assert_eq!(finite_range([f64::NAN].iter()), None);
    }
}
