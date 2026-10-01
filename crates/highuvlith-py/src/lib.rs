//! PyO3 bindings for the highuvlith lithography simulator (VUV → EUV →
//! X-ray).
//!
//! Wraps the `highuvlith-core` pipeline types as Python classes
//! (`SourceConfig`, `OpticsConfig`, `MaskConfig`, ...) exposed through the
//! native `highuvlith._native` module.
//!
//! - **GIL:** every compute-heavy call (engine construction, aerial images,
//!   sweeps, process windows, deep-layer, LIGA, Talbot, ILT/OPC/SRAF, LELE,
//!   DSA, stochastic LER, multilayer scans, N00N ideal-limit images) runs inside `py.allow_threads`, so
//!   other Python threads keep running and thread pools parallelize.
//! - **Arrays:** the large result arrays (`AerialImageResult.intensity`,
//!   `VolumetricResult.values`, `HeightMapResult.values`, the Talbot / LELE /
//!   DSA images) are read-only numpy views of the Rust buffers — no copy
//!   (see `py_arrays`); arrays computed by a function call are moved into
//!   numpy without copying (`into_pyarray`). Smaller accessors (coordinate
//!   axes, histories, depth-dose curves) return fresh copies.
//! - **Validation:** inputs pass through three layers — the Python wrappers,
//!   these PyO3 shims, and the Rust core — so bad parameters are rejected
//!   before any simulation work runs.

use pyo3::prelude::*;

mod py_arrays;
mod py_config;
mod py_mnsl;
mod py_optim;
mod py_quantum;
mod py_results;
mod py_simulation;
mod py_stochastic;
mod py_sweep;
mod py_vector;
mod py_volumetric;
mod py_xray;

use py_config::*;
use py_mnsl::*;
use py_results::*;
use py_simulation::*;
use py_sweep::*;
use py_vector::register_vector_module;
use py_volumetric::register_volumetric_module;

/// highuvlith native module: VUV → X-ray lithography simulation engine.
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

    // Vector (polarized) pupil model: VectorSettings + pupil-physics helpers
    register_vector_module(m)?;
    // X-ray materials / LIGA diffraction (edge profile, multilayer mirrors,
    // Henke optical constants)
    py_xray::register_xray_module(m)?;

    // Optimization / patterning module (ILT, OPC, SRAF, SADP/SAQP, LELE, DSA)
    py_optim::register_optim_module(m)?;

    // Stochastic LER/LWR and photon counting
    py_stochastic::register_stochastic_module(m)?;

    // Quantum (N-photon) research module: I^N absorption, N00N two-beam
    // fringes, flux budget (the λ/N ideal-limit image is an engine method)
    py_quantum::register_quantum_module(m)?;

    Ok(())
}
