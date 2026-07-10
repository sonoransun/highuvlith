//! PyO3 bindings for the highuvlith VUV/EUV lithography engine.
//!
//! Wraps the `highuvlith-core` pipeline types as Python classes
//! (`SourceConfig`, `OpticsConfig`, `MaskConfig`, ...) exposed through the
//! native `highuvlith._native` module. Numpy arrays cross the boundary
//! zero-copy, and long-running compute releases the GIL so batch sweeps
//! parallelize across threads. Inputs pass through three validation layers —
//! the Python wrappers, these PyO3 shims, and the Rust core — so bad
//! parameters are rejected before any simulation work runs.

use pyo3::prelude::*;

mod py_config;
mod py_mnsl;
mod py_results;
mod py_simulation;
mod py_sweep;
mod py_volumetric;

use py_config::*;
use py_mnsl::*;
use py_results::*;
use py_simulation::*;
use py_sweep::*;
use py_volumetric::register_volumetric_module;

/// highuvlith native module: VUV lithography simulation engine.
#[pymodule]
fn _native(m: &Bound<'_, PyModule>) -> PyResult<()> {
    // Configuration classes
    m.add_class::<PySourceConfig>()?;
    m.add_class::<PyOpticsConfig>()?;
    m.add_class::<PyMaskConfig>()?;
    m.add_class::<PyResistConfig>()?;
    m.add_class::<PyFilmStackConfig>()?;
    m.add_class::<PyProcessConfig>()?;
    m.add_class::<PyGridConfig>()?;

    // Simulation engine
    m.add_class::<PySimulationEngine>()?;

    // Results
    m.add_class::<PyAerialImageResult>()?;
    m.add_class::<PyResistProfileResult>()?;

    // Batch / sweep
    m.add_class::<PyBatchSimulator>()?;
    m.add_class::<PyProcessWindowResult>()?;

    // MNSL module
    register_mnsl_module(m)?;

    // Volumetric / deep-layer module (volumetric, LIGA, grayscale,
    // interference, quantum aerial image)
    register_volumetric_module(m)?;

    Ok(())
}
