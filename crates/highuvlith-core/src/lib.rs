//! # highuvlith-core
//!
//! Physics engine for VUV/EUV/X-ray projection-lithography simulation. This
//! crate owns the full imaging-to-resist pipeline; the Python, CLI, and GUI
//! frontends are thin wrappers over the types defined here.
//!
//! ## Pipeline
//!
//! ```text
//!   Source ─▶ Optics ─▶ Mask ─▶ Aerial image ─▶ Thin film ─▶ Resist ─▶ Metrics
//!  (spectrum, (pupil   (complex  (Hopkins        (standing    (Dill +   (CD, NILS,
//!   pupil)     P(f))    trans.)   TCC / SOCS)     waves)        Mack)     contrast)
//! ```
//!
//! Each stage is a module: [`source`], [`optics`], [`mask`], [`aerial`],
//! [`thinfilm`], [`resist`], and [`metrics`], with [`process`] driving
//! dose/focus sweeps and the research modules ([`opc`], [`ilt`], [`dsa`], ...)
//! layered on top.
//!
//! ## Extension traits
//!
//! The pipeline is generic over three traits so new hardware slots in without
//! touching the imaging code:
//!
//! - [`source::LithographySource`] — light sources (VUV excimer, LPA-FEL, ...).
//! - [`optics::OpticalSystem`] — pupil functions (refractive, zone plate, ...).
//! - [`compute::ComputeBackend`] — FFT backend (CPU rayon; GPU planned).
//!
//! ## Conventions
//!
//! - Lengths in nm, energies in eV, doses in mJ/cm².
//! - Photon energy uses `hc = 1239.84193 eV·nm`, so `E_eV = hc / λ_nm`.
//! - Fallible constructors return [`error::Result`], whose error is
//!   [`error::LithographyError`].
//!
//! ## Example
//!
//! Build an F2 excimer source, refractive optics, and a line/space mask, then
//! compute an in-focus aerial image and check that it is modulated:
//!
//! ```
//! use highuvlith_core::aerial::AerialImageEngine;
//! use highuvlith_core::mask::Mask;
//! use highuvlith_core::metrics::image_contrast;
//! use highuvlith_core::optics::ProjectionOptics;
//! use highuvlith_core::source::VuvSource;
//! use highuvlith_core::types::GridConfig;
//!
//! # fn main() -> Result<(), highuvlith_core::error::LithographyError> {
//! let source = VuvSource::f2_laser(0.5)?;      // F2 excimer at 157.63 nm
//! let optics = ProjectionOptics::new(0.75)?;   // NA 0.75 refractive CaF2
//! let mask = Mask::line_space(65.0, 180.0)?;   // 65 nm lines on 180 nm pitch
//! let grid = GridConfig::new(128, 2.0)?;       // 128 x 128 pixels, 2 nm each
//!
//! let engine = AerialImageEngine::new(&source, &optics, grid, 16)?;
//! let image = engine.compute(&mask, 0.0);      // in-focus aerial image
//! assert!(image_contrast(&image.data) > 0.0);
//! # Ok(())
//! # }
//! ```

pub mod aerial;
pub mod compute;
pub mod deep_xray;
pub mod double_patterning;
pub mod dsa;
pub mod error;
pub mod grayscale;
pub mod ilt;
pub mod interference;
pub mod io;
pub mod mask;
pub mod materials;
pub mod math;
pub mod metrics;
pub mod mnsl;
pub mod opc;
pub mod optics;
pub mod process;
pub mod ptychography;
pub mod quantum;
pub mod resist;
pub mod source;
pub mod source_models;
pub mod stochastic;
pub mod thinfilm;
pub mod types;
pub mod volumetric;
