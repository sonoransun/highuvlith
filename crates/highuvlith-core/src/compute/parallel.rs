//! Data-parallel helpers that compile to Rayon with the `parallel` feature
//! and to plain sequential loops without it.
//!
//! The imaging engine's hot loops (A-matrix assembly, Gram products,
//! per-kernel inverse FFTs) all reduce to "map over an index range" or
//! "fold over an index range, then combine the partial results". Routing
//! them through these helpers keeps every call site free of
//! `#[cfg(feature = "parallel")]` duplication.
//!
//! # Model status
//!
//! Scheduling only — no physics. `map_range` preserves order, so ordered
//! reductions over its output are bit-reproducible; `fold_range` combines
//! partial results in a scheduling-dependent order (last-bit differences
//! between runs), so the imaging engine avoids it where reproducibility
//! matters.

#[cfg(feature = "parallel")]
use rayon::prelude::*;

/// Number of worker threads the helpers below fan out to (1 without the
/// `parallel` feature).
pub fn num_threads() -> usize {
    #[cfg(feature = "parallel")]
    {
        rayon::current_num_threads().max(1)
    }
    #[cfg(not(feature = "parallel"))]
    {
        1
    }
}

/// `(0..n).map(f).collect()`, in parallel when the `parallel` feature is on.
pub fn map_range<T, F>(n: usize, f: F) -> Vec<T>
where
    T: Send,
    F: Fn(usize) -> T + Sync + Send,
{
    #[cfg(feature = "parallel")]
    {
        (0..n).into_par_iter().map(f).collect()
    }
    #[cfg(not(feature = "parallel"))]
    {
        (0..n).map(f).collect()
    }
}

/// Fold `f` over `0..n` starting from `identity()` and combine partial
/// accumulators with `reduce` (Rayon fold/reduce semantics: `identity` may
/// be called once per worker chunk, so it must return a neutral element).
pub fn fold_range<A, I, F, R>(n: usize, identity: I, f: F, reduce: R) -> A
where
    A: Send,
    I: Fn() -> A + Sync + Send,
    F: Fn(A, usize) -> A + Sync + Send,
    R: Fn(A, A) -> A + Sync + Send,
{
    #[cfg(feature = "parallel")]
    {
        (0..n)
            .into_par_iter()
            .fold(&identity, &f)
            .reduce(&identity, &reduce)
    }
    #[cfg(not(feature = "parallel"))]
    {
        let _ = &reduce;
        (0..n).fold(identity(), f)
    }
}

/// Call `f(chunk_index, chunk)` on consecutive `chunk_len`-sized pieces of
/// `data` (the last piece may be shorter), in parallel when enabled.
pub fn for_each_chunk_mut<T, F>(data: &mut [T], chunk_len: usize, f: F)
where
    T: Send,
    F: Fn(usize, &mut [T]) + Sync + Send,
{
    let chunk_len = chunk_len.max(1);
    #[cfg(feature = "parallel")]
    {
        data.par_chunks_mut(chunk_len)
            .enumerate()
            .for_each(|(i, c)| f(i, c));
    }
    #[cfg(not(feature = "parallel"))]
    {
        data.chunks_mut(chunk_len)
            .enumerate()
            .for_each(|(i, c)| f(i, c));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn map_range_preserves_order() {
        let v = map_range(100, |i| i * i);
        assert_eq!(v.len(), 100);
        for (i, x) in v.iter().enumerate() {
            assert_eq!(*x, i * i);
        }
    }

    #[test]
    fn fold_range_sums() {
        let s = fold_range(1000, || 0u64, |a, i| a + i as u64, |a, b| a + b);
        assert_eq!(s, 999 * 1000 / 2);
    }

    #[test]
    fn chunks_cover_everything() {
        let mut v = vec![0usize; 103];
        for_each_chunk_mut(&mut v, 10, |ci, c| {
            for (k, x) in c.iter_mut().enumerate() {
                *x = ci * 10 + k;
            }
        });
        for (i, x) in v.iter().enumerate() {
            assert_eq!(*x, i);
        }
    }
}
