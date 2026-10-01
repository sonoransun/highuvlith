//! Concrete light-source implementations beyond the original VUV excimer
//! and LPA-FEL models, plus the shared accelerator/strong-field physics
//! they are built on.
//!
//! Each source family lives in its own file and implements
//! [`crate::source::LithographySource`]; the type-erased
//! [`crate::source::SourceKind`] enum re-exports them for the PyO3/CLI/GUI
//! layers. `physics.rs` holds the pure formula layer (undulator resonance,
//! critical energy, HHG cutoff, ICS kinematics, ...) with fixture tests.

pub mod betatron;
pub mod dpp;
pub mod entangled;
pub mod hhg;
pub mod ics;
pub mod lpp;
pub mod physics;
pub mod smith_purcell;
pub mod ssmb;
pub mod sxrl;
pub mod synchrotron;
pub mod throughput;
pub mod xfel;
pub mod xray_tube;
