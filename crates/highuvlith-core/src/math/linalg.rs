//! Dense complex linear algebra for the SOCS decomposition of the imaging
//! engine's transmission cross-coefficient (TCC).
//!
//! The engine never forms the TCC element by element: it assembles the
//! factor `A` (one column per source point, or per source point ×
//! polarization field component) with `TCC = A·Aᴴ`, and needs the leading
//! eigenpairs of that Hermitian positive semi-definite product. Three tools
//! cover every problem size:
//!
//! - [`ColMatrix::gram`] — the Hermitian Gram matrix `XᴴX` of a set of column
//!   vectors, tiled and parallel, streaming the data in row chunks;
//! - [`HermitianEigen`] / [`hermitian_eigen_desc`] — complete Hermitian
//!   eigendecomposition (Householder tridiagonalization, phase
//!   normalization to a real tridiagonal, implicit QL with shifts), sorted by
//!   decreasing eigenvalue, eigenvectors back-transformed on demand;
//! - [`randomized_eigen`] — randomized subspace iteration: a Gaussian range
//!   finder with `q` power iterations, re-orthonormalized after every
//!   product, followed by a Rayleigh–Ritz step, giving the leading `l`
//!   eigenpairs of `X·Xᴴ` without forming it.
//!
//! # Key equations
//!
//! ```text
//!   Gram:           H_ab = Σ_i conj(X_ia) X_ib            (H = XᴴX, Hermitian PSD)
//!   Range finder:   Y = X Ω,  Q = orth(Y);  repeat q times: Q ← orth(X · orth(Xᴴ Q))
//!   Rayleigh–Ritz:  Z = Xᴴ Q,  T = ZᴴZ = Qᴴ (X Xᴴ) Q = W M Wᴴ,  U = Q W,  λ_k ≈ M_kk
//! ```
//! Orthonormalization is classical Gram–Schmidt with one full
//! re-orthogonalization pass per column ("twice is enough"); numerically
//! rank-deficient columns are replaced by fresh random directions.
//!
//! # Model status
//!
//! Exact dense algorithms up to rounding (eigenpairs reproduce `H` to
//! ~1e-14 relative). The randomized path is an approximation whose error
//! decays like `(λ_{l+1}/λ_k)^{q+1/2}`; the engine reports the captured TCC
//! energy so truncation is never silent.
//!
//! # References
//!
//! - N. Halko, P.-G. Martinsson, J. A. Tropp, "Finding structure with
//!   randomness: probabilistic algorithms for constructing approximate matrix
//!   decompositions," SIAM Review 53(2), 217–288 (2011).

use nalgebra::DMatrix;
use rand::rngs::StdRng;
use rand::SeedableRng;
use rand_distr::{Distribution, StandardNormal};

use crate::compute::parallel::{for_each_chunk_mut, map_range, num_threads};
use crate::types::Complex64;

/// Rows streamed per chunk in the tiled products (keeps the working set in L2).
const ROW_CHUNK: usize = 256;
/// Column-tile edge for the Gram product.
const GRAM_TILE: usize = 48;
/// Orthonormalize columns at least this long in parallel.
const PARALLEL_ORTHO_ROWS: usize = 16_384;

const ZERO: Complex64 = Complex64 { re: 0.0, im: 0.0 };

/// Conjugated dot product `Σ_i conj(x_i) · y_i`.
#[inline]
pub fn dotc(x: &[Complex64], y: &[Complex64]) -> Complex64 {
    let n = x.len().min(y.len());
    let (x, y) = (&x[..n], &y[..n]);
    let (mut r0, mut r1, mut i0, mut i1) = (0.0, 0.0, 0.0, 0.0);
    let mut xc = x.chunks_exact(2);
    let mut yc = y.chunks_exact(2);
    for (a, b) in (&mut xc).zip(&mut yc) {
        r0 += a[0].re * b[0].re + a[0].im * b[0].im;
        i0 += a[0].re * b[0].im - a[0].im * b[0].re;
        r1 += a[1].re * b[1].re + a[1].im * b[1].im;
        i1 += a[1].re * b[1].im - a[1].im * b[1].re;
    }
    for (a, b) in xc.remainder().iter().zip(yc.remainder()) {
        r0 += a.re * b.re + a.im * b.im;
        i0 += a.re * b.im - a.im * b.re;
    }
    Complex64::new(r0 + r1, i0 + i1)
}

/// `y += alpha · x`.
#[inline]
fn axpy(alpha: Complex64, x: &[Complex64], y: &mut [Complex64]) {
    for (yi, xi) in y.iter_mut().zip(x) {
        *yi += alpha * xi;
    }
}

/// Column-major complex matrix: column `c` is the contiguous slice
/// `data[c·rows .. (c+1)·rows]`.
#[derive(Debug, Clone, PartialEq)]
pub struct ColMatrix {
    rows: usize,
    cols: usize,
    data: Vec<Complex64>,
}

impl ColMatrix {
    /// All-zero `rows × cols` matrix.
    pub fn zeros(rows: usize, cols: usize) -> Self {
        Self {
            rows,
            cols,
            data: vec![ZERO; rows * cols],
        }
    }

    /// Wrap column-major data.
    ///
    /// # Panics
    /// Panics if `data.len() != rows * cols`.
    pub fn from_col_major(rows: usize, cols: usize, data: Vec<Complex64>) -> Self {
        assert_eq!(data.len(), rows * cols, "ColMatrix data length mismatch");
        Self { rows, cols, data }
    }

    /// Copy of an nalgebra matrix (also column-major).
    pub fn from_dmatrix(m: &DMatrix<Complex64>) -> Self {
        Self::from_col_major(m.nrows(), m.ncols(), m.as_slice().to_vec())
    }

    /// Number of rows (length of every column).
    pub fn rows(&self) -> usize {
        self.rows
    }

    /// Number of columns.
    pub fn cols(&self) -> usize {
        self.cols
    }

    /// Column `c`.
    pub fn col(&self, c: usize) -> &[Complex64] {
        &self.data[c * self.rows..(c + 1) * self.rows]
    }

    /// Mutable column `c`.
    pub fn col_mut(&mut self, c: usize) -> &mut [Complex64] {
        &mut self.data[c * self.rows..(c + 1) * self.rows]
    }

    /// The raw column-major buffer.
    pub fn as_slice(&self) -> &[Complex64] {
        &self.data
    }

    /// Mutable raw column-major buffer.
    pub fn as_mut_slice(&mut self) -> &mut [Complex64] {
        &mut self.data
    }

    /// Consume into the raw column-major buffer.
    pub fn into_vec(self) -> Vec<Complex64> {
        self.data
    }

    /// Copy into an nalgebra matrix.
    pub fn to_dmatrix(&self) -> DMatrix<Complex64> {
        DMatrix::from_column_slice(self.rows, self.cols, &self.data)
    }

    /// Squared Frobenius norm `Σ |X_ij|²` (fixed chunking, partial sums added
    /// in order: bit-reproducible under any thread schedule).
    pub fn frobenius_norm_sq(&self) -> f64 {
        let data = &self.data;
        let chunk = ROW_CHUNK * 16;
        map_range(data.len().div_ceil(chunk), |k| {
            let hi = ((k + 1) * chunk).min(data.len());
            data[k * chunk..hi]
                .iter()
                .map(|v| v.norm_sqr())
                .sum::<f64>()
        })
        .into_iter()
        .sum()
    }

    /// Hermitian Gram matrix `H = XᴴX` (`cols × cols`), both triangles filled.
    pub fn gram(&self) -> DMatrix<Complex64> {
        let n = self.cols;
        let rows = self.rows;
        let n_tiles = n.div_ceil(GRAM_TILE);
        // Upper-triangular tile pairs (ta <= tb).
        let pairs: Vec<(usize, usize)> = (0..n_tiles)
            .flat_map(|ta| (ta..n_tiles).map(move |tb| (ta, tb)))
            .collect();
        let tiles = map_range(pairs.len(), |p| {
            let (ta, tb) = pairs[p];
            let (a0, a1) = (ta * GRAM_TILE, ((ta + 1) * GRAM_TILE).min(n));
            let (b0, b1) = (tb * GRAM_TILE, ((tb + 1) * GRAM_TILE).min(n));
            let wb = b1 - b0;
            let mut tile = vec![ZERO; (a1 - a0) * wb];
            let mut r = 0;
            while r < rows {
                let r_end = (r + ROW_CHUNK).min(rows);
                for a in a0..a1 {
                    let xa = &self.col(a)[r..r_end];
                    let b_start = if ta == tb { a } else { b0 };
                    for b in b_start..b1 {
                        tile[(a - a0) * wb + (b - b0)] += dotc(xa, &self.col(b)[r..r_end]);
                    }
                }
                r = r_end;
            }
            tile
        });
        let mut h = DMatrix::<Complex64>::zeros(n, n);
        for (p, tile) in pairs.iter().zip(tiles) {
            let (ta, tb) = *p;
            let (a0, a1) = (ta * GRAM_TILE, ((ta + 1) * GRAM_TILE).min(n));
            let (b0, b1) = (tb * GRAM_TILE, ((tb + 1) * GRAM_TILE).min(n));
            let wb = b1 - b0;
            for a in a0..a1 {
                let b_start = if ta == tb { a } else { b0 };
                for b in b_start..b1 {
                    let v = tile[(a - a0) * wb + (b - b0)];
                    h[(a, b)] = v;
                    h[(b, a)] = v.conj();
                }
            }
        }
        // The diagonal is real up to rounding; make it exactly real.
        for a in 0..n {
            h[(a, a)].im = 0.0;
        }
        h
    }

    /// Product `X · B` (`rows × B.ncols()`).
    ///
    /// # Panics
    /// Panics if `B.nrows() != self.cols()`.
    pub fn mul(&self, b: &DMatrix<Complex64>) -> ColMatrix {
        assert_eq!(b.nrows(), self.cols, "ColMatrix::mul shape mismatch");
        let k = b.ncols();
        let rows = self.rows;
        // Enough row chunks to keep every thread busy even for short columns.
        let chunk = rows.div_ceil(4 * num_threads()).clamp(16, ROW_CHUNK);
        let n_chunks = rows.div_ceil(chunk);
        let chunks = map_range(n_chunks, |ci| {
            let r0 = ci * chunk;
            let r1 = (r0 + chunk).min(rows);
            let len = r1 - r0;
            let mut out = vec![ZERO; len * k];
            for c in 0..self.cols {
                let xc = &self.col(c)[r0..r1];
                for j in 0..k {
                    let bcj = b[(c, j)];
                    if bcj != ZERO {
                        axpy(bcj, xc, &mut out[j * len..(j + 1) * len]);
                    }
                }
            }
            out
        });
        let mut result = ColMatrix::zeros(rows, k);
        for (ci, out) in chunks.iter().enumerate() {
            let r0 = ci * chunk;
            let len = out.len() / k.max(1);
            for j in 0..k {
                result.col_mut(j)[r0..r0 + len].copy_from_slice(&out[j * len..(j + 1) * len]);
            }
        }
        result
    }

    /// Product `Xᴴ · Q` (`self.cols() × q.cols()`).
    ///
    /// # Panics
    /// Panics if `q.rows() != self.rows()`.
    pub fn adjoint_mul(&self, q: &ColMatrix) -> ColMatrix {
        assert_eq!(q.rows, self.rows, "ColMatrix::adjoint_mul shape mismatch");
        let k = q.cols;
        let rows = self.rows;
        let n_blocks = self.cols.div_ceil(GRAM_TILE);
        let blocks = map_range(n_blocks, |bi| {
            let c0 = bi * GRAM_TILE;
            let c1 = (c0 + GRAM_TILE).min(self.cols);
            let mut acc = vec![ZERO; (c1 - c0) * k];
            let mut r = 0;
            while r < rows {
                let r_end = (r + ROW_CHUNK).min(rows);
                for c in c0..c1 {
                    let xc = &self.col(c)[r..r_end];
                    for j in 0..k {
                        acc[(c - c0) * k + j] += dotc(xc, &q.col(j)[r..r_end]);
                    }
                }
                r = r_end;
            }
            acc
        });
        let mut result = ColMatrix::zeros(self.cols, k);
        for (bi, acc) in blocks.iter().enumerate() {
            let c0 = bi * GRAM_TILE;
            let width = acc.len() / k.max(1);
            for dc in 0..width {
                for j in 0..k {
                    result.col_mut(j)[c0 + dc] = acc[dc * k + j];
                }
            }
        }
        result
    }
}

/// Eigendecomposition of a Hermitian matrix `H = V Λ Vᴴ`.
///
/// Algorithm: Householder reduction to Hermitian tridiagonal form
/// `T = QᴴHQ` (reflections chosen so no cancellation occurs), a diagonal
/// unitary `D` that makes the off-diagonal real and non-negative
/// (`T̃ = DᴴTD`), the implicit QL iteration with Wilkinson-type shifts on
/// the real symmetric tridiagonal `T̃` (textbook `tql2` form) accumulating
/// its eigenvectors `Z`, and finally `V = Q·D·Z`, applied only to the
/// eigenvectors actually requested ([`HermitianEigen::vectors`]).
///
/// (nalgebra 0.33's complex `SymmetricEigen` is not used: for some
/// structured Hermitian inputs — e.g. defocused TCCs — it returns an
/// orthonormal but wrong eigenvector basis, reconstructing `H` only to
/// ~1e-5.)
#[derive(Debug, Clone)]
pub struct HermitianEigen {
    n: usize,
    /// Eigenvalues, decreasing.
    values: Vec<f64>,
    /// Unit Householder vectors `u_k` acting on rows `k+1..n` (None = identity).
    reflectors: Vec<Option<Vec<Complex64>>>,
    /// Diagonal of `D`.
    phases: Vec<Complex64>,
    /// Eigenvectors of `T̃`, column-major n×n, in the original QL order.
    z: Vec<f64>,
    /// `order[k]` = column of `z` holding the k-th largest eigenvalue.
    order: Vec<usize>,
}

/// Implicit QL iteration for a real symmetric tridiagonal matrix with
/// diagonal `d` and off-diagonal `e` (`e[i]` couples `d[i]` and `d[i+1]`,
/// `e[n-1]` unused). On return `d` holds the eigenvalues and the columns of
/// `z` (column-major n×n, initialized to the identity by the caller) the
/// eigenvectors. Returns `false` if an eigenvalue fails to converge.
fn tql2(d: &mut [f64], e: &mut [f64], z: &mut [f64]) -> bool {
    let n = d.len();
    // ‖T‖ bound. Off-diagonal elements below ε‖T‖ are dropped as well as
    // those below ε·(|d_m| + |d_{m+1}|): a backward-stable perturbation of
    // size ε‖T‖, without which a rank-deficient matrix (a cluster of
    // rounding-level eigenvalues next to large ones) never converges.
    let dmax = d.iter().fold(0.0f64, |a, b| a.max(b.abs()));
    let emax = e.iter().fold(0.0f64, |a, b| a.max(b.abs()));
    let small = f64::EPSILON * (dmax + 2.0 * emax);
    for l in 0..n {
        let mut iterations = 0;
        loop {
            // Find a negligible off-diagonal element to split the matrix.
            let mut m = l;
            while m + 1 < n {
                let dd = d[m].abs() + d[m + 1].abs();
                if e[m].abs() <= f64::EPSILON * dd || e[m].abs() <= small {
                    break;
                }
                m += 1;
            }
            if m == l {
                break;
            }
            iterations += 1;
            if iterations > 90 {
                return false;
            }
            // Shift from the leading 2×2 block.
            let mut g = (d[l + 1] - d[l]) / (2.0 * e[l]);
            let mut r = g.hypot(1.0);
            g = d[m] - d[l] + e[l] / (g + r.copysign(g));
            let (mut s, mut c, mut p) = (1.0, 1.0, 0.0);
            let mut underflow = false;
            let mut i = m;
            while i > l {
                i -= 1;
                let f = s * e[i];
                let b = c * e[i];
                r = f.hypot(g);
                e[i + 1] = r;
                if r == 0.0 {
                    d[i + 1] -= p;
                    e[m] = 0.0;
                    underflow = true;
                    break;
                }
                s = f / r;
                c = g / r;
                g = d[i + 1] - p;
                r = (d[i] - g) * s + 2.0 * c * b;
                p = s * r;
                d[i + 1] = g + p;
                g = c * r - b;
                let (lo, hi) = z.split_at_mut((i + 1) * n);
                let zi = &mut lo[i * n..];
                let zi1 = &mut hi[..n];
                for (a, b1) in zi.iter_mut().zip(zi1.iter_mut()) {
                    let t = *b1;
                    *b1 = s * *a + c * t;
                    *a = c * *a - s * t;
                }
            }
            if underflow {
                continue;
            }
            d[l] -= p;
            e[l] = g;
            e[m] = 0.0;
        }
    }
    true
}

impl HermitianEigen {
    /// Decompose `h` (both triangles are read; `h` must be Hermitian).
    ///
    /// # Panics
    /// Panics if `h` is not square or if the QL iteration fails to converge
    /// (does not happen for finite Hermitian input).
    pub fn new(h: &DMatrix<Complex64>) -> Self {
        assert_eq!(
            h.nrows(),
            h.ncols(),
            "Hermitian eigenproblem needs a square matrix"
        );
        let n = h.nrows();
        let scale = h.iter().map(|v| v.norm()).fold(0.0, f64::max);
        let inv = if scale > 0.0 { 1.0 / scale } else { 1.0 };
        let mut a = h.map(|v| v * inv);
        let mut sub = vec![ZERO; n.saturating_sub(1)];
        let mut reflectors: Vec<Option<Vec<Complex64>>> = Vec::with_capacity(n);
        for k in 0..n.saturating_sub(2) {
            let m = n - k - 1;
            let alpha = (0..m)
                .map(|i| a[(k + 1 + i, k)].norm_sqr())
                .sum::<f64>()
                .sqrt();
            if alpha == 0.0 {
                sub[k] = ZERO;
                reflectors.push(None);
                continue;
            }
            let x0 = a[(k + 1, k)];
            let phase = if x0.norm() > 0.0 {
                x0 / x0.norm()
            } else {
                Complex64::new(1.0, 0.0)
            };
            let beta = -phase * alpha;
            let mut u: Vec<Complex64> = (0..m).map(|i| a[(k + 1 + i, k)]).collect();
            u[0] -= beta;
            let unorm = u.iter().map(|v| v.norm_sqr()).sum::<f64>().sqrt();
            u.iter_mut().for_each(|v| *v /= unorm);
            // Trailing block A22 ← H A22 H with H = I − 2uuᴴ:
            // p = 2·A22·u, K = uᴴp (real), q = p − K·u, A22 −= u qᴴ + q uᴴ.
            let mut p = vec![ZERO; m];
            for (j, &uj) in u.iter().enumerate() {
                let uj = uj * 2.0;
                let col = a.column(k + 1 + j);
                for (pi, &aij) in p.iter_mut().zip(col.iter().skip(k + 1)) {
                    *pi += aij * uj;
                }
            }
            let kk = dotc(&u, &p).re;
            let q: Vec<Complex64> = p.iter().zip(&u).map(|(pi, ui)| pi - ui * kk).collect();
            for j in 0..m {
                let (qj, uj) = (q[j].conj(), u[j].conj());
                let mut col = a.column_mut(k + 1 + j);
                for i in 0..m {
                    col[k + 1 + i] -= u[i] * qj + q[i] * uj;
                }
            }
            sub[k] = beta;
            reflectors.push(Some(u));
        }
        if n >= 2 {
            sub[n - 2] = a[(n - 1, n - 2)];
        }
        let mut d: Vec<f64> = (0..n).map(|i| a[(i, i)].re).collect();
        let mut e: Vec<f64> = sub
            .iter()
            .map(|v| v.norm())
            .chain(std::iter::once(0.0))
            .collect();
        e.truncate(n);
        // D: δ_0 = 1, δ_{k+1} = δ_k · e_k/|e_k|.
        let mut phases = vec![Complex64::new(1.0, 0.0); n];
        for k in 0..n.saturating_sub(1) {
            let r = sub[k].norm();
            phases[k + 1] = if r > 0.0 {
                phases[k] * (sub[k] / r)
            } else {
                phases[k]
            };
        }
        let mut z = vec![0.0; n * n];
        for i in 0..n {
            z[i * n + i] = 1.0;
        }
        assert!(
            tql2(&mut d, &mut e, &mut z),
            "Hermitian eigensolver: QL iteration failed to converge"
        );
        let mut order: Vec<usize> = (0..n).collect();
        order.sort_by(|&x, &y| d[y].total_cmp(&d[x]));
        let values = order.iter().map(|&i| d[i] * scale).collect();
        Self {
            n,
            values,
            reflectors,
            phases,
            z,
            order,
        }
    }

    /// All eigenvalues, decreasing.
    pub fn values(&self) -> &[f64] {
        &self.values
    }

    /// The first `count` eigenvectors (decreasing eigenvalue) as the
    /// orthonormal columns of an `n × count` matrix.
    pub fn vectors(&self, count: usize) -> DMatrix<Complex64> {
        let n = self.n;
        let count = count.min(n);
        // W = D · Z[:, order[..count]]
        let mut w = DMatrix::<Complex64>::from_fn(n, count, |i, c| {
            self.phases[i] * self.z[self.order[c] * n + i]
        });
        // V = H_0 H_1 ⋯ H_{n-3} W  (apply the last reflector first).
        for (k, refl) in self.reflectors.iter().enumerate().rev() {
            if let Some(u) = refl {
                for c in 0..count {
                    let mut col = w.column_mut(c);
                    let tail = &mut col.as_mut_slice()[k + 1..];
                    let t = dotc(u, tail) * 2.0;
                    for (x, ui) in tail.iter_mut().zip(u) {
                        *x -= ui * t;
                    }
                }
            }
        }
        w
    }
}

/// Full eigendecomposition of a Hermitian matrix, sorted by decreasing
/// eigenvalue: returns `(λ, V)` with `H·V[:,k] = λ_k·V[:,k]` and orthonormal
/// columns (see [`HermitianEigen`]).
pub fn hermitian_eigen_desc(h: DMatrix<Complex64>) -> (Vec<f64>, DMatrix<Complex64>) {
    let n = h.nrows();
    if n == 0 {
        return (Vec::new(), DMatrix::zeros(0, 0));
    }
    let eig = HermitianEigen::new(&h);
    let v = eig.vectors(n);
    (eig.values, v)
}

/// Complex standard-normal sample (unit variance: E|z|² = 1).
fn complex_gaussian(rng: &mut StdRng) -> Complex64 {
    let re: f64 = StandardNormal.sample(rng);
    let im: f64 = StandardNormal.sample(rng);
    Complex64::new(re, im) * std::f64::consts::FRAC_1_SQRT_2
}

/// Orthonormalize the columns of `m` in place (classical Gram–Schmidt with
/// one re-orthogonalization pass per column). A column that is numerically
/// dependent on its predecessors is replaced by a random direction, so the
/// result always has orthonormal columns (requires `cols <= rows`).
pub fn orthonormalize(m: &mut ColMatrix, rng: &mut StdRng) {
    let rows = m.rows;
    let cols = m.cols;
    assert!(cols <= rows, "cannot orthonormalize more columns than rows");
    let n_chunks = rows.div_ceil(ROW_CHUNK);
    // Short columns: thread fan-out costs more than the arithmetic.
    let parallel = rows >= PARALLEL_ORTHO_ROWS;
    for j in 0..cols {
        let original_norm = dotc(m.col(j), m.col(j)).re.sqrt();
        let mut attempts = 0;
        loop {
            let (done, rest) = m.data.split_at_mut(j * rows);
            let v = &mut rest[..rows];
            for _pass in 0..2 {
                if j == 0 {
                    break;
                }
                // coeffs_i = <q_i, v>, then v -= Σ_i coeffs_i q_i
                let coeffs: Vec<Complex64> = if parallel {
                    // Per-chunk partial dot products, summed in chunk order
                    // (deterministic regardless of scheduling).
                    let partials = map_range(n_chunks, |ci| {
                        let r0 = ci * ROW_CHUNK;
                        let r1 = (r0 + ROW_CHUNK).min(rows);
                        (0..j)
                            .map(|i| dotc(&done[i * rows + r0..i * rows + r1], &v[r0..r1]))
                            .collect::<Vec<_>>()
                    });
                    let mut acc = vec![ZERO; j];
                    for part in partials {
                        for (a, p) in acc.iter_mut().zip(part) {
                            *a += p;
                        }
                    }
                    acc
                } else {
                    (0..j)
                        .map(|i| dotc(&done[i * rows..(i + 1) * rows], v))
                        .collect()
                };
                let done_ref: &[Complex64] = done;
                let update = |ci: usize, vc: &mut [Complex64]| {
                    let r0 = ci * ROW_CHUNK;
                    for (i, &c) in coeffs.iter().enumerate() {
                        let qi = &done_ref[i * rows + r0..i * rows + r0 + vc.len()];
                        axpy(-c, qi, vc);
                    }
                };
                if parallel {
                    for_each_chunk_mut(v, ROW_CHUNK, update);
                } else {
                    for (ci, vc) in v.chunks_mut(ROW_CHUNK).enumerate() {
                        update(ci, vc);
                    }
                }
            }
            let norm = dotc(v, v).re.sqrt();
            let reference = if attempts == 0 { original_norm } else { 1.0 };
            if norm > 1e-10 * reference && norm > 0.0 {
                let inv = 1.0 / norm;
                v.iter_mut().for_each(|x| *x *= inv);
                break;
            }
            // Numerically dependent (or zero) column: try a random direction.
            attempts += 1;
            assert!(
                attempts < 8,
                "orthonormalize: failed to find an independent direction"
            );
            for x in v.iter_mut() {
                *x = complex_gaussian(rng);
            }
        }
    }
}

/// Leading eigenpairs of `X·Xᴴ` by randomized subspace iteration.
///
/// Returns `(λ, U)` with at most `l` eigenvalues in decreasing order and the
/// corresponding orthonormal eigenvector approximations as the columns of
/// `U` (`X.rows() × l`). `power_iterations` (`q`) trades time for accuracy;
/// `seed` makes the result deterministic.
pub fn randomized_eigen(
    x: &ColMatrix,
    l: usize,
    power_iterations: usize,
    seed: u64,
) -> (Vec<f64>, ColMatrix) {
    let l = l.min(x.rows).min(x.cols).max(1);
    let mut rng = StdRng::seed_from_u64(seed);
    let omega = DMatrix::<Complex64>::from_fn(x.cols, l, |_, _| complex_gaussian(&mut rng));
    let mut q = x.mul(&omega);
    orthonormalize(&mut q, &mut rng);
    for _ in 0..power_iterations {
        let mut z = x.adjoint_mul(&q);
        orthonormalize(&mut z, &mut rng);
        q = x.mul(&z.to_dmatrix());
        orthonormalize(&mut q, &mut rng);
    }
    let z = x.adjoint_mul(&q);
    let t = z.gram();
    let (values, w) = hermitian_eigen_desc(t);
    let u = q.mul(&w);
    (values, u)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn random_colmatrix(rows: usize, cols: usize, seed: u64) -> ColMatrix {
        let mut rng = StdRng::seed_from_u64(seed);
        let data = (0..rows * cols)
            .map(|_| complex_gaussian(&mut rng))
            .collect();
        ColMatrix::from_col_major(rows, cols, data)
    }

    fn max_abs_diff(a: &DMatrix<Complex64>, b: &DMatrix<Complex64>) -> f64 {
        a.iter()
            .zip(b.iter())
            .map(|(x, y)| (x - y).norm())
            .fold(0.0, f64::max)
    }

    #[test]
    fn dotc_matches_naive() {
        let x = random_colmatrix(7, 1, 1);
        let y = random_colmatrix(7, 1, 2);
        let naive: Complex64 = x
            .col(0)
            .iter()
            .zip(y.col(0))
            .map(|(a, b)| a.conj() * b)
            .sum();
        assert!((dotc(x.col(0), y.col(0)) - naive).norm() < 1e-14);
    }

    #[test]
    fn gram_matches_naive_product() {
        // More than one tile and more than one row chunk.
        let x = random_colmatrix(600, 101, 3);
        let naive = x.to_dmatrix().adjoint() * x.to_dmatrix();
        let g = x.gram();
        assert!(max_abs_diff(&g, &naive) < 1e-10);
        for a in 0..101 {
            assert_eq!(g[(a, a)].im, 0.0);
        }
    }

    #[test]
    fn mul_and_adjoint_mul_match_naive() {
        let x = random_colmatrix(530, 60, 4);
        let b = random_colmatrix(60, 9, 5).to_dmatrix();
        let naive = x.to_dmatrix() * &b;
        assert!(max_abs_diff(&x.mul(&b).to_dmatrix(), &naive) < 1e-10);

        let q = random_colmatrix(530, 7, 6);
        let naive_adj = x.to_dmatrix().adjoint() * q.to_dmatrix();
        assert!(max_abs_diff(&x.adjoint_mul(&q).to_dmatrix(), &naive_adj) < 1e-10);
    }

    #[test]
    fn frobenius_norm_matches_sum() {
        let x = random_colmatrix(1000, 13, 7);
        let naive: f64 = x.as_slice().iter().map(|v| v.norm_sqr()).sum();
        assert!((x.frobenius_norm_sq() - naive).abs() < 1e-9 * naive);
    }

    #[test]
    fn hermitian_eigen_reconstructs_and_sorts() {
        let x = random_colmatrix(40, 30, 8);
        let h = x.gram();
        let (vals, v) = hermitian_eigen_desc(h.clone());
        for w in vals.windows(2) {
            assert!(w[0] >= w[1]);
        }
        let d = DMatrix::from_diagonal(&nalgebra::DVector::from_iterator(
            vals.len(),
            vals.iter().map(|&l| Complex64::new(l, 0.0)),
        ));
        let recon = &v * d * v.adjoint();
        let scale = h.iter().map(|c| c.norm()).fold(0.0, f64::max);
        assert!(max_abs_diff(&recon, &h) < 1e-12 * scale);
        let id = DMatrix::<Complex64>::identity(30, 30);
        assert!(max_abs_diff(&(v.adjoint() * &v), &id) < 1e-12);
    }

    #[test]
    fn orthonormalize_handles_dependent_columns() {
        let mut m = random_colmatrix(50, 6, 9);
        // Make column 3 a combination of columns 0 and 1, column 5 zero.
        let c0 = m.col(0).to_vec();
        let c1 = m.col(1).to_vec();
        for (i, v) in m.col_mut(3).iter_mut().enumerate() {
            *v = c0[i] * 2.0 - c1[i] * Complex64::new(0.0, 1.0);
        }
        m.col_mut(5).iter_mut().for_each(|v| *v = ZERO);
        let mut rng = StdRng::seed_from_u64(1);
        orthonormalize(&mut m, &mut rng);
        let g = m.gram();
        let id = DMatrix::<Complex64>::identity(6, 6);
        assert!(max_abs_diff(&g, &id) < 1e-12);
    }

    #[test]
    fn randomized_eigen_recovers_decaying_spectrum() {
        // X = U diag(s) Vᴴ with s_k = 0.6^k: the eigenvalues of X Xᴴ are s_k².
        let (rows, cols, rank) = (400, 300, 60);
        let mut rng = StdRng::seed_from_u64(10);
        let mut u = random_colmatrix(rows, rank, 11);
        orthonormalize(&mut u, &mut rng);
        let mut v = random_colmatrix(cols, rank, 12);
        orthonormalize(&mut v, &mut rng);
        let s: Vec<f64> = (0..rank).map(|k| 0.6f64.powi(k as i32)).collect();
        let us = DMatrix::from_fn(rows, rank, |i, k| u.col(k)[i] * s[k]);
        let x = ColMatrix::from_dmatrix(&(us * v.to_dmatrix().adjoint()));

        let (vals, vecs) = randomized_eigen(&x, 20, 3, 42);
        for k in 0..8 {
            let exact = s[k] * s[k];
            assert!(
                (vals[k] - exact).abs() < 1e-9 * s[0] * s[0],
                "λ_{k}: {} vs {exact}",
                vals[k]
            );
            let overlap = dotc(u.col(k), vecs.col(k)).norm();
            assert!(
                (overlap - 1.0).abs() < 1e-8,
                "eigenvector {k} overlap {overlap}"
            );
        }
    }

    fn check_decomposition(h: &DMatrix<Complex64>, tol: f64) {
        let n = h.nrows();
        let (vals, v) = hermitian_eigen_desc(h.clone());
        for w in vals.windows(2) {
            assert!(w[0] >= w[1]);
        }
        let d = DMatrix::from_diagonal(&nalgebra::DVector::from_iterator(
            n,
            vals.iter().map(|&l| Complex64::new(l, 0.0)),
        ));
        let recon = &v * d * v.adjoint();
        let scale = h.iter().map(|c| c.norm()).fold(1e-300, f64::max);
        let err = max_abs_diff(&recon, h) / scale;
        assert!(err < tol, "reconstruction error {err:.2e}");
        let id = DMatrix::<Complex64>::identity(n, n);
        let orth = max_abs_diff(&(v.adjoint() * &v), &id);
        assert!(orth < tol, "orthogonality error {orth:.2e}");
    }

    #[test]
    fn hermitian_eigen_structured_inputs() {
        // Degenerate spectrum with a random unitary basis.
        let mut rng = StdRng::seed_from_u64(20);
        let mut u = random_colmatrix(9, 9, 21);
        orthonormalize(&mut u, &mut rng);
        let spectrum = [3.0, 3.0, 3.0, 1.0, 1.0, 0.5, 0.0, 0.0, -0.25];
        let ud = DMatrix::from_fn(9, 9, |i, k| u.col(k)[i] * spectrum[k]);
        let h = &ud * u.to_dmatrix().adjoint();
        check_decomposition(&h, 1e-13);
        let (vals, _) = hermitian_eigen_desc(h);
        for (a, b) in vals
            .iter()
            .zip([3.0, 3.0, 3.0, 1.0, 1.0, 0.5, 0.0, 0.0, -0.25])
        {
            assert!((a - b).abs() < 1e-13);
        }
        // Diagonal (every Householder column is already zero), block
        // diagonal with complex couplings, 1×1 and 2×2, and all-zero.
        let diag = DMatrix::from_fn(5, 5, |i, j| {
            if i == j {
                Complex64::new(i as f64 - 2.0, 0.0)
            } else {
                ZERO
            }
        });
        check_decomposition(&diag, 1e-14);
        let mut block = DMatrix::<Complex64>::zeros(6, 6);
        for (i, j, v) in [
            (0, 1, Complex64::new(0.3, 0.8)),
            (2, 4, Complex64::new(-0.5, 0.2)),
            (3, 5, Complex64::new(0.0, -1.1)),
            (4, 5, Complex64::new(0.25, 0.25)),
        ] {
            block[(i, j)] = v;
            block[(j, i)] = v.conj();
        }
        for i in 0..6 {
            block[(i, i)] = Complex64::new(0.1 * i as f64, 0.0);
        }
        check_decomposition(&block, 1e-14);
        check_decomposition(
            &DMatrix::from_element(1, 1, Complex64::new(2.5, 0.0)),
            1e-15,
        );
        let two = DMatrix::from_row_slice(
            2,
            2,
            &[
                Complex64::new(1.0, 0.0),
                Complex64::new(0.0, 2.0),
                Complex64::new(0.0, -2.0),
                Complex64::new(1.0, 0.0),
            ],
        );
        check_decomposition(&two, 1e-15);
        let (vals, _) = hermitian_eigen_desc(DMatrix::zeros(4, 4));
        assert!(vals.iter().all(|&v| v == 0.0));
    }

    #[test]
    fn hermitian_eigen_partial_vectors_match_full() {
        let x = random_colmatrix(50, 20, 22);
        let h = x.gram();
        let eig = HermitianEigen::new(&h);
        let full = eig.vectors(20);
        let part = eig.vectors(5);
        for c in 0..5 {
            for i in 0..20 {
                assert_eq!(full[(i, c)], part[(i, c)]);
            }
        }
    }

    #[test]
    fn hermitian_eigen_rank_deficient_with_rounding_noise() {
        // λ ≈ 96 and ~1e-14 noise elsewhere: the configuration that stalls a
        // purely relative QL convergence test.
        let n = 60;
        let mut rng = StdRng::seed_from_u64(30);
        let a = random_colmatrix(n, 1, 31);
        let mut h = DMatrix::from_fn(n, n, |i, j| a.col(0)[i] * a.col(0)[j].conj() * 2.0);
        for i in 0..n {
            for j in 0..=i {
                let noise = complex_gaussian(&mut rng) * 1e-14;
                h[(i, j)] += noise;
                if i == j {
                    h[(i, i)].im = 0.0;
                } else {
                    h[(j, i)] = h[(i, j)].conj();
                }
            }
        }
        check_decomposition(&h, 1e-13);
    }

    #[test]
    fn orthonormalize_long_columns_parallel_branch() {
        // rows >= PARALLEL_ORTHO_ROWS exercises the chunked (parallel) path;
        // it must agree bit-for-bit across runs and give orthonormal columns.
        let rows = PARALLEL_ORTHO_ROWS + 1234;
        let base = random_colmatrix(rows, 4, 40);
        let run = || {
            let mut m = base.clone();
            let mut rng = StdRng::seed_from_u64(2);
            orthonormalize(&mut m, &mut rng);
            m
        };
        let (a, b) = (run(), run());
        assert_eq!(a, b);
        let g = a.gram();
        assert!(max_abs_diff(&g, &DMatrix::<Complex64>::identity(4, 4)) < 1e-12);
    }
}
