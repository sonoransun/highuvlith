//! Cached 2D FFT engine used by every Fourier-optics stage.
//!
//! Row transforms run as one batched rustfft call over the contiguous
//! row-major buffer; the column pass transposes (cache-blocked) into a
//! thread-local work buffer, runs the batched transform again, and transposes
//! back. [`Fft2D::inverse_pruned_rows`] skips the row pass for rows that are
//! known to be zero, which roughly halves the cost of synthesizing a field
//! from a band-limited spectrum (the imaging engine's SOCS kernels occupy
//! only the rows inside the pupil support).
//!
//! Conventions: `forward` is unnormalized (`F[k] = Σ_n x[n] e^{-2πi kn/N}`),
//! `inverse` carries the full `1/(N_x N_y)` factor, and index `k` maps to
//! frequency `k/L` for `k < N/2` and `(k − N)/L` otherwise.
//!
//! # Model status
//!
//! Exact discrete Fourier transforms (rustfft) up to rounding. The DFT
//! implies periodic boundary conditions: every field is one period of an
//! infinite array (no zero padding, no windowing).

use ndarray::Array2;
use num::Complex;
use rustfft::{FftDirection, FftPlanner};
use std::cell::RefCell;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

type Complex64 = Complex<f64>;

/// Tile edge for the cache-blocked transpose.
const TRANSPOSE_TILE: usize = 32;

thread_local! {
    /// Per-thread (transpose buffer, rustfft scratch) reused across calls.
    static WORK: RefCell<(Vec<Complex64>, Vec<Complex64>)> =
        const { RefCell::new((Vec::new(), Vec::new())) };
}

/// Cached 2D FFT engine. Reuses FFT plans for repeated transforms of the same size.
pub struct Fft2D {
    planner: Mutex<FftPlanner<f64>>,
    forward_plans: Mutex<HashMap<usize, Arc<dyn rustfft::Fft<f64>>>>,
    inverse_plans: Mutex<HashMap<usize, Arc<dyn rustfft::Fft<f64>>>>,
}

/// Cache-blocked out-of-place transpose of a row-major `rows × cols` matrix.
fn transpose_into(src: &[Complex64], dst: &mut [Complex64], rows: usize, cols: usize) {
    debug_assert_eq!(src.len(), rows * cols);
    debug_assert_eq!(dst.len(), rows * cols);
    for ib in (0..rows).step_by(TRANSPOSE_TILE) {
        let i_end = (ib + TRANSPOSE_TILE).min(rows);
        for jb in (0..cols).step_by(TRANSPOSE_TILE) {
            let j_end = (jb + TRANSPOSE_TILE).min(cols);
            for i in ib..i_end {
                let row = &src[i * cols..(i + 1) * cols];
                for j in jb..j_end {
                    dst[j * rows + i] = row[j];
                }
            }
        }
    }
}

impl Fft2D {
    pub fn new() -> Self {
        Self {
            planner: Mutex::new(FftPlanner::new()),
            forward_plans: Mutex::new(HashMap::new()),
            inverse_plans: Mutex::new(HashMap::new()),
        }
    }

    fn get_plan(&self, n: usize, direction: FftDirection) -> Arc<dyn rustfft::Fft<f64>> {
        let cache = match direction {
            FftDirection::Forward => &self.forward_plans,
            FftDirection::Inverse => &self.inverse_plans,
        };

        let mut cache_lock = cache.lock().unwrap_or_else(|e| e.into_inner());
        if let Some(plan) = cache_lock.get(&n) {
            return Arc::clone(plan);
        }

        let mut planner = self.planner.lock().unwrap_or_else(|e| e.into_inner());
        let plan = match direction {
            FftDirection::Forward => planner.plan_fft_forward(n),
            FftDirection::Inverse => planner.plan_fft_inverse(n),
        };
        cache_lock.insert(n, Arc::clone(&plan));
        plan
    }

    /// Unnormalized 2D transform of a standard-layout array. `rows` limits
    /// the first (row) pass to the listed rows — the caller guarantees every
    /// other row is identically zero, so its transform is zero too.
    fn transform(
        &self,
        data: &mut Array2<Complex64>,
        direction: FftDirection,
        rows: Option<&[usize]>,
    ) {
        let (ny, nx) = data.dim();
        if ny == 0 || nx == 0 {
            return;
        }
        if data.as_slice().is_none() {
            // Non-standard layout (e.g. a transposed view): work on a copy.
            let mut owned = data.as_standard_layout().into_owned();
            self.transform(&mut owned, direction, rows);
            data.assign(&owned);
            return;
        }
        let plan_x = self.get_plan(nx, direction);
        let plan_y = self.get_plan(ny, direction);
        let slice = data.as_slice_mut().expect("standard layout checked above");
        WORK.with(|work| {
            let mut work = work.borrow_mut();
            let (tmp, scratch) = &mut *work;
            let scratch_len = plan_x
                .get_inplace_scratch_len()
                .max(plan_y.get_inplace_scratch_len());
            if scratch.len() < scratch_len {
                scratch.resize(scratch_len, Complex64::new(0.0, 0.0));
            }
            if tmp.len() < nx * ny {
                tmp.resize(nx * ny, Complex64::new(0.0, 0.0));
            }
            let tmp = &mut tmp[..nx * ny];

            // Row pass (all rows in one batched call, or only the listed ones).
            match rows {
                None => plan_x.process_with_scratch(slice, &mut scratch[..]),
                Some(list) => {
                    for &r in list {
                        plan_x.process_with_scratch(
                            &mut slice[r * nx..(r + 1) * nx],
                            &mut scratch[..],
                        );
                    }
                }
            }

            // Column pass through a transposed copy.
            transpose_into(slice, tmp, ny, nx);
            plan_y.process_with_scratch(tmp, &mut scratch[..]);
            transpose_into(tmp, slice, nx, ny);
        });
    }

    /// Forward 2D FFT (in-place, unnormalized).
    pub fn forward(&self, data: &mut Array2<Complex64>) {
        self.transform(data, FftDirection::Forward, None);
    }

    /// Inverse 2D FFT (in-place, with 1/N normalization).
    pub fn inverse(&self, data: &mut Array2<Complex64>) {
        self.transform(data, FftDirection::Inverse, None);
        let n = data.len() as f64;
        data.mapv_inplace(|v| v / n);
    }

    /// Inverse 2D FFT (in-place, 1/N normalized) of an array whose only
    /// non-zero rows are `nonzero_rows`. Rows not listed MUST be zero;
    /// the row pass is skipped for them. List each row at most once (a
    /// duplicate would be transformed twice).
    pub fn inverse_pruned_rows(&self, data: &mut Array2<Complex64>, nonzero_rows: &[usize]) {
        self.transform(data, FftDirection::Inverse, Some(nonzero_rows));
        let n = data.len() as f64;
        data.mapv_inplace(|v| v / n);
    }

    /// Forward FFT of real-valued 2D data.
    pub fn forward_real(&self, data: &Array2<f64>) -> Array2<Complex64> {
        let mut complex_data = data.mapv(|v| Complex64::new(v, 0.0));
        self.forward(&mut complex_data);
        complex_data
    }

    /// Apply FFT shift: move zero-frequency component to center.
    pub fn fftshift(data: &Array2<Complex64>) -> Array2<Complex64> {
        let (ny, nx) = data.dim();
        let hx = nx / 2;
        let hy = ny / 2;
        let mut shifted = Array2::zeros((ny, nx));
        for i in 0..ny {
            for j in 0..nx {
                let si = (i + hy) % ny;
                let sj = (j + hx) % nx;
                shifted[[si, sj]] = data[[i, j]];
            }
        }
        shifted
    }

    /// Inverse FFT shift: undo fftshift.
    pub fn ifftshift(data: &Array2<Complex64>) -> Array2<Complex64> {
        let (ny, nx) = data.dim();
        let hx = nx.div_ceil(2);
        let hy = ny.div_ceil(2);
        let mut shifted = Array2::zeros((ny, nx));
        for i in 0..ny {
            for j in 0..nx {
                let si = (i + hy) % ny;
                let sj = (j + hx) % nx;
                shifted[[si, sj]] = data[[i, j]];
            }
        }
        shifted
    }
}

impl Default for Fft2D {
    fn default() -> Self {
        Self::new()
    }
}

// Fft2D is safe to share across threads since all interior mutability
// is behind Mutex.
unsafe impl Send for Fft2D {}
unsafe impl Sync for Fft2D {}

#[cfg(test)]
mod tests {
    use super::*;
    use approx::assert_relative_eq;

    #[test]
    fn test_roundtrip() {
        let fft = Fft2D::new();
        let n = 64;
        let mut data = Array2::zeros((n, n));
        // Create a simple pattern
        data[[n / 2, n / 2]] = Complex64::new(1.0, 0.0);

        let original = data.clone();
        fft.forward(&mut data);
        fft.inverse(&mut data);

        for i in 0..n {
            for j in 0..n {
                assert_relative_eq!(data[[i, j]].re, original[[i, j]].re, epsilon = 1e-10);
                assert_relative_eq!(data[[i, j]].im, original[[i, j]].im, epsilon = 1e-10);
            }
        }
    }

    #[test]
    fn test_parseval() {
        let fft = Fft2D::new();
        let n = 64;
        let mut data = Array2::zeros((n, n));
        data[[10, 20]] = Complex64::new(3.0, 1.0);
        data[[30, 40]] = Complex64::new(-2.0, 0.5);

        let energy_spatial: f64 = data.iter().map(|v| v.norm_sqr()).sum();

        let mut freq = data.clone();
        fft.forward(&mut freq);
        let energy_freq: f64 = freq.iter().map(|v| v.norm_sqr()).sum();

        // Parseval: sum|f|^2 = (1/N) * sum|F|^2
        assert_relative_eq!(
            energy_spatial,
            energy_freq / (n * n) as f64,
            epsilon = 1e-10
        );
    }

    #[test]
    fn test_fftshift_ifftshift_roundtrip() {
        let n = 64;
        let mut data = Array2::zeros((n, n));
        data[[0, 0]] = Complex64::new(1.0, 0.0);
        data[[10, 20]] = Complex64::new(3.0, -1.0);
        data[[50, 30]] = Complex64::new(-2.0, 0.5);

        let shifted = Fft2D::fftshift(&data);
        let restored = Fft2D::ifftshift(&shifted);

        for i in 0..n {
            for j in 0..n {
                assert_relative_eq!(restored[[i, j]].re, data[[i, j]].re, epsilon = 1e-15);
                assert_relative_eq!(restored[[i, j]].im, data[[i, j]].im, epsilon = 1e-15);
            }
        }
    }

    #[test]
    fn test_forward_real_matches_forward() {
        let fft = Fft2D::new();
        let n = 64;
        let real_data = Array2::from_shape_fn((n, n), |(i, j)| ((i + j) as f64 * 0.1).sin());

        let result_real = fft.forward_real(&real_data);

        let mut complex_data = real_data.mapv(|v| Complex64::new(v, 0.0));
        fft.forward(&mut complex_data);

        for i in 0..n {
            for j in 0..n {
                assert_relative_eq!(
                    result_real[[i, j]].re,
                    complex_data[[i, j]].re,
                    epsilon = 1e-10
                );
                assert_relative_eq!(
                    result_real[[i, j]].im,
                    complex_data[[i, j]].im,
                    epsilon = 1e-10
                );
            }
        }
    }

    #[test]
    fn test_dc_component() {
        let fft = Fft2D::new();
        let n = 64;
        let constant_val = 3.5;
        let real_data = Array2::from_elem((n, n), constant_val);

        let result = fft.forward_real(&real_data);

        // DC component at [0,0] should be N*N * constant_val
        let dc = result[[0, 0]];
        assert_relative_eq!(dc.re, (n * n) as f64 * constant_val, epsilon = 1e-8);
        assert_relative_eq!(dc.im, 0.0, epsilon = 1e-8);

        // All other components should be zero
        for i in 0..n {
            for j in 0..n {
                if i == 0 && j == 0 {
                    continue;
                }
                assert_relative_eq!(result[[i, j]].norm(), 0.0, epsilon = 1e-8);
            }
        }
    }

    /// Naive O(N⁴) DFT reference for small non-square arrays.
    fn naive_dft(data: &Array2<Complex64>, sign: f64) -> Array2<Complex64> {
        let (ny, nx) = data.dim();
        let mut out = Array2::zeros((ny, nx));
        for ky in 0..ny {
            for kx in 0..nx {
                let mut acc = Complex64::new(0.0, 0.0);
                for y in 0..ny {
                    for x in 0..nx {
                        let ph = sign
                            * 2.0
                            * std::f64::consts::PI
                            * ((ky * y) as f64 / ny as f64 + (kx * x) as f64 / nx as f64);
                        acc += data[[y, x]] * Complex64::from_polar(1.0, ph);
                    }
                }
                out[[ky, kx]] = acc;
            }
        }
        out
    }

    #[test]
    fn test_forward_matches_naive_dft_non_square() {
        let fft = Fft2D::new();
        let data = Array2::from_shape_fn((6, 10), |(i, j)| {
            Complex64::new((i as f64 * 0.7 + j as f64).sin(), (i * j) as f64 * 0.1)
        });
        let mut fast = data.clone();
        fft.forward(&mut fast);
        let slow = naive_dft(&data, -1.0);
        for (a, b) in fast.iter().zip(slow.iter()) {
            assert!((a - b).norm() < 1e-10, "{a} vs {b}");
        }
        let mut back = fast.clone();
        fft.inverse(&mut back);
        for (a, b) in back.iter().zip(data.iter()) {
            assert!((a - b).norm() < 1e-12);
        }
    }

    #[test]
    fn test_inverse_pruned_rows_matches_full_inverse() {
        let fft = Fft2D::new();
        let n = 32;
        let rows = [0usize, 1, 5, 31];
        let mut data = Array2::zeros((n, n));
        for &r in &rows {
            for j in 0..n {
                data[[r, j]] = Complex64::new((r * 3 + j) as f64 * 0.01, (j as f64).cos());
            }
        }
        let mut full = data.clone();
        fft.inverse(&mut full);
        let mut pruned = data.clone();
        fft.inverse_pruned_rows(&mut pruned, &rows);
        for (a, b) in full.iter().zip(pruned.iter()) {
            assert!((a - b).norm() < 1e-14);
        }
    }

    #[test]
    fn test_non_standard_layout_is_handled() {
        let fft = Fft2D::new();
        let base = Array2::from_shape_fn((8, 4), |(i, j)| Complex64::new(i as f64, j as f64));
        let mut transposed = base.t().to_owned(); // standard layout copy (4 x 8)
        let mut view_based = base.clone().reversed_axes(); // non-standard layout (4 x 8)
        assert!(view_based.as_slice().is_none());
        fft.forward(&mut transposed);
        fft.forward(&mut view_based);
        for (a, b) in transposed.iter().zip(view_based.iter()) {
            assert!((a - b).norm() < 1e-12);
        }
    }
}
