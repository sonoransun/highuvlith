use thiserror::Error;

#[derive(Debug, Error)]
pub enum LithographyError {
    #[error("invalid parameter: {name} = {value} ({reason})")]
    InvalidParameter {
        name: &'static str,
        value: f64,
        reason: &'static str,
    },

    #[error("grid size {0} must be a power of 2 for FFT")]
    GridSizeNotPowerOfTwo(usize),

    #[error("no diffraction orders pass the pupil (pitch too small or NA too low)")]
    NoDiffractionOrders,

    #[error(
        "pupil sampling too dense: {samples} in-pupil frequency samples exceeds the limit of {max}; \
         the TCC matrix would not fit in memory. Use a coarser grid (larger pixel_nm), a smaller \
         field (smaller grid size), or a lower NA / longer wavelength"
    )]
    PupilSamplingTooDense { samples: usize, max: usize },

    #[error("material not found: {0}")]
    MaterialNotFound(String),

    /// A tabulated optical constant was requested outside the wavelength
    /// range its data cover. Lookups never extrapolate silently; `hint` names
    /// the alternative (another key or the data range of a fallback table).
    #[error(
        "{material}: wavelength {} nm is outside its data range {}-{} nm; {hint}",
        fmt_nm(*.wavelength_nm),
        fmt_nm(.range_nm.0),
        fmt_nm(.range_nm.1)
    )]
    WavelengthOutOfRange {
        material: String,
        wavelength_nm: f64,
        /// Accepted wavelength range `(min, max)` in nm.
        range_nm: (f64, f64),
        hint: String,
    },

    #[error("TCC decomposition failed: {0}")]
    TccDecomposition(String),

    #[error("convergence failure after {iterations} iterations (residual: {residual:.2e})")]
    ConvergenceFailure { iterations: usize, residual: f64 },

    #[error("dimension mismatch: expected {expected}, got {got}")]
    DimensionMismatch { expected: String, got: String },

    #[error("numerical error: {0}")]
    NumericalError(String),

    #[error("I/O error: {0}")]
    IoError(#[from] std::io::Error),

    #[error("internal error: {0}")]
    InternalError(String),
}

/// Compact wavelength formatting for messages: at most 4 decimals, trailing
/// zeros dropped (`125`, `41.3281`, `0.0413`).
fn fmt_nm(x: f64) -> String {
    let s = format!("{x:.4}");
    let s = s.trim_end_matches('0').trim_end_matches('.');
    s.to_string()
}

pub type Result<T> = std::result::Result<T, LithographyError>;
