//! 3D datasets for the volume viewer and the pure slicing / lookup logic
//! behind it (drawing lives in `widgets`).
//!
//! Every dataset is a `(z, y, x)` array on a cell-centred grid with
//! cell-edge extents, like `highuvlith_core::types::Grid3D`: `z` is the
//! depth below the resist top (or the propagation distance behind a
//! grating), positive downward. A Talbot carpet is an x–z plane, stored
//! with a single y row.

use highuvlith_core::types::{Grid2D, Grid3D};
use ndarray::{Array2, Array3, Axis};

use crate::colormap::{finite_range, Colormap};

/// Which computation produced a dataset.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum DatasetKind {
    /// Resist latent image (PAC) of the volumetric exposure (after the bake).
    Latent,
    /// Developed region (1 = dissolved) of the fast-marching / level-set
    /// development.
    Developed,
    /// LIGA absorbed-dose volume (kJ/cm³).
    LigaDose,
    /// Talbot carpet I(x, z) behind a grating (x–z plane only).
    TalbotCarpet,
}

impl DatasetKind {
    /// All kinds, in menu order.
    pub const ALL: [DatasetKind; 4] = [
        DatasetKind::Latent,
        DatasetKind::Developed,
        DatasetKind::LigaDose,
        DatasetKind::TalbotCarpet,
    ];

    /// Command-line name (`--dataset`).
    pub fn cli_name(self) -> &'static str {
        match self {
            DatasetKind::Latent => "latent",
            DatasetKind::Developed => "developed",
            DatasetKind::LigaDose => "liga",
            DatasetKind::TalbotCarpet => "talbot",
        }
    }

    /// Parse a `--dataset` value.
    pub fn from_cli_name(name: &str) -> Option<DatasetKind> {
        DatasetKind::ALL.into_iter().find(|d| d.cli_name() == name)
    }

    /// Menu label.
    pub fn label(self) -> &'static str {
        match self {
            DatasetKind::Latent => "Resist latent image (PAC)",
            DatasetKind::Developed => "Developed resist",
            DatasetKind::LigaDose => "LIGA absorbed dose",
            DatasetKind::TalbotCarpet => "Talbot carpet",
        }
    }
}

/// A `(z, y, x)` scalar field with physical extents.
#[derive(Clone, Debug)]
pub struct VolumeData {
    pub kind: DatasetKind,
    /// What the values are, e.g. "PAC m (1 = unexposed)".
    pub quantity: String,
    /// Value unit ("" for dimensionless).
    pub unit: String,
    /// Values, indexed `[z, y, x]`.
    pub data: Array3<f64>,
    /// Cell-edge extents in nm.
    pub x_nm: (f64, f64),
    pub y_nm: (f64, f64),
    pub z_nm: (f64, f64),
    /// Label of the z axis, e.g. "depth below the resist top".
    pub z_label: String,
    /// Length unit used for display: nm per display unit and its symbol.
    pub display_scale_nm: f64,
    pub display_unit: &'static str,
    /// Finite (min, max) of the whole volume.
    pub range: (f64, f64),
}

impl VolumeData {
    /// Wrap a core `Grid3D`.
    pub fn from_grid3d(
        kind: DatasetKind,
        g: &Grid3D<f64>,
        quantity: &str,
        unit: &str,
        z_label: &str,
    ) -> Self {
        Self::new(
            kind,
            g.data.clone(),
            (g.x_min_nm, g.x_max_nm),
            (g.y_min_nm, g.y_max_nm),
            (g.z_min_nm, g.z_max_nm),
            quantity,
            unit,
            z_label,
        )
    }

    /// Wrap an x–z plane stored as a `Grid2D` whose rows are z (the
    /// grid's y extent is the z range), e.g. a Talbot carpet.
    pub fn from_xz_plane(
        kind: DatasetKind,
        g: &Grid2D<f64>,
        quantity: &str,
        unit: &str,
        z_label: &str,
    ) -> Self {
        let (nz, nx) = g.data.dim();
        let data = g
            .data
            .clone()
            .into_shape_with_order((nz, 1, nx))
            .expect("same element count");
        let half_dy = 0.5;
        Self::new(
            kind,
            data,
            (g.x_min_nm, g.x_max_nm),
            (-half_dy, half_dy),
            (g.y_min_nm, g.y_max_nm),
            quantity,
            unit,
            z_label,
        )
    }

    #[allow(clippy::too_many_arguments)]
    fn new(
        kind: DatasetKind,
        data: Array3<f64>,
        x_nm: (f64, f64),
        y_nm: (f64, f64),
        z_nm: (f64, f64),
        quantity: &str,
        unit: &str,
        z_label: &str,
    ) -> Self {
        let range = finite_range(data.iter()).unwrap_or((0.0, 1.0));
        let span = (x_nm.1 - x_nm.0).max(z_nm.1 - z_nm.0);
        let (display_scale_nm, display_unit) = if span >= 20_000.0 {
            (1000.0, "\u{b5}m")
        } else {
            (1.0, "nm")
        };
        Self {
            kind,
            quantity: quantity.to_string(),
            unit: unit.to_string(),
            data,
            x_nm,
            y_nm,
            z_nm,
            z_label: z_label.to_string(),
            display_scale_nm,
            display_unit,
            range,
        }
    }

    /// `(nz, ny, nx)`.
    pub fn dims(&self) -> (usize, usize, usize) {
        self.data.dim()
    }

    /// An x–z plane only (no lateral y extent to slice).
    pub fn is_xz_plane(&self) -> bool {
        self.dims().1 == 1
    }

    /// Cell-centre coordinate (nm) of index `i` of `n` cells over `range`.
    fn centre(range: (f64, f64), n: usize, i: usize) -> f64 {
        range.0 + (i as f64 + 0.5) * (range.1 - range.0) / n as f64
    }

    /// Depth (nm) of slice `k`.
    pub fn z_at(&self, k: usize) -> f64 {
        Self::centre(self.z_nm, self.dims().0, k)
    }

    /// y (nm) of row `i`.
    pub fn y_at(&self, i: usize) -> f64 {
        Self::centre(self.y_nm, self.dims().1, i)
    }

    /// x (nm) of column `j`.
    pub fn x_at(&self, j: usize) -> f64 {
        Self::centre(self.x_nm, self.dims().2, j)
    }

    /// Lateral `(y, x)` slice at depth index `k` (clamped).
    pub fn xy_slice(&self, k: usize) -> Array2<f64> {
        let k = k.min(self.dims().0 - 1);
        self.data.index_axis(Axis(0), k).to_owned()
    }

    /// `(z, x)` cross-section through row `i` (clamped).
    pub fn xz_slice(&self, i: usize) -> Array2<f64> {
        let i = i.min(self.dims().1 - 1);
        self.data.index_axis(Axis(1), i).to_owned()
    }

    /// Row index nearest y = 0 (the cross-section the metrics use).
    pub fn centre_row(&self) -> usize {
        let (_, ny, _) = self.dims();
        cell_index(0.0, self.y_nm, ny).unwrap_or(ny / 2)
    }
}

/// Index of the cell containing `v` among `n` equal cells spanning
/// `range = (lo, hi)` (edges), or `None` outside.
pub fn cell_index(v: f64, range: (f64, f64), n: usize) -> Option<usize> {
    let (lo, hi) = range;
    if n == 0 || !(v.is_finite() && hi > lo) || v < lo || v > hi {
        return None;
    }
    let i = ((v - lo) / (hi - lo) * n as f64).floor() as usize;
    Some(i.min(n - 1))
}

/// How the colour scale is set.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum RangeMode {
    /// Min–max of the whole volume (slices stay comparable).
    #[default]
    Global,
    /// Min–max of each displayed slice.
    PerSlice,
}

/// Viewer state (pure; the textures live in the app).
#[derive(Clone, Debug, PartialEq)]
pub struct ViewerState {
    pub dataset: DatasetKind,
    /// Depth slice shown in the x–y view.
    pub slice_k: usize,
    /// Row shown in the x–z view.
    pub row_i: usize,
    pub colormap: Colormap,
    pub range_mode: RangeMode,
}

impl Default for ViewerState {
    fn default() -> Self {
        Self {
            dataset: DatasetKind::Latent,
            slice_k: 0,
            row_i: 0,
            colormap: Colormap::Inferno,
            range_mode: RangeMode::Global,
        }
    }
}

impl ViewerState {
    /// Keep the indices inside a (new) dataset; a dataset whose shape
    /// changed starts at the top slice and the centre row.
    pub fn conform(&mut self, v: &VolumeData, shape_changed: bool) {
        let (nz, ny, _) = v.dims();
        if shape_changed {
            self.slice_k = 0;
            self.row_i = v.centre_row();
        }
        self.slice_k = self.slice_k.min(nz.saturating_sub(1));
        self.row_i = self.row_i.min(ny.saturating_sub(1));
    }

    /// Colour range for a displayed array.
    pub fn color_range(&self, v: &VolumeData, shown: &Array2<f64>) -> (f64, f64) {
        match self.range_mode {
            RangeMode::Global => v.range,
            RangeMode::PerSlice => finite_range(shown.iter()).unwrap_or(v.range),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> VolumeData {
        // nz = 4, ny = 3, nx = 5; value = 100 k + 10 i + j.
        let g = Grid3D {
            data: Array3::from_shape_fn((4, 3, 5), |(k, i, j)| (100 * k + 10 * i + j) as f64),
            x_min_nm: -50.0,
            x_max_nm: 50.0,
            y_min_nm: -30.0,
            y_max_nm: 30.0,
            z_min_nm: 0.0,
            z_max_nm: 400.0,
        };
        VolumeData::from_grid3d(DatasetKind::Latent, &g, "PAC", "", "depth")
    }

    #[test]
    fn coordinates_are_cell_centres() {
        let v = sample();
        assert_eq!(v.dims(), (4, 3, 5));
        assert_eq!(v.z_at(0), 50.0);
        assert_eq!(v.z_at(3), 350.0);
        assert_eq!(v.x_at(0), -40.0);
        assert_eq!(v.y_at(1), 0.0);
        assert_eq!(v.centre_row(), 1);
        assert_eq!(v.range, (0.0, 324.0));
        assert_eq!(v.display_unit, "nm");
    }

    #[test]
    fn slices_pick_the_right_axes() {
        let v = sample();
        let xy = v.xy_slice(2);
        assert_eq!(xy.dim(), (3, 5));
        assert_eq!(xy[[1, 4]], 214.0);
        let xz = v.xz_slice(2);
        assert_eq!(xz.dim(), (4, 5));
        assert_eq!(xz[[3, 0]], 320.0);
        // Out-of-range indices clamp.
        assert_eq!(v.xy_slice(99)[[0, 0]], 300.0);
        assert_eq!(v.xz_slice(99)[[0, 0]], 20.0);
    }

    #[test]
    fn hover_lookup_maps_coordinates_to_cells() {
        let v = sample();
        let (nz, ny, nx) = v.dims();
        // x = 25 nm → column 3 (cells of 20 nm from −50), y = −25 → row 0.
        assert_eq!(cell_index(25.0, v.x_nm, nx), Some(3));
        assert_eq!(cell_index(-25.0, v.y_nm, ny), Some(0));
        assert_eq!(v.data[[1, 0, 3]], 103.0);
        // Exactly on the far edge → last cell; beyond → none.
        assert_eq!(cell_index(50.0, v.x_nm, nx), Some(4));
        assert_eq!(cell_index(50.1, v.x_nm, nx), None);
        // z = 260 nm → slice 2.
        assert_eq!(cell_index(260.0, v.z_nm, nz), Some(2));
        assert_eq!(cell_index(f64::NAN, (0.0, 1.0), 4), None);
        assert_eq!(cell_index(0.5, (1.0, 1.0), 4), None);
    }

    #[test]
    fn xz_plane_datasets_have_one_row() {
        let mut g = Grid2D::<f64>::new(6, 8, (-30.0, 30.0), (0.0, 800.0)).unwrap();
        for (r, mut row) in g.data.outer_iter_mut().enumerate() {
            row.fill(r as f64);
        }
        let v = VolumeData::from_xz_plane(DatasetKind::TalbotCarpet, &g, "I", "", "z");
        assert!(v.is_xz_plane());
        assert_eq!(v.dims(), (8, 1, 6));
        assert_eq!(v.z_nm, (0.0, 800.0));
        assert_eq!(v.xz_slice(0)[[5, 2]], 5.0);
        assert_eq!(v.centre_row(), 0);
    }

    #[test]
    fn micron_scale_volumes_display_in_microns() {
        let g = Grid3D {
            data: Array3::zeros((2, 2, 2)),
            x_min_nm: -64_000.0,
            x_max_nm: 64_000.0,
            y_min_nm: -64_000.0,
            y_max_nm: 64_000.0,
            z_min_nm: 0.0,
            z_max_nm: 500_000.0,
        };
        let v = VolumeData::from_grid3d(DatasetKind::LigaDose, &g, "dose", "kJ/cm\u{b3}", "depth");
        assert_eq!((v.display_scale_nm, v.display_unit), (1000.0, "\u{b5}m"));
    }

    #[test]
    fn viewer_state_conforms_and_picks_ranges() {
        let v = sample();
        let mut s = ViewerState {
            slice_k: 10,
            row_i: 10,
            ..ViewerState::default()
        };
        s.conform(&v, false);
        assert_eq!((s.slice_k, s.row_i), (3, 2));
        s.conform(&v, true);
        assert_eq!((s.slice_k, s.row_i), (0, 1));
        let shown = v.xy_slice(1);
        assert_eq!(s.color_range(&v, &shown), (0.0, 324.0));
        s.range_mode = RangeMode::PerSlice;
        assert_eq!(s.color_range(&v, &shown), (100.0, 124.0));
    }
}
