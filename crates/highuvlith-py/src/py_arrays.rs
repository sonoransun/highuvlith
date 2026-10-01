//! Zero-copy numpy views of Rust-owned result arrays.
//!
//! Result classes that hand large arrays to Python (`AerialImageResult`,
//! `VolumetricResult`, `HeightMapResult`) are `#[pyclass(frozen)]`: their
//! arrays are written once at construction and never mutated or
//! reallocated afterwards. Their getters return numpy arrays that point
//! straight into that Rust buffer (no copy) and keep the owning Python
//! object alive through numpy's `base` reference. The views are read-only,
//! so Python code cannot silently change a result that Rust methods such as
//! `image_contrast()` or `cd_at_z()` read; call `.copy()` for a writable
//! array.

use ndarray::{ArrayBase, Data, Dimension};
use numpy::{Element, PyArray, PyUntypedArrayMethods};
use pyo3::prelude::*;

/// A read-only numpy view of `array`, owned by the Python object `owner`.
///
/// Callers must pass the `Bound` of the frozen pyclass instance that owns
/// `array` (so the buffer stays valid while any view exists) — every call
/// site in this crate does exactly that from a `#[getter]` of a
/// `#[pyclass(frozen)]` class that has no mutating methods.
pub(crate) fn readonly_view<'py, T, S, D>(
    array: &ArrayBase<S, D>,
    owner: &Bound<'py, PyAny>,
) -> Bound<'py, PyArray<T, D>>
where
    T: Element,
    S: Data<Elem = T>,
    D: Dimension,
{
    // SAFETY: `owner` owns `array` and is frozen: the array is never
    // mutated, moved out of, or reallocated while the object lives, and the
    // numpy array holds a strong reference to `owner` as its base.
    let view = unsafe { PyArray::borrow_from_array(array, owner.clone()) };
    // SAFETY: the view was just created and no other reference to it
    // exists; clearing WRITEABLE only restricts what Python may do with it.
    unsafe {
        (*view.as_array_ptr()).flags &= !numpy::npyffi::NPY_ARRAY_WRITEABLE;
    }
    view
}
