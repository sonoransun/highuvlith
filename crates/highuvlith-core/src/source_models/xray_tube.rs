//! Hard X-ray tube: thick-target bremsstrahlung plus anode characteristic
//! lines, for lab-scale LIGA / deep-resist exposure without a synchrotron.
//!
//! An electron beam accelerated through the tube voltage `V` (so every
//! electron arrives with kinetic energy `E0 = eV`, numerically `kVp` in keV)
//! stops in a thick metal anode. About 1% of the beam power leaves as X-rays:
//! a smooth bremsstrahlung continuum up to the Duane–Hunt limit `E0`, plus
//! the anode's characteristic K/L lines once `E0` exceeds the matching shell
//! binding energy. The rest heats the anode — tubes are thermally limited to
//! the kW class (sealed tubes) or ~10 kW (rotating anodes).
//!
//! The output is **broadband** (Δλ/λ ~ 1), spatially incoherent, and hard
//! (0.02–1 nm): this family is **not a projection-imaging source**. Its
//! consumer is 1:1 proximity shadow printing — LIGA deep X-ray lithography via
//! [`crate::deep_xray`]. [`XrayTubeSource::xray_spectrum`] returns the
//! *relative* spectrum (`XraySpectrum::Tabulated`), which is what the CLI
//! `highuvlith deep` LIGA mode uses for a `type = "xray_tube"` source (depth
//! dose and dose ratios, no exposure time).
//! [`XrayTubeSource::spectral_flux_density`] returns the *absolute* photons
//! s⁻¹ mm⁻² keV⁻¹ at a chosen distance; it is exposed but **not consumed
//! automatically** — hand it to the LIGA module yourself (CLI `[deep]
//! flux_density` table, Python `simulate_liga(flux_density=...)`, or
//! `XraySpectrum::from_flux_density`) to get an absolute exposure time.
//!
//! # Key equations
//!
//! - Kramers' law (thick target), photon-number spectrum per electron:
//!   `dn/dE = (2 eta / E0) (E0/E - 1)`, `0 < E < E0` [photons keV⁻¹],
//!   normalized so that `Int_0^E0 E dn/dE dE = eta E0` (the energy spectrum
//!   `E dn/dE ∝ Z (E0 - E)` is Kramers' original form).
//! - Empirical conversion efficiency: `eta = 1.1e-9 Z V[V]`
//!   (bremsstrahlung power / beam power; textbook values of the constant differ
//!   by ~20%).
//! - Absolute photon rate: `dN/dt dE = (I / e) dn/dE`; bin integral
//!   `Int_a^b (E0/E - 1) dE = E0 ln(b/a) - (b - a)`.
//! - Duane–Hunt limit: `lambda_min[nm] = 1.23984193 / kVp`.
//! - Characteristic lines (EMPIRICAL): photons per electron in a line of
//!   series s, `n_line = k M_s omega_s (r_line / Sum_s r) (U - 1)^1.67`, with
//!   overvoltage `U = E0 / E_edge` (zero for `U <= 1`), `k = 7e-4`, fluorescence
//!   yield `omega_s`, L/K multiplicity `M_L = 2`, `M_K = 1`, and relative line
//!   intensities `r` within the series.
//! - Inherent filtration: `T(E) = exp(-mu_Be(E) t)` through the Be exit
//!   window (NIST mass-attenuation data via
//!   [`crate::materials::attenuation::Compound::beryllium`], tabulated over the
//!   whole 1 keV – kVp grid).
//! - Flux density at distance `d` (isotropic point source, vacuum path):
//!   `Phi(E) = (dN/dt dE) T(E) / (4 pi d^2)` [photons s⁻¹ mm⁻² keV⁻¹].
//!
//! # Model status
//!
//! Implemented (✅): the characteristic line energies and their excitation
//! edges (standard tables, below) with the overvoltage gating of every line,
//! the Duane–Hunt cutoff of the continuum, and the Be-window filtration
//! (NIST data) are computed and fixture-tested.
//!
//! Simplified (🔶): the continuum *shape* and everything derived from it —
//! the spectral moments (mean photon energy and wavelength, bandwidth,
//! characteristic-photon fraction, photons per unit dose) and the absolute
//! flux. The continuum is Kramers' thick-target approximation, and the model
//! omits **anode self-absorption / the heel effect**: photons generated below
//! the anode surface are attenuated in the anode on their way out, which in a
//! real tube removes much of the soft continuum, so real spectra are harder
//! than the unabsorbed moments computed here (by an amount that depends on
//! the take-off angle and is not quantified). Electron backscatter losses
//! (tens of percent of the electrons leave a high-Z anode), the focal-spot
//! size, off-axis intensity variation, air attenuation, and relativistic
//! corrections (Kramers is a diagnostic-range, ≲150 kV law) are not modelled
//! either. The absolute scale further rests on the empirical
//! `eta = 1.1e-9 Z V` efficiency and assumes isotropic emission into 4π from a
//! point focus (per-steradian intensity `P / 4 pi`). The line strengths are an
//! **explicitly empirical** thick-target law: the exponent 1.6–1.7 is the
//! long-standing electron-probe-microanalysis form, and the normalization
//! `k = 7e-4` was chosen to agree within about a factor of two with a
//! non-relativistic Bethe cross-section / Bethe-stopping thick-target
//! estimate for Cu, Mo and Rh K lines (that estimate is about 2x lower for W
//! K lines near threshold): absolute K-line intensities are uncertain by ±x2
//! (a factor of two), W L-line intensities by ±x3. Mo and Rh L lines
//! (2.3 / 2.7 keV; weak, ω_L ≲ 0.1, and strongly absorbed by the Be window)
//! and Cu L lines (0.93 keV, fully absorbed) are omitted.
//!
//! Imaging: [`LithographySource::wavelength_nm`] reports the photon-number
//! weighted **mean photon wavelength** of the filtered spectrum and
//! `bandwidth_pm` a Gaussian-equivalent FWHM (2.3548 × rms wavelength
//! spread) — bookkeeping values only; the per-sample-focus polychromatic
//! imaging path is not valid for Δλ/λ ~ 1. `photon_energy_ev` and
//! `photon_density_per_mj_cm2` are overridden to use the mean photon *energy*
//! (the trait defaults, `hc / wavelength`, would use the harmonic-mean energy
//! and overcount photons per unit dose by ~1.6 for the W 60 kV preset).
//!
//! # References
//!
//! - H. A. Kramers, "On the theory of X-ray absorption and of the continuous
//!   X-ray spectrum," Phil. Mag. 46, 836 (1923) — the continuum law.
//! - Line energies and edges: X-ray Data Booklet (Center for X-ray Optics,
//!   LBNL), Tables 1-1 (electron binding energies) and 1-2 (emission-line
//!   energies), which compile J. A. Bearden, Rev. Mod. Phys. 39, 78 (1967) and
//!   J. A. Bearden & A. F. Burr, Rev. Mod. Phys. 39, 125 (1967).
//! - K-shell fluorescence yields: M. O. Krause, J. Phys. Chem. Ref. Data 8,
//!   307 (1979).
//! - Be attenuation: NIST X-ray mass attenuation coefficients (J. H. Hubbell
//!   and S. M. Seltzer), as tabulated in [`crate::materials::attenuation`].

use serde::{Deserialize, Serialize};

use super::physics::{ELECTRON_CHARGE_C, HC_EV_NM};
use crate::deep_xray::XraySpectrum;
use crate::materials::attenuation::Compound;
use crate::source::{evaluate_illumination, DerivedQuantity, IlluminationShape, LithographySource};

/// `hc` in keV·nm: `E[keV] = HC_KEV_NM / lambda[nm]`.
const HC_KEV_NM: f64 = HC_EV_NM * 1e-3;
/// Joules per keV.
const J_PER_KEV: f64 = ELECTRON_CHARGE_C * 1e3;
/// FWHM of a Gaussian in units of its standard deviation, `2 sqrt(2 ln 2)`.
const FWHM_PER_SIGMA: f64 = 2.354_820_045_030_949;

/// Empirical thick-target bremsstrahlung efficiency constant per volt:
/// `eta = BREMSSTRAHLUNG_EFFICIENCY_PER_ZV * Z * V[V]`.
pub const BREMSSTRAHLUNG_EFFICIENCY_PER_ZV: f64 = 1.1e-9;
/// Lower edge of the spectral grid in keV. Photons below 1 keV do not
/// survive any practical Be window (T < 1e-4 for 100 µm at 1 keV).
pub const SPECTRUM_MIN_KEV: f64 = 1.0;
/// Reference distance (mm) for the flux-density derived quantities.
pub const REFERENCE_DISTANCE_MM: f64 = 100.0;
/// Bulk density of the Be exit window in g/cm³.
const BE_DENSITY_G_CM3: f64 = 1.85;
/// EMPIRICAL characteristic-yield normalization: photons per electron per
/// unit fluorescence yield at `U - 1 = 1` (see the module "Model status").
const CHARACTERISTIC_YIELD_K: f64 = 7.0e-4;
/// EMPIRICAL thick-target overvoltage exponent in `n ∝ (U - 1)^n`.
const CHARACTERISTIC_EXPONENT: f64 = 1.67;
/// L-shell / K-shell ionization multiplicity at equal overvoltage (Bethe-type
/// estimate: 8 vs 2 electrons, partly offset by the smaller per-electron
/// cross section and log factor). Uncertain to ~x3.
const L_SHELL_MULTIPLICITY: f64 = 2.0;
/// Number of bins used internally for the trait-level spectral moments.
const MOMENT_BINS: usize = 400;

/// Anode (target) element; fixes the atomic number, the characteristic
/// lines, and their excitation edges.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum XrayAnode {
    /// Tungsten (Z = 74): the general-purpose / high-power anode. L lines at
    /// 8.4–11.3 keV above 10.2 kV; K lines at 58–67 keV only above 69.5 kV.
    W,
    /// Molybdenum (Z = 42): K lines at 17.4–19.6 keV above 20.0 kV.
    Mo,
    /// Copper (Z = 29): K lines at 8.0–8.9 keV above 8.98 kV (the XRD anode).
    Cu,
    /// Rhodium (Z = 45): K lines at 20.1–22.7 keV above 23.2 kV (XRF anode).
    Rh,
}

/// Shell series a characteristic line belongs to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LineSeries {
    /// K series (vacancy in the 1s shell).
    K,
    /// L series (vacancy in a 2s/2p subshell).
    L,
}

/// One characteristic emission line of an anode element.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CharacteristicLine {
    /// Siegbahn label, e.g. `"Ka1"`.
    pub label: &'static str,
    /// Series the line belongs to.
    pub series: LineSeries,
    /// Line (photon) energy in keV.
    pub energy_kev: f64,
    /// Binding energy in keV of the shell/subshell whose vacancy the line
    /// fills — the excitation threshold (the line is absent for `kVp` below
    /// it).
    pub edge_kev: f64,
    /// Relative intensity within its series at high overvoltage
    /// (approximate: Ka2/Ka1 ≈ 0.5–0.6, Kb/Ka rising with Z from ≈0.13 (Cu)
    /// to ≈0.3 (W); W L ratios ±30%).
    pub relative_intensity: f64,
}

const fn line(
    label: &'static str,
    series: LineSeries,
    energy_kev: f64,
    edge_kev: f64,
    relative_intensity: f64,
) -> CharacteristicLine {
    CharacteristicLine {
        label,
        series,
        energy_kev,
        edge_kev,
        relative_intensity,
    }
}

// Line energies and edges: X-ray Data Booklet Tables 1-1/1-2 (eV -> keV).
// The Kb group is represented by ONE line at the Kb1 energy carrying the whole
// Kb-group intensity (the Kb relative intensities below are Kb-group / Ka1
// ratios). Kb3 lies a few tens of eV below Kb1 for Mo and Rh (unresolved for
// Cu) but ~0.29 keV below it for W, and Kb2 lies above Kb1 (~1.8 keV above for
// W); the misplacement is negligible for the moments and the LIGA dose (the W K
// lines carry <1% of the photons even at 100 kV).
const W_LINES: [CharacteristicLine; 8] = [
    line("Ka1", LineSeries::K, 59.31824, 69.525, 1.0),
    line("Ka2", LineSeries::K, 57.9817, 69.525, 0.58),
    line("Kb1", LineSeries::K, 67.2443, 69.525, 0.47),
    // L3 edge 10.207 keV, L2 edge 11.544 keV.
    line("La1", LineSeries::L, 8.3976, 10.207, 1.0),
    line("La2", LineSeries::L, 8.3352, 10.207, 0.11),
    line("Lb1", LineSeries::L, 9.67235, 11.544, 0.55),
    line("Lb2", LineSeries::L, 9.9615, 10.207, 0.20),
    line("Lg1", LineSeries::L, 11.2859, 11.544, 0.10),
];
const MO_LINES: [CharacteristicLine; 3] = [
    line("Ka1", LineSeries::K, 17.47934, 20.000, 1.0),
    line("Ka2", LineSeries::K, 17.3743, 20.000, 0.52),
    line("Kb1", LineSeries::K, 19.6083, 20.000, 0.29),
];
const CU_LINES: [CharacteristicLine; 3] = [
    line("Ka1", LineSeries::K, 8.04778, 8.979, 1.0),
    line("Ka2", LineSeries::K, 8.02783, 8.979, 0.51),
    line("Kb1", LineSeries::K, 8.90529, 8.979, 0.20),
];
const RH_LINES: [CharacteristicLine; 3] = [
    line("Ka1", LineSeries::K, 20.2161, 23.220, 1.0),
    line("Ka2", LineSeries::K, 20.0737, 23.220, 0.53),
    line("Kb1", LineSeries::K, 22.7236, 23.220, 0.31),
];

impl XrayAnode {
    /// Parse an anode symbol or name (case-insensitive): `"w"`/`"tungsten"`,
    /// `"mo"`/`"molybdenum"`, `"cu"`/`"copper"`, `"rh"`/`"rhodium"`.
    pub fn from_name(name: &str) -> Option<Self> {
        match name.to_ascii_lowercase().as_str() {
            "w" | "tungsten" => Some(Self::W),
            "mo" | "molybdenum" => Some(Self::Mo),
            "cu" | "copper" => Some(Self::Cu),
            "rh" | "rhodium" => Some(Self::Rh),
            _ => None,
        }
    }

    /// Chemical symbol.
    pub fn symbol(self) -> &'static str {
        match self {
            Self::W => "W",
            Self::Mo => "Mo",
            Self::Cu => "Cu",
            Self::Rh => "Rh",
        }
    }

    /// Atomic number Z (enters Kramers' law and the efficiency).
    pub fn atomic_number(self) -> u32 {
        match self {
            Self::W => 74,
            Self::Mo => 42,
            Self::Cu => 29,
            Self::Rh => 45,
        }
    }

    /// K-shell binding energy (K absorption edge) in keV.
    pub fn k_edge_kev(self) -> f64 {
        match self {
            Self::W => 69.525,
            Self::Mo => 20.000,
            Self::Cu => 8.979,
            Self::Rh => 23.220,
        }
    }

    /// Tabulated characteristic lines modelled for this anode.
    pub fn characteristic_lines(self) -> &'static [CharacteristicLine] {
        match self {
            Self::W => &W_LINES,
            Self::Mo => &MO_LINES,
            Self::Cu => &CU_LINES,
            Self::Rh => &RH_LINES,
        }
    }

    /// Fluorescence yield of the series (K: Krause 1979, 2 significant
    /// digits; W L3: ≈0.26). Series not modelled for an anode return 0.
    fn fluorescence_yield(self, series: LineSeries) -> f64 {
        match (self, series) {
            (Self::W, LineSeries::K) => 0.96,
            (Self::W, LineSeries::L) => 0.26,
            (Self::Mo, LineSeries::K) => 0.76,
            (Self::Cu, LineSeries::K) => 0.44,
            (Self::Rh, LineSeries::K) => 0.81,
            _ => 0.0,
        }
    }
}

/// Hard X-ray tube source (Kramers continuum + characteristic lines through
/// a Be exit window).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct XrayTubeSource {
    /// Anode element.
    pub anode: XrayAnode,
    /// Tube (peak) voltage in kV; numerically equal to the maximum photon
    /// energy `E0` in keV (Duane–Hunt limit).
    pub kvp: f64,
    /// Tube (beam) current in mA; the absolute photon rate is ∝ current.
    pub current_ma: f64,
    /// Be exit-window thickness in µm (inherent filtration; 0 = windowless).
    pub be_window_um: f64,
    /// Number of spectral bins returned by `spectral_weights()`.
    pub spectral_samples: usize,
    /// Pupil shape (only meaningful if forced through the projection
    /// pipeline; the tube is an incoherent, extended source).
    pub illumination: IlluminationShape,
}

/// Photon-weighted spectral moments of the filtered spectrum.
struct SpectrumMoments {
    photons_per_s: f64,
    line_photons_per_s: f64,
    energy_kev_per_s: f64,
    mean_wavelength_nm: f64,
    rms_wavelength_nm: f64,
}

impl XrayTubeSource {
    /// Tube with the given anode, voltage (kV) and current (mA) and a 250 µm
    /// Be window (typical sealed-tube windows are a few hundred µm).
    pub fn new(anode: XrayAnode, kvp: f64, current_ma: f64) -> crate::error::Result<Self> {
        let src = Self {
            anode,
            kvp,
            current_ma,
            be_window_um: 250.0,
            spectral_samples: 16,
            illumination: IlluminationShape::Conventional { sigma: 1.0 },
        };
        src.validate()?;
        Ok(src)
    }

    /// Sealed-tube Cu anode at 40 kV / 40 mA (1.6 kW): the XRD-class
    /// operating point; Cu Kα (8.05 keV) dominates the spectrum.
    pub fn cu_40kv() -> crate::error::Result<Self> {
        Self::new(XrayAnode::Cu, 40.0, 40.0)
    }

    /// Mo anode at 50 kV / 40 mA (2 kW): continuum plus Mo K lines at
    /// 17.5 keV.
    pub fn mo_50kv() -> crate::error::Result<Self> {
        Self::new(XrayAnode::Mo, 50.0, 40.0)
    }

    /// W anode at 60 kV / 30 mA (1.8 kW): a lab-LIGA design point — hard
    /// continuum (unabsorbed-anode mean ≈ 13 keV after 250 µm Be) plus W L
    /// lines, no K lines (below the 69.5 keV K edge). Illustrative, not a
    /// specific product.
    pub fn w_60kv() -> crate::error::Result<Self> {
        Self::new(XrayAnode::W, 60.0, 30.0)
    }

    /// Rh anode at 50 kV / 40 mA (2 kW): XRF-class operating point with the
    /// Rh K lines at 20.2–22.7 keV. Illustrative.
    pub fn rh_50kv() -> crate::error::Result<Self> {
        Self::new(XrayAnode::Rh, 50.0, 40.0)
    }

    /// The operating-point preset of an anode (W 60 kV/30 mA, Mo 50/40,
    /// Cu 40/40, Rh 50/40) — the defaults used by the CLI and Python
    /// frontends when `kvp` / current are not given.
    pub fn preset(anode: XrayAnode) -> crate::error::Result<Self> {
        match anode {
            XrayAnode::W => Self::w_60kv(),
            XrayAnode::Mo => Self::mo_50kv(),
            XrayAnode::Cu => Self::cu_40kv(),
            XrayAnode::Rh => Self::rh_50kv(),
        }
    }

    /// Check every field; called by the constructors and available to
    /// frontends that override fields after construction.
    pub fn validate(&self) -> crate::error::Result<()> {
        if !(self.kvp.is_finite() && (5.0..=300.0).contains(&self.kvp)) {
            return Err(crate::error::LithographyError::InvalidParameter {
                name: "kvp",
                value: self.kvp,
                reason: "tube voltage must be in [5, 300] kV (Kramers' thick-target law; \
                         the spectral grid starts at 1 keV)",
            });
        }
        if !(self.current_ma.is_finite() && self.current_ma > 0.0) {
            return Err(crate::error::LithographyError::InvalidParameter {
                name: "current_ma",
                value: self.current_ma,
                reason: "must be positive",
            });
        }
        if !(self.be_window_um.is_finite() && self.be_window_um >= 0.0) {
            return Err(crate::error::LithographyError::InvalidParameter {
                name: "be_window_um",
                value: self.be_window_um,
                reason: "must be >= 0",
            });
        }
        if self.spectral_samples == 0 {
            return Err(crate::error::LithographyError::InvalidParameter {
                name: "spectral_samples",
                value: 0.0,
                reason: "must be >= 1",
            });
        }
        Ok(())
    }

    /// Electrons per second hitting the anode, `I / e`.
    pub fn electrons_per_s(&self) -> f64 {
        self.current_ma * 1e-3 / ELECTRON_CHARGE_C
    }

    /// Electron-beam power on the anode in W, `V I`.
    pub fn beam_power_w(&self) -> f64 {
        self.kvp * 1e3 * self.current_ma * 1e-3
    }

    /// Bremsstrahlung conversion efficiency `eta = 1.1e-9 Z V[V]` (empirical).
    pub fn bremsstrahlung_efficiency(&self) -> f64 {
        BREMSSTRAHLUNG_EFFICIENCY_PER_ZV * self.anode.atomic_number() as f64 * self.kvp * 1e3
    }

    /// Generated bremsstrahlung power in W (4π, before the window):
    /// `eta V I`.
    pub fn bremsstrahlung_power_w(&self) -> f64 {
        self.bremsstrahlung_efficiency() * self.beam_power_w()
    }

    /// Generated continuum photons per second (4π, before the window) with
    /// energies in `[e_lo_kev, e_hi_kev]` (clamped to `(0, kVp]`), from the
    /// exact Kramers bin integral.
    pub fn continuum_photons_per_s(&self, e_lo_kev: f64, e_hi_kev: f64) -> f64 {
        let e0 = self.kvp;
        let a = e_lo_kev.max(1e-9);
        let b = e_hi_kev.min(e0);
        if b <= a {
            return 0.0;
        }
        let per_electron =
            2.0 * self.bremsstrahlung_efficiency() / e0 * (e0 * (b / a).ln() - (b - a));
        self.electrons_per_s() * per_electron
    }

    /// Characteristic photons per electron generated in `line` (4π, before
    /// the window) at this tube voltage; zero below the line's edge.
    pub fn line_photons_per_electron(&self, line: &CharacteristicLine) -> f64 {
        let u = self.kvp / line.edge_kev;
        if u <= 1.0 {
            return 0.0;
        }
        let lines = self.anode.characteristic_lines();
        let series_sum: f64 = lines
            .iter()
            .filter(|l| l.series == line.series)
            .map(|l| l.relative_intensity)
            .sum();
        let multiplicity = match line.series {
            LineSeries::K => 1.0,
            LineSeries::L => L_SHELL_MULTIPLICITY,
        };
        CHARACTERISTIC_YIELD_K
            * multiplicity
            * self.anode.fluorescence_yield(line.series)
            * (line.relative_intensity / series_sum)
            * (u - 1.0).powf(CHARACTERISTIC_EXPONENT)
    }

    /// Excited characteristic lines with their photon rate after the window
    /// (photons/s, 4π). Lines below their excitation edge are omitted.
    pub fn line_photon_rates(&self) -> Vec<(CharacteristicLine, f64)> {
        let ne = self.electrons_per_s();
        self.anode
            .characteristic_lines()
            .iter()
            .filter_map(|l| {
                let n = self.line_photons_per_electron(l);
                (n > 0.0).then(|| (*l, ne * n * self.window_transmission(l.energy_kev)))
            })
            .collect()
    }

    /// Be exit-window intensity transmission `exp(-mu_Be(E) t)`.
    pub fn window_transmission(&self, energy_kev: f64) -> f64 {
        if self.be_window_um <= 0.0 {
            return 1.0;
        }
        let be = Compound::beryllium(BE_DENSITY_G_CM3);
        (-be.mu_per_um(energy_kev) * self.be_window_um).exp()
    }

    /// Filtered photon rate (photons/s, 4π) binned on `n_bins` equal-width
    /// energy bins over `[SPECTRUM_MIN_KEV, kVp]`, as `(bin-center keV,
    /// photons/s)`. The continuum uses the exact Kramers bin integral times
    /// the window transmission at the bin center; each line is deposited in
    /// the bin containing its energy (natural widths ~eV ≪ bin width).
    pub fn binned_photon_rate(&self, n_bins: usize) -> Vec<(f64, f64)> {
        let n = n_bins.max(1);
        let lo = SPECTRUM_MIN_KEV;
        let de = (self.kvp - lo) / n as f64;
        let mut bins: Vec<(f64, f64)> = (0..n)
            .map(|j| {
                let a = lo + j as f64 * de;
                let b = a + de;
                let center = 0.5 * (a + b);
                let photons = self.continuum_photons_per_s(a, b) * self.window_transmission(center);
                (center, photons)
            })
            .collect();
        for (l, rate) in self.line_photon_rates() {
            let j = (((l.energy_kev - lo) / de).floor().max(0.0) as usize).min(n - 1);
            bins[j].1 += rate;
        }
        bins
    }

    /// Absolute spectral photon flux density at `distance_mm` from the focal
    /// spot: `(E_keV, photons s⁻¹ mm⁻² keV⁻¹)` at the centres of `n_bins`
    /// equal-width bins over `[1 keV, kVp]` (contract C5). Assumes isotropic
    /// emission (per-sr intensity = 4π total / 4π), a point focus, and a
    /// vacuum (or He) beam path; includes only the tube's own Be window.
    /// Exposed but **not consumed automatically**: the CLI LIGA mode uses the
    /// relative [`Self::xray_spectrum`]; for an absolute exposure time pass
    /// this table to `XraySpectrum::from_flux_density` (CLI `[deep]
    /// flux_density`, Python `simulate_liga(flux_density=...)`), whose
    /// equal-spacing (histogram) semantics it matches. Returns an empty vector
    /// for a non-positive distance.
    pub fn spectral_flux_density(&self, distance_mm: f64, n_bins: usize) -> Vec<(f64, f64)> {
        if !(distance_mm.is_finite() && distance_mm > 0.0) {
            return Vec::new();
        }
        let n = n_bins.max(1);
        let de = (self.kvp - SPECTRUM_MIN_KEV) / n as f64;
        let area_mm2 = 4.0 * std::f64::consts::PI * distance_mm * distance_mm;
        self.binned_photon_rate(n)
            .into_iter()
            .map(|(e, rate)| (e, rate / (de * area_mm2)))
            .collect()
    }

    /// Relative spectrum for the LIGA module — contract C5:
    /// `XraySpectrum::Tabulated` on `n_bins` equal-width bins over
    /// `[1 keV, kVp]`, `relative_flux` = filtered photons per bin, normalized
    /// to sum to 1 (equivalently ∝ dN/dE on the uniform grid). This is what
    /// the CLI `highuvlith deep` LIGA mode uses for a `type = "xray_tube"`
    /// source (relative: dose ratios, no exposure time). The LIGA sampler's
    /// `default_energy_range_kev` spans the table's own min/max within the
    /// 0.03 keV – 20 MeV attenuation-data range, so the whole spectrum up to
    /// kVp reaches the depth-dose model.
    pub fn xray_spectrum(&self, n_bins: usize) -> XraySpectrum {
        let bins = self.binned_photon_rate(n_bins);
        let total: f64 = bins.iter().map(|(_, r)| r).sum();
        let scale = if total > 0.0 { 1.0 / total } else { 0.0 };
        XraySpectrum::Tabulated {
            energies_kev: bins.iter().map(|(e, _)| *e).collect(),
            relative_flux: bins.iter().map(|(_, r)| r * scale).collect(),
        }
    }

    /// Photon-weighted moments of the filtered spectrum (continuum on
    /// `MOMENT_BINS` bins at bin-center wavelengths, lines exact).
    fn moments(&self) -> SpectrumMoments {
        let lo = SPECTRUM_MIN_KEV;
        let de = (self.kvp - lo) / MOMENT_BINS as f64;
        let (mut n, mut ne, mut nl, mut nl2) = (0.0, 0.0, 0.0, 0.0);
        for j in 0..MOMENT_BINS {
            let a = lo + j as f64 * de;
            let center = a + 0.5 * de;
            let rate = self.continuum_photons_per_s(a, a + de) * self.window_transmission(center);
            let lambda = HC_KEV_NM / center;
            n += rate;
            ne += rate * center;
            nl += rate * lambda;
            nl2 += rate * lambda * lambda;
        }
        let mut n_lines = 0.0;
        for (l, rate) in self.line_photon_rates() {
            let lambda = HC_KEV_NM / l.energy_kev;
            n += rate;
            n_lines += rate;
            ne += rate * l.energy_kev;
            nl += rate * lambda;
            nl2 += rate * lambda * lambda;
        }
        let mean = if n > 0.0 { nl / n } else { 0.0 };
        let var = if n > 0.0 {
            (nl2 / n - mean * mean).max(0.0)
        } else {
            0.0
        };
        SpectrumMoments {
            photons_per_s: n,
            line_photons_per_s: n_lines,
            energy_kev_per_s: ne,
            mean_wavelength_nm: mean,
            rms_wavelength_nm: var.sqrt(),
        }
    }

    /// Filtered photon rate (photons/s, 4π, E ≥ 1 keV).
    pub fn photon_rate(&self) -> f64 {
        self.moments().photons_per_s
    }

    /// Filtered X-ray power in W (continuum + lines, 4π-equivalent).
    pub fn xray_power_w(&self) -> f64 {
        self.moments().energy_kev_per_s * J_PER_KEV
    }

    /// Fraction of filtered photons in characteristic lines.
    pub fn characteristic_fraction(&self) -> f64 {
        let m = self.moments();
        if m.photons_per_s > 0.0 {
            m.line_photons_per_s / m.photons_per_s
        } else {
            0.0
        }
    }

    /// Photon-weighted mean photon energy in keV of the filtered spectrum.
    pub fn mean_photon_energy_kev(&self) -> f64 {
        let m = self.moments();
        if m.photons_per_s > 0.0 {
            m.energy_kev_per_s / m.photons_per_s
        } else {
            0.0
        }
    }

    /// Photon-weighted mean photon wavelength in nm (the trait wavelength).
    pub fn mean_photon_wavelength_nm(&self) -> f64 {
        self.moments().mean_wavelength_nm
    }

    /// Duane–Hunt short-wavelength limit `hc / (e V)` in nm.
    pub fn duane_hunt_wavelength_nm(&self) -> f64 {
        HC_KEV_NM / self.kvp
    }
}

impl Default for XrayTubeSource {
    fn default() -> Self {
        Self::w_60kv().expect("W 60 kV preset is valid")
    }
}

impl LithographySource for XrayTubeSource {
    /// DERIVED: photon-number-weighted mean wavelength of the filtered
    /// spectrum (bookkeeping value for a Δλ/λ ~ 1 source).
    fn wavelength_nm(&self) -> f64 {
        self.mean_photon_wavelength_nm()
    }

    /// Gaussian-equivalent FWHM: 2.3548 × the photon-weighted rms wavelength
    /// spread (a broadband continuum has no true FWHM).
    fn bandwidth_pm(&self) -> f64 {
        FWHM_PER_SIGMA * self.moments().rms_wavelength_nm * 1e3
    }

    /// Photon-number-weighted **mean photon energy** of the filtered spectrum
    /// in eV (not `hc / wavelength_nm()`, which for this broadband source is
    /// the harmonic-mean energy, ≈1.6x lower for the W 60 kV preset).
    fn photon_energy_ev(&self) -> f64 {
        self.mean_photon_energy_kev() * 1e3
    }

    /// Photons per nm² at 1 mJ/cm² from the mean photon energy
    /// (`dose / <E>`), consistent with [`Self::photon_energy_ev`]; the trait
    /// default (`hc / <lambda>`) would overcount photons per unit dose by
    /// `<E><1/E>`.
    fn photon_density_per_mj_cm2(&self) -> f64 {
        let e_photon_j = self.photon_energy_ev() * ELECTRON_CHARGE_C;
        // 1 mJ/cm^2 = 10 J/m^2; photons/m^2 -> photons/nm^2
        10.0 / e_photon_j * 1e-18
    }

    fn intensity_at(&self, fx_norm: f64, fy_norm: f64) -> f64 {
        evaluate_illumination(&self.illumination, fx_norm, fy_norm)
    }

    /// The filtered photon spectrum on `spectral_samples` equal-energy bins,
    /// as `(bin-center wavelength, photon fraction)` — spectral bookkeeping,
    /// not a valid polychromatic-imaging sampling.
    fn spectral_weights(&self) -> Vec<(f64, f64)> {
        let bins = self.binned_photon_rate(self.spectral_samples);
        let total: f64 = bins.iter().map(|(_, r)| r).sum();
        bins.into_iter()
            .rev()
            .map(|(e, r)| (HC_KEV_NM / e, if total > 0.0 { r / total } else { 0.0 }))
            .collect()
    }

    /// Continuous-wave tube: the filtered X-ray power (4π-equivalent).
    fn average_power_w(&self) -> Option<f64> {
        Some(self.xray_power_w())
    }

    // Incoherent, CW: keep the trait defaults (no pulse metadata,
    // transverse_coherence 0, shot_to_shot_rms 0).

    fn derived_quantities(&self) -> Vec<DerivedQuantity> {
        let m = self.moments();
        let power_w = m.energy_kev_per_s * J_PER_KEV;
        let d = REFERENCE_DISTANCE_MM;
        let sphere_mm2 = 4.0 * std::f64::consts::PI * d * d;
        let mut out = vec![
            DerivedQuantity::new(
                "beam_power_w",
                self.beam_power_w(),
                "W",
                "electron-beam power V*I on the anode",
            ),
            DerivedQuantity::new(
                "bremsstrahlung_efficiency",
                self.bremsstrahlung_efficiency(),
                "-",
                "empirical eta = 1.1e-9 Z V[V] (thick target, +-~20%)",
            ),
            DerivedQuantity::new(
                "bremsstrahlung_power_w",
                self.bremsstrahlung_power_w(),
                "W",
                "generated continuum eta*V*I, 4pi, before the Be window",
            ),
            DerivedQuantity::new(
                "xray_power_w",
                power_w,
                "W",
                "continuum + lines after the Be window, E >= 1 keV, 4pi-equivalent",
            ),
            DerivedQuantity::new(
                "anode_heat_load_w",
                (self.beam_power_w() - self.bremsstrahlung_power_w()).max(0.0),
                "W",
                "beam power not converted to bremsstrahlung (~99%): the thermal limit",
            ),
            DerivedQuantity::new(
                "photon_rate",
                m.photons_per_s,
                "photons/s",
                "after the Be window, E >= 1 keV, 4pi (isotropic assumption)",
            ),
            DerivedQuantity::new(
                "characteristic_photon_fraction",
                if m.photons_per_s > 0.0 {
                    m.line_photons_per_s / m.photons_per_s
                } else {
                    0.0
                },
                "-",
                "EMPIRICAL line law k*omega*(U-1)^1.67, k = 7e-4 (+-x2; W L lines +-x3)",
            ),
            DerivedQuantity::new(
                "mean_photon_energy_kev",
                if m.photons_per_s > 0.0 {
                    m.energy_kev_per_s / m.photons_per_s
                } else {
                    0.0
                },
                "keV",
                "photon-number-weighted, after the Be window; no anode self-absorption (real tubes are harder)",
            ),
            DerivedQuantity::new(
                "mean_photon_wavelength_nm",
                m.mean_wavelength_nm,
                "nm",
                "photon-number-weighted mean wavelength (the trait wavelength); no anode self-absorption",
            ),
            DerivedQuantity::new(
                "duane_hunt_min_wavelength_nm",
                self.duane_hunt_wavelength_nm(),
                "nm",
                "lambda_min = hc/(eV): continuum cutoff at E = kVp",
            ),
            DerivedQuantity::new(
                "photon_flux_density_at_100mm",
                m.photons_per_s / sphere_mm2,
                "photons/s/mm^2",
                "isotropic point source, vacuum path, 100 mm from the focal spot",
            ),
            DerivedQuantity::new(
                "power_density_at_100mm",
                power_w / (sphere_mm2 * 1e-2) * 1e3,
                "mW/cm^2",
                "isotropic point source, vacuum path, 100 mm from the focal spot",
            ),
        ];
        let k_edge = self.anode.k_edge_kev();
        out.push(DerivedQuantity::new(
            "k_shell_overvoltage",
            self.kvp / k_edge,
            "-",
            "U = kVp / E_K; K lines are emitted only for U > 1",
        ));
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use approx::assert_relative_eq;

    // Fixture numbers below were computed independently with a NumPy
    // re-implementation of the same equations (NIST Be mu/rho nodes with
    // log-log interpolation, exact Kramers bin integrals, 400-bin moments).

    #[test]
    fn test_bremsstrahlung_efficiency_fixture() {
        let src = XrayTubeSource::w_60kv().unwrap();
        // eta = 1.1e-9 * 74 * 60000 = 4.884e-3
        assert_relative_eq!(
            src.bremsstrahlung_efficiency(),
            4.884e-3,
            max_relative = 1e-12
        );
        // 60 kV * 30 mA = 1800 W; eta * 1800 W = 8.7912 W
        assert_relative_eq!(src.beam_power_w(), 1800.0, max_relative = 1e-12);
        assert_relative_eq!(src.bremsstrahlung_power_w(), 8.7912, max_relative = 1e-12);
    }

    #[test]
    fn test_efficiency_scales_with_z_and_voltage() {
        let w = XrayTubeSource::new(XrayAnode::W, 40.0, 10.0).unwrap();
        let cu = XrayTubeSource::new(XrayAnode::Cu, 40.0, 10.0).unwrap();
        assert_relative_eq!(
            w.bremsstrahlung_efficiency() / cu.bremsstrahlung_efficiency(),
            74.0 / 29.0,
            max_relative = 1e-12
        );
        // Doubling the voltage doubles eta and quadruples the brems power.
        let w80 = XrayTubeSource::new(XrayAnode::W, 80.0, 10.0).unwrap();
        assert_relative_eq!(
            w80.bremsstrahlung_efficiency() / w.bremsstrahlung_efficiency(),
            2.0,
            max_relative = 1e-12
        );
        assert_relative_eq!(
            w80.bremsstrahlung_power_w() / w.bremsstrahlung_power_w(),
            4.0,
            max_relative = 1e-12
        );
    }

    #[test]
    fn test_kramers_continuum_fixture_and_energy_normalization() {
        let src = XrayTubeSource::w_60kv().unwrap();
        // Ne (2 eta / E0) (E0 ln 60 - 59), Ne = 0.03 / e
        assert_relative_eq!(
            src.continuum_photons_per_s(1.0, 60.0),
            5.690_076_307_239_195e15,
            max_relative = 1e-9
        );
        // The energy spectrum E dN/dE ∝ (E0 - E) integrates to eta V I over
        // (0, E0]; over [0.5, 60] keV it carries the fraction
        // (60*59.5 - (60^2 - 0.5^2)/2) / (60^2/2) = 1770.125 / 1800.
        let n = 20_000;
        let (lo, hi) = (0.5, 60.0);
        let de = (hi - lo) / n as f64;
        let energy_kev_per_s: f64 = (0..n)
            .map(|j| {
                let a = lo + j as f64 * de;
                (a + 0.5 * de) * src.continuum_photons_per_s(a, a + de)
            })
            .sum();
        assert_relative_eq!(
            energy_kev_per_s * J_PER_KEV,
            src.bremsstrahlung_power_w() * 1770.125 / 1800.0,
            max_relative = 1e-5
        );
    }

    #[test]
    fn test_duane_hunt_limit() {
        let src = XrayTubeSource::w_60kv().unwrap();
        // lambda_min = 1.23984193 / 60 nm
        assert_relative_eq!(
            src.duane_hunt_wavelength_nm(),
            0.020_664_032_166_666_67,
            max_relative = 1e-12
        );
        // No continuum above E0, and the density vanishes linearly at E0
        // (energy spectrum ∝ E0 - E): adjacent 1 eV bins below the endpoint
        // carry photon counts in the ratio 3 : 1.
        assert_eq!(src.continuum_photons_per_s(60.0, 80.0), 0.0);
        let c1 = src.continuum_photons_per_s(59.998, 59.999);
        let c2 = src.continuum_photons_per_s(59.999, 60.0);
        assert_relative_eq!(c1 / c2, 3.0, max_relative = 1e-3);
        let bins = src.binned_photon_rate(64);
        assert!(bins.iter().all(|(e, _)| *e < 60.0 && *e > 1.0));
    }

    #[test]
    fn test_characteristic_lines_gated_by_edges() {
        let w60 = XrayTubeSource::w_60kv().unwrap();
        let labels: Vec<&str> = w60
            .line_photon_rates()
            .iter()
            .map(|(l, _)| l.label)
            .collect();
        assert!(labels.contains(&"La1") && labels.contains(&"Lb1"));
        assert!(
            !labels.iter().any(|l| l.starts_with('K')),
            "no W K lines below 69.5 kV"
        );
        let w100 = XrayTubeSource::new(XrayAnode::W, 100.0, 10.0).unwrap();
        assert_eq!(w100.line_photon_rates().len(), 8);

        // Mo K edge 20.000 keV; Cu K edge 8.979 keV.
        let mo19 = XrayTubeSource::new(XrayAnode::Mo, 19.0, 10.0).unwrap();
        let mo21 = XrayTubeSource::new(XrayAnode::Mo, 21.0, 10.0).unwrap();
        assert!(mo19.line_photon_rates().is_empty());
        assert_eq!(mo21.line_photon_rates().len(), 3);
        let cu = XrayTubeSource::new(XrayAnode::Cu, 8.9, 10.0).unwrap();
        assert!(cu.line_photon_rates().is_empty());

        // W Lb1 comes from an L2 vacancy (edge 11.544 keV): absent at 11 kV
        // while La1 (L3, 10.207 keV) is present.
        let w11 = XrayTubeSource::new(XrayAnode::W, 11.0, 10.0).unwrap();
        let l11: Vec<&str> = w11
            .line_photon_rates()
            .iter()
            .map(|(l, _)| l.label)
            .collect();
        assert!(l11.contains(&"La1") && !l11.contains(&"Lb1"));
    }

    #[test]
    fn test_line_energies_fixture() {
        // Cu Ka1 is the 1.5406 Å XRD line; Mo Ka1 is 0.7093 Å.
        let cu_ka1 = XrayAnode::Cu.characteristic_lines()[0];
        assert_eq!(cu_ka1.label, "Ka1");
        assert_relative_eq!(
            HC_KEV_NM / cu_ka1.energy_kev,
            0.154_060_117,
            max_relative = 1e-8
        );
        let mo_ka1 = XrayAnode::Mo.characteristic_lines()[0];
        assert_relative_eq!(
            HC_KEV_NM / mo_ka1.energy_kev,
            0.070_931_850,
            max_relative = 1e-8
        );
        // Every line lies below its own excitation edge.
        for anode in [XrayAnode::W, XrayAnode::Mo, XrayAnode::Cu, XrayAnode::Rh] {
            for l in anode.characteristic_lines() {
                assert!(l.energy_kev < l.edge_kev, "{:?} {}", anode, l.label);
            }
        }
    }

    #[test]
    fn test_line_yield_overvoltage_scaling() {
        // n ∝ (U - 1)^1.67: U = 3 vs U = 2 gives 2^1.67 = 3.18215.
        let edge = XrayAnode::Cu.k_edge_kev();
        let u2 = XrayTubeSource::new(XrayAnode::Cu, 2.0 * edge, 1.0).unwrap();
        let u3 = XrayTubeSource::new(XrayAnode::Cu, 3.0 * edge, 1.0).unwrap();
        let ka1 = XrayAnode::Cu.characteristic_lines()[0];
        assert_relative_eq!(
            u3.line_photons_per_electron(&ka1) / u2.line_photons_per_electron(&ka1),
            3.182_145_935_019_674_4,
            max_relative = 1e-12
        );
        // Absolute per-electron yields (empirical law): Cu Ka1 at 40 kV,
        // W La1 at 60 kV, W Ka1 at 100 kV.
        let cu40 = XrayTubeSource::cu_40kv().unwrap();
        assert_relative_eq!(
            cu40.line_photons_per_electron(&ka1),
            1.428_002_833_264_267_3e-3,
            max_relative = 1e-9
        );
        let w60 = XrayTubeSource::w_60kv().unwrap();
        let la1 = W_LINES[3];
        assert_relative_eq!(
            w60.line_photons_per_electron(&la1),
            2.619_729_858_605_342_5e-3,
            max_relative = 1e-9
        );
        let w100 = XrayTubeSource::new(XrayAnode::W, 100.0, 1.0).unwrap();
        assert_relative_eq!(
            w100.line_photons_per_electron(&W_LINES[0]),
            8.268_485_116_164_985e-5,
            max_relative = 1e-9
        );
    }

    #[test]
    fn test_be_window_transmission_fixture() {
        let src = XrayTubeSource::w_60kv().unwrap(); // 250 µm Be
        assert_relative_eq!(
            src.window_transmission(8.0),
            0.949_343_106_882_692_3,
            max_relative = 1e-9
        );
        assert_relative_eq!(
            src.window_transmission(3.0),
            0.373_910_991_699_512_7,
            max_relative = 1e-9
        );
        let mut bare = src.clone();
        bare.be_window_um = 0.0;
        assert_eq!(bare.window_transmission(3.0), 1.0);
    }

    #[test]
    fn test_window_hardens_the_spectrum() {
        let mut thin = XrayTubeSource::w_60kv().unwrap();
        thin.be_window_um = 50.0;
        let mut thick = thin.clone();
        thick.be_window_um = 2000.0;
        assert!(thick.mean_photon_energy_kev() > thin.mean_photon_energy_kev());
        assert!(thick.photon_rate() < thin.photon_rate());
        assert!(thick.wavelength_nm() < thin.wavelength_nm());
    }

    #[test]
    fn test_moments_fixture_w60() {
        let src = XrayTubeSource::w_60kv().unwrap();
        assert_relative_eq!(src.photon_rate(), 4.279_671_3e15, max_relative = 1e-6);
        assert_relative_eq!(
            src.characteristic_fraction(),
            0.199_912_2,
            max_relative = 1e-5
        );
        assert_relative_eq!(
            src.mean_photon_energy_kev(),
            12.921_383_4,
            max_relative = 1e-6
        );
        assert_relative_eq!(src.wavelength_nm(), 0.152_560_8, max_relative = 1e-6);
        assert_relative_eq!(src.bandwidth_pm(), 242.195_33, max_relative = 1e-6);
        assert_relative_eq!(src.xray_power_w(), 8.859_920_5, max_relative = 1e-6);
        assert_relative_eq!(src.average_power_w().unwrap(), src.xray_power_w());
    }

    #[test]
    fn test_moments_fixture_cu40() {
        let src = XrayTubeSource::cu_40kv().unwrap();
        assert_relative_eq!(src.photon_rate(), 1.537_106_2e15, max_relative = 1e-6);
        assert_relative_eq!(
            src.characteristic_fraction(),
            0.377_280_3,
            max_relative = 1e-5
        );
        assert_relative_eq!(src.wavelength_nm(), 0.167_473_2, max_relative = 1e-6);
        // Cu Ka1 after 250 µm Be: 3.387135e14 photons/s.
        let ka1 = src
            .line_photon_rates()
            .into_iter()
            .find(|(l, _)| l.label == "Ka1")
            .unwrap()
            .1;
        assert_relative_eq!(ka1, 3.387_135e14, max_relative = 1e-6);
    }

    #[test]
    fn test_spectral_flux_density_fixture_and_scaling() {
        let src = XrayTubeSource::w_60kv().unwrap();
        // 59 bins of exactly 1 keV; bin centred at 20.5 keV, 500 mm.
        let flux = src.spectral_flux_density(500.0, 59);
        assert_eq!(flux.len(), 59);
        let (e, phi) = flux[19];
        assert_relative_eq!(e, 20.5, max_relative = 1e-12);
        assert_relative_eq!(phi, 1.851_102_36e7, max_relative = 1e-8);
        // Inverse-square law.
        let far = src.spectral_flux_density(1000.0, 59);
        assert_relative_eq!(far[19].1 * 4.0, phi, max_relative = 1e-12);
        // Linear in tube current.
        let mut hot = src.clone();
        hot.current_ma *= 2.0;
        assert_relative_eq!(
            hot.spectral_flux_density(500.0, 59)[19].1,
            2.0 * phi,
            max_relative = 1e-12
        );
        // Integral over energy reproduces the total rate / (4 pi d^2).
        let integral: f64 = flux.iter().map(|(_, p)| p * 1.0).sum();
        let total: f64 = src.binned_photon_rate(59).iter().map(|(_, r)| r).sum();
        assert_relative_eq!(
            integral,
            total / (4.0 * std::f64::consts::PI * 500.0 * 500.0),
            max_relative = 1e-12
        );
        assert!(src.spectral_flux_density(0.0, 10).is_empty());
    }

    #[test]
    fn test_xray_spectrum_is_normalized_tabulated() {
        let src = XrayTubeSource::cu_40kv().unwrap();
        match src.xray_spectrum(128) {
            XraySpectrum::Tabulated {
                energies_kev,
                relative_flux,
            } => {
                assert_eq!(energies_kev.len(), 128);
                assert!(energies_kev.windows(2).all(|w| w[1] > w[0]));
                let sum: f64 = relative_flux.iter().sum();
                assert_relative_eq!(sum, 1.0, epsilon = 1e-12);
                // The Cu Ka bin dominates the spectrum.
                let (imax, _) = relative_flux
                    .iter()
                    .enumerate()
                    .fold(
                        (0, 0.0),
                        |acc, (i, &v)| if v > acc.1 { (i, v) } else { acc },
                    );
                assert!(
                    (energies_kev[imax] - 8.04).abs() < 0.4,
                    "peak at {}",
                    energies_kev[imax]
                );
            }
            other => panic!("expected Tabulated, got {other:?}"),
        }
        // deep_xray's sampler accepts it and returns normalized weights.
        let pairs = src.xray_spectrum(64).sample(64, 1.0, 40.0);
        let sum: f64 = pairs.iter().map(|(_, w)| w).sum();
        assert_relative_eq!(sum, 1.0, epsilon = 1e-12);
    }

    #[test]
    fn test_spectral_weights_sum_to_one() {
        for src in [
            XrayTubeSource::w_60kv().unwrap(),
            XrayTubeSource::cu_40kv().unwrap(),
            XrayTubeSource::mo_50kv().unwrap(),
        ] {
            let w = src.spectral_weights();
            assert_eq!(w.len(), src.spectral_samples);
            let sum: f64 = w.iter().map(|(_, x)| x).sum();
            assert_relative_eq!(sum, 1.0, epsilon = 1e-12);
            // Ascending wavelength order.
            assert!(w.windows(2).all(|p| p[1].0 > p[0].0));
        }
    }

    #[test]
    fn test_validation_rejects_bad_parameters() {
        assert!(XrayTubeSource::new(XrayAnode::W, 4.0, 10.0).is_err());
        assert!(XrayTubeSource::new(XrayAnode::W, 301.0, 10.0).is_err());
        assert!(XrayTubeSource::new(XrayAnode::W, f64::NAN, 10.0).is_err());
        assert!(XrayTubeSource::new(XrayAnode::W, 60.0, 0.0).is_err());
        assert!(XrayTubeSource::new(XrayAnode::W, 60.0, -1.0).is_err());
        let mut src = XrayTubeSource::w_60kv().unwrap();
        src.be_window_um = -1.0;
        assert!(src.validate().is_err());
        src.be_window_um = 100.0;
        src.spectral_samples = 0;
        assert!(src.validate().is_err());
        assert_eq!(XrayAnode::from_name("Tungsten"), Some(XrayAnode::W));
        assert_eq!(XrayAnode::from_name("rh"), Some(XrayAnode::Rh));
        assert_eq!(XrayAnode::from_name("unobtainium"), None);
    }

    #[test]
    fn test_derived_quantities_cover_key_physics() {
        let src = XrayTubeSource::w_60kv().unwrap();
        let dq = src.derived_quantities();
        let get = |name: &str| {
            dq.iter()
                .find(|q| q.name == name)
                .unwrap_or_else(|| panic!("{name}"))
                .value
        };
        assert_relative_eq!(
            get("bremsstrahlung_efficiency"),
            4.884e-3,
            max_relative = 1e-12
        );
        assert_relative_eq!(get("photon_rate"), 4.279_671_3e15, max_relative = 1e-6);
        assert_relative_eq!(
            get("photon_flux_density_at_100mm"),
            3.405_654_2e10,
            max_relative = 1e-6
        );
        assert_relative_eq!(
            get("power_density_at_100mm"),
            7.050_500_7,
            max_relative = 1e-6
        );
        assert_relative_eq!(
            get("k_shell_overvoltage"),
            60.0 / 69.525,
            max_relative = 1e-12
        );
        assert!(get("anode_heat_load_w") > 0.99 * src.beam_power_w());
        assert!(dq.iter().all(|q| q.value.is_finite()));
    }

    #[test]
    fn test_photon_density_uses_mean_photon_energy() {
        let src = XrayTubeSource::w_60kv().unwrap();
        // 10 J/m^2 / (12.9213834 keV) -> 4.8303722e-3 photons/nm^2 per mJ/cm^2
        assert_relative_eq!(
            src.photon_density_per_mj_cm2(),
            4.830_372_2e-3,
            max_relative = 1e-6
        );
        // photon_energy_ev reports the same mean photon energy.
        assert_relative_eq!(src.photon_energy_ev(), 12_921.383_4, max_relative = 1e-7);
        assert_relative_eq!(
            src.photon_density_per_mj_cm2(),
            10.0 / (src.photon_energy_ev() * ELECTRON_CHARGE_C) * 1e-18,
            max_relative = 1e-12
        );
        // The harmonic-mean energy hc/<lambda> would overcount by <E><1/E>.
        let harmonic_kev = HC_KEV_NM / src.wavelength_nm();
        assert_relative_eq!(
            src.mean_photon_energy_kev() / harmonic_kev,
            1.589_957_8,
            max_relative = 1e-6
        );
    }

    #[test]
    fn test_anode_presets() {
        // Rh 50 kV / 40 mA: independent fixture (NIST Be, 250 um).
        let rh = XrayTubeSource::preset(XrayAnode::Rh).unwrap();
        assert_eq!(rh.anode, XrayAnode::Rh);
        assert_relative_eq!(rh.beam_power_w(), 2000.0, max_relative = 1e-12);
        assert_relative_eq!(rh.photon_rate(), 2.283_887_1e15, max_relative = 1e-6);
        assert_relative_eq!(rh.wavelength_nm(), 0.156_651_8, max_relative = 1e-6);
        // Rh K lines (edge 23.22 keV) are excited at 50 kV.
        assert_eq!(rh.line_photon_rates().len(), 3);
        for (anode, kvp) in [
            (XrayAnode::W, 60.0),
            (XrayAnode::Mo, 50.0),
            (XrayAnode::Cu, 40.0),
            (XrayAnode::Rh, 50.0),
        ] {
            let p = XrayTubeSource::preset(anode).unwrap();
            assert_eq!(p.anode, anode);
            assert_relative_eq!(p.kvp, kvp);
        }
    }

    #[test]
    fn test_incoherent_and_cw() {
        let src = XrayTubeSource::w_60kv().unwrap();
        assert_eq!(src.transverse_coherence(), 0.0);
        assert_eq!(src.pulse_energy_j(), None);
        assert_eq!(LithographySource::rep_rate_hz(&src), None);
    }
}
