use ndarray::Array2;
use num::Complex;
use serde::{Deserialize, Serialize};

pub type Complex64 = Complex<f64>;

/// 2D grid with physical coordinate mapping.
#[derive(Debug, Clone)]
pub struct Grid2D<T> {
    pub data: Array2<T>,
    pub x_min_nm: f64,
    pub x_max_nm: f64,
    pub y_min_nm: f64,
    pub y_max_nm: f64,
}

impl<T: Clone + Default> Grid2D<T> {
    pub fn new(
        nx: usize,
        ny: usize,
        x_range: (f64, f64),
        y_range: (f64, f64),
    ) -> crate::error::Result<Self> {
        if nx == 0 {
            return Err(crate::error::LithographyError::InvalidParameter {
                name: "nx",
                value: 0.0,
                reason: "must be positive",
            });
        }
        if ny == 0 {
            return Err(crate::error::LithographyError::InvalidParameter {
                name: "ny",
                value: 0.0,
                reason: "must be positive",
            });
        }
        if x_range.0 >= x_range.1 {
            return Err(crate::error::LithographyError::InvalidParameter {
                name: "x_range",
                value: x_range.0,
                reason: "x_min must be less than x_max",
            });
        }
        if y_range.0 >= y_range.1 {
            return Err(crate::error::LithographyError::InvalidParameter {
                name: "y_range",
                value: y_range.0,
                reason: "y_min must be less than y_max",
            });
        }
        Ok(Self {
            data: Array2::default((ny, nx)),
            x_min_nm: x_range.0,
            x_max_nm: x_range.1,
            y_min_nm: y_range.0,
            y_max_nm: y_range.1,
        })
    }

    pub fn nx(&self) -> usize {
        self.data.ncols()
    }

    pub fn ny(&self) -> usize {
        self.data.nrows()
    }

    pub fn pixel_size_x(&self) -> f64 {
        (self.x_max_nm - self.x_min_nm) / self.nx() as f64
    }

    pub fn pixel_size_y(&self) -> f64 {
        (self.y_max_nm - self.y_min_nm) / self.ny() as f64
    }

    /// Physical x-coordinate for column index.
    pub fn x_at(&self, col: usize) -> f64 {
        self.x_min_nm + (col as f64 + 0.5) * self.pixel_size_x()
    }

    /// Physical y-coordinate for row index.
    pub fn y_at(&self, row: usize) -> f64 {
        self.y_min_nm + (row as f64 + 0.5) * self.pixel_size_y()
    }
}

/// 3D grid for volumetric data (e.g., resist dose distribution).
///
/// Axis order of `data` is `(z, y, x)`: `data[[k, i, j]]` is depth slice
/// `k`, row `i`, column `j` — so `data.index_axis(Axis(0), k)` yields a
/// lateral `(y, x)` slice directly comparable to `Grid2D::data`.
/// By convention z is measured from the resist top, positive downward
/// (matching `FilmStack::standing_wave`).
#[derive(Debug, Clone)]
pub struct Grid3D<T> {
    pub data: ndarray::Array3<T>,
    pub x_min_nm: f64,
    pub x_max_nm: f64,
    pub y_min_nm: f64,
    pub y_max_nm: f64,
    pub z_min_nm: f64,
    pub z_max_nm: f64,
}

impl<T: Clone + Default> Grid3D<T> {
    pub fn new(
        nx: usize,
        ny: usize,
        nz: usize,
        x_range: (f64, f64),
        y_range: (f64, f64),
        z_range: (f64, f64),
    ) -> crate::error::Result<Self> {
        if nx == 0 {
            return Err(crate::error::LithographyError::InvalidParameter {
                name: "nx",
                value: 0.0,
                reason: "must be positive",
            });
        }
        if ny == 0 {
            return Err(crate::error::LithographyError::InvalidParameter {
                name: "ny",
                value: 0.0,
                reason: "must be positive",
            });
        }
        if nz == 0 {
            return Err(crate::error::LithographyError::InvalidParameter {
                name: "nz",
                value: 0.0,
                reason: "must be positive",
            });
        }
        if x_range.0 >= x_range.1 {
            return Err(crate::error::LithographyError::InvalidParameter {
                name: "x_range",
                value: x_range.0,
                reason: "x_min must be less than x_max",
            });
        }
        if y_range.0 >= y_range.1 {
            return Err(crate::error::LithographyError::InvalidParameter {
                name: "y_range",
                value: y_range.0,
                reason: "y_min must be less than y_max",
            });
        }
        if z_range.0 >= z_range.1 {
            return Err(crate::error::LithographyError::InvalidParameter {
                name: "z_range",
                value: z_range.0,
                reason: "z_min must be less than z_max",
            });
        }
        Ok(Self {
            data: ndarray::Array3::default((nz, ny, nx)),
            x_min_nm: x_range.0,
            x_max_nm: x_range.1,
            y_min_nm: y_range.0,
            y_max_nm: y_range.1,
            z_min_nm: z_range.0,
            z_max_nm: z_range.1,
        })
    }

    pub fn nx(&self) -> usize {
        self.data.shape()[2]
    }

    pub fn ny(&self) -> usize {
        self.data.shape()[1]
    }

    pub fn nz(&self) -> usize {
        self.data.shape()[0]
    }

    pub fn pixel_size_x(&self) -> f64 {
        (self.x_max_nm - self.x_min_nm) / self.nx() as f64
    }

    pub fn pixel_size_y(&self) -> f64 {
        (self.y_max_nm - self.y_min_nm) / self.ny() as f64
    }

    pub fn pixel_size_z(&self) -> f64 {
        (self.z_max_nm - self.z_min_nm) / self.nz() as f64
    }

    /// Physical x-coordinate for column index.
    pub fn x_at(&self, col: usize) -> f64 {
        self.x_min_nm + (col as f64 + 0.5) * self.pixel_size_x()
    }

    /// Physical y-coordinate for row index.
    pub fn y_at(&self, row: usize) -> f64 {
        self.y_min_nm + (row as f64 + 0.5) * self.pixel_size_y()
    }

    /// Physical z-coordinate (depth from resist top) for slice index.
    pub fn z_at(&self, slice: usize) -> f64 {
        self.z_min_nm + (slice as f64 + 0.5) * self.pixel_size_z()
    }
}

/// Simulation grid configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GridConfig {
    /// Number of grid points (must be power of 2).
    pub size: usize,
    /// Physical pixel size at wafer in nm.
    pub pixel_nm: f64,
}

impl GridConfig {
    pub fn new(size: usize, pixel_nm: f64) -> crate::error::Result<Self> {
        if !size.is_power_of_two() {
            return Err(crate::error::LithographyError::GridSizeNotPowerOfTwo(size));
        }
        if !(pixel_nm.is_finite() && pixel_nm > 0.0) {
            return Err(crate::error::LithographyError::InvalidParameter {
                name: "pixel_nm",
                value: pixel_nm,
                reason: "must be positive",
            });
        }
        Ok(Self { size, pixel_nm })
    }

    /// Grid of `size` pixels whose square field is an exact whole multiple of
    /// `pitch_x_nm` and, if given, `pitch_y_nm`, with the pixel as close to
    /// `target_pixel_nm` as possible without being coarser.
    ///
    /// The imaging FFTs make the field periodic, so a periodic mask is imaged
    /// without a boundary defect only if the field holds a whole number of
    /// periods along each axis. Because the field is square, both pitches
    /// must divide it: the unit cell is their smallest common period
    /// ([`common_period_nm`]). The field is `M` unit cells with
    /// `M = max(1, ⌊size·target_pixel_nm / cell⌋)`, so the pixel is
    /// `M·cell/size ≤ target_pixel_nm` — except when one cell does not fit at
    /// the target pixel (`size·target_pixel_nm < cell`), where the pixel
    /// coarsens to `cell/size` to hold exactly one cell.
    ///
    /// # Errors
    /// `size` not a power of two; a non-positive or non-finite pitch or
    /// target; two pitches without a common period (their ratio must be a
    /// ratio of integers up to 1000, e.g. 150:200 = 3:4).
    pub fn commensurate(
        pitch_x_nm: f64,
        pitch_y_nm: Option<f64>,
        size: usize,
        target_pixel_nm: f64,
    ) -> crate::error::Result<Self> {
        use crate::error::LithographyError;
        if !size.is_power_of_two() {
            return Err(LithographyError::GridSizeNotPowerOfTwo(size));
        }
        let positive = |name: &'static str, v: f64| {
            if v.is_finite() && v > 0.0 {
                Ok(())
            } else {
                Err(LithographyError::InvalidParameter {
                    name,
                    value: v,
                    reason: "must be positive and finite",
                })
            }
        };
        positive("pitch_x_nm", pitch_x_nm)?;
        positive("target_pixel_nm", target_pixel_nm)?;
        let cell = match pitch_y_nm {
            None => pitch_x_nm,
            Some(py) => {
                positive("pitch_y_nm", py)?;
                common_period_nm(pitch_x_nm, py).ok_or_else(|| {
                    LithographyError::DimensionMismatch {
                        expected: format!(
                            "pitches {pitch_x_nm} nm and {py} nm with a common period (a square \
                             field can only hold whole periods of both if their ratio is a ratio \
                             of integers up to {MAX_COMMON_PERIOD_MULTIPLE}, e.g. 150:200 = 3:4)"
                        ),
                        got: format!("ratio {:.9}", pitch_x_nm / py),
                    }
                })?
            }
        };
        let cells = ((size as f64 * target_pixel_nm / cell) + 1e-9)
            .floor()
            .max(1.0);
        Self::new(size, cells * cell / size as f64)
    }

    pub fn field_size_nm(&self) -> f64 {
        self.size as f64 * self.pixel_nm
    }

    /// Number of periods of `pitch_nm` in the field (`field_size_nm / pitch_nm`).
    pub fn periods_in_field(&self, pitch_nm: f64) -> f64 {
        self.field_size_nm() / pitch_nm
    }

    /// True if the field holds a whole number (≥ 1) of periods of `pitch_nm`,
    /// to the relative tolerance [`crate::mask::COMMENSURATE_RTOL`].
    pub fn is_commensurate_with(&self, pitch_nm: f64) -> bool {
        if !(pitch_nm.is_finite() && pitch_nm > 0.0) {
            return false;
        }
        let periods = self.periods_in_field(pitch_nm);
        let m = periods.round();
        m >= 1.0 && (periods - m).abs() <= crate::mask::COMMENSURATE_RTOL * periods
    }

    /// Frequency spacing in 1/nm.
    pub fn freq_step(&self) -> f64 {
        1.0 / self.field_size_nm()
    }
}

/// Largest multiple searched by [`common_period_nm`].
pub const MAX_COMMON_PERIOD_MULTIPLE: usize = 1000;

/// Smallest common period (least common multiple) of two pitches: the
/// smallest `c = i·a` (`i ≤ 1000`) that is also a whole multiple of `b` to a
/// relative tolerance of 1e-9. `None` for invalid pitches or when no such
/// multiple exists (irrational or large-integer ratios).
pub fn common_period_nm(a: f64, b: f64) -> Option<f64> {
    if !(a.is_finite() && b.is_finite() && a > 0.0 && b > 0.0) {
        return None;
    }
    (1..=MAX_COMMON_PERIOD_MULTIPLE).find_map(|i| {
        let c = i as f64 * a;
        let j = (c / b).round();
        (j >= 1.0 && (c - j * b).abs() <= 1e-9 * c).then_some(c)
    })
}

/// Polarization state for thin-film calculations.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Polarization {
    TE,
    TM,
    Unpolarized,
}

impl Default for GridConfig {
    fn default() -> Self {
        Self {
            size: 512,
            pixel_nm: 1.0,
        }
    }
}
