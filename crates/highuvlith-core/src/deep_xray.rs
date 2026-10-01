//! LIGA deep X-ray lithography: 1:1 proximity shadow printing.
//!
//! LIGA (Lithographie, Galvanoformung, Abformung) exposes very thick resist
//! (hundreds of um of PMMA) with the hard, penetrating white beam of a
//! synchrotron bending magnet to make high-aspect-ratio microstructures. This
//! is **shadow printing**, not projection: the mask sits a small proximity gap
//! above the resist and casts a near-geometric shadow, so there is no pupil,
//! no aerial-imaging TCC, and no [`crate::source::LithographySource`]
//! illumination here. The mask is an Au absorber pattern on a thin low-Z
//! membrane (Be or Ti); high-energy photons leak through the absorber, setting
//! the dose contrast.
//!
//! # Pipeline
//!
//! ```text
//!   spectrum (BM universal function, absolute BM beamline, or an absolute
//!             flux-density table)          -> photon-weighted energy bins
//!     -> beam filters + membrane transmission            (harden the beam)
//!     -> depth dose u(z) = sum_j N_j E_j T_j mu_en,j exp(-mu_j z)
//!     -> scale so u(bottom) = target bottom dose; absolute spectra also give
//!        the dose RATE and hence the exposure time
//!   lateral, per bin: complex mask field t_j = m + (1 - m) a_j,
//!        a_j = exp(-mu_abs t / 2) exp(-i 2 pi delta_abs t / lambda)
//!     -> Fresnel (angular-spectrum) propagation over gap + depth
//!        (default) or the legacy Gaussian blur sigma = sqrt(lambda g) / 2
//!     -> volumetric dose D(x,y,z), development depth, sidewall metrics
//! ```
//!
//! # Key equations
//!
//! - Bending-magnet flux, vertically integrated (standard synchrotron
//!   formula): `dN/dtheta = 2.457e13 E[GeV] I[A] G1(y)` photons s^-1 mrad^-1
//!   (0.1% BW)^-1, `G1(y) = y Int_y^inf K_5/3`, `y = E/E_c`. The constant is
//!   derived here from CODATA values as `sqrt(3) alpha gamma(1 GeV) / (2 pi
//!   e) * 1e-6`. Photons per unit `ln E` are `1e3 dN/dtheta`, so on bins
//!   equally spaced in `ln E` the photon weight is proportional to `G1`.
//! - Scan-averaged flux density at the mask: a uniform vertical scan of
//!   height `H` at source distance `L` spreads one mrad of horizontal fan over
//!   `L[m] mm x H mm`: `Phi(E) = dN/(dtheta dE) / (L H)` photons s^-1 mm^-2
//!   keV^-1.
//! - Absorbed dose rate: `Ddot(z) = sum_j N_j E_j T_j mu_en,j exp(-mu_j z)`;
//!   with `N_j` in photons s^-1 mm^-2, `E_j` in J and `mu` in mm^-1 this is
//!   W mm^-3 = kJ cm^-3 s^-1. Exposure time `t = D_target / Ddot(bottom)`.
//! - Angular-spectrum propagation over distance `d` (exact scalar free-space
//!   transfer function, `exp(-i w t)` convention):
//!   `U(d) = IFFT[ FFT[t] H ]`, `H = exp(i 2 pi d (sqrt(1/lambda^2 - f^2) -
//!   1/lambda))`, which is `exp(-i pi lambda d f^2)` in the paraxial limit.
//!   Inside the resist the field keeps propagating (`n - 1 ~ 1e-6`), so depth
//!   `z` sees `d = g + z`.
//! - Straight edge (analytic reference): `U = a + (1 - a) F(v)`,
//!   `F = (1 - i)/2 [(1/2 + C(v)) + i (1/2 + S(v))]`, `v = x sqrt(2/(lambda
//!   d))`, with the Fresnel integrals `C`, `S`; for an opaque absorber
//!   `|F|^2 = 0.25` at the geometric edge and the first maximum is 1.3704 at
//!   `v = 1.2172`.
//! - Absorber optical constants: `mu` from the NIST tables
//!   ([`crate::materials::attenuation`]), `delta` from the Henke/CXRO `f1`
//!   tables ([`crate::materials::henke`]) - `f1` is held at its 30 keV value
//!   above 30 keV (valid away from K edges; Au's K edge at 80.7 keV is not
//!   modelled).
//!
//! # Model status
//!
//! Implemented: polychromatic depth dose with spectral hardening and the
//! `mu` (attenuation) / `mu_en` (deposition) distinction, bending-magnet
//! photon weights per unit `ln E`, absolute bending-magnet flux from ring
//! parameters and beamline geometry, absolute flux-density tables (contract
//! C5, e.g. X-ray tubes), dose rate and exposure time, a complex
//! (absorbing + phase-shifting) thin-screen absorber, scalar Fresnel
//! propagation across the gap and through the resist depth (2D FFT on the
//! LIGA grid, analytic straight edge, periodic 1D profiles), the legacy
//! Gaussian proximity blur, a sampling check, volumetric dose, threshold
//! development depth, sidewall metrics and a direct-scission PMMA rate model.
//!
//! Approximations (documented at each site): the absorber is a thin screen
//! (valid while its own Fresnel scale `sqrt(lambda t_abs)` is small compared
//! with `sqrt(lambda g)`); the beam is a collimated, spatially coherent plane
//! wave at normal incidence (no horizontal-fan runout, no source-size
//! penumbra - for BM beamlines `sigma_source (g + z) / L` is nm-scale); the
//! exposure is averaged over a uniform vertical scan taller than the beam
//! (the vertical angular profile is not modelled); the 2D path is periodic
//! over the simulation field; photoelectron transport is only an optional
//! Gaussian of the Grun range (off by default) - no Monte-Carlo electron
//! transport, no secondary electrons or fluorescence from the absorber,
//! membrane or substrate, no mask heating; there is **no PEB** (PMMA
//! main-chain scission is a direct radiolysis process); beamline mirrors are
//! not modelled (emulate their high-energy cut-off with filters or supply a
//! measured spectrum).
//!
//! Data range: attenuation is tabulated for 0.03 keV - 20 MeV. The sampled
//! energy window is always inside it - default windows are clamped to it, an
//! explicit `energy_range_kev` reaching outside is rejected by the
//! `Result`-returning entry points ([`DeepXrayConfig::check_energy_window`])
//! and truncated with a warning by [`expose_depth`] (photons outside are
//! dropped, never given end-of-table coefficients). The absorber phase holds
//! the Henke `f1` at its 30 keV value above 30 keV with `delta ~ lambda^2`
//! (the high-energy limit `f1 -> Z`, minus small relativistic corrections).

use ndarray::Array2;
use serde::{Deserialize, Serialize};

use crate::mask::Mask;
use crate::materials::attenuation::{self, Compound};
use crate::materials::henke;
use crate::math::fft2d::Fft2D;
use crate::source_models::physics::{self, bm_universal_flux};
use crate::source_models::synchrotron::{SynchrotronBeamline, SynchrotronSource};
use crate::types::{Complex64, Grid3D, GridConfig};

/// `hc` in keV nm (CODATA).
const HC_KEV_NM: f64 = 1.239_841_93;
/// Joules per keV (exact, SI 2019).
const J_PER_KEV: f64 = 1.602_176_634e-16;
/// Fine-structure constant (CODATA 2018).
const ALPHA: f64 = 7.297_352_569_3e-3;
/// Elementary charge in C (exact).
const E_CHARGE: f64 = 1.602_176_634e-19;

/// Photon energy [keV] -> wavelength [nm] via `lambda = hc/E`.
fn wavelength_nm(energy_kev: f64) -> f64 {
    HC_KEV_NM / energy_kev
}

/// Textbook bending-magnet flux constant in photons s^-1 mrad^-1 (0.1%
/// BW)^-1 per GeV per A: `sqrt(3) alpha gamma(1 GeV) / (2 pi e) * 1e-3 (per
/// mrad) * 1e-3 (per 0.1% BW)` = 2.457e13 with `gamma = E/(m_e c^2)`. The
/// computations use [`physics::bm_flux_per_mrad`] (the repository's
/// `gamma = 1 + E/(m_e c^2)` convention, 2e-4 larger at 2.5 GeV).
pub fn bm_flux_constant() -> f64 {
    let gamma_per_gev = 1e3 / physics::ELECTRON_REST_MEV;
    3f64.sqrt() * ALPHA * gamma_per_gev / (2.0 * std::f64::consts::PI * E_CHARGE) * 1e-6
}

/// Absolute bending-magnet exposure beamline: ring parameters plus the
/// geometry that turns the fan into an areal flux density at the mask.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BendingMagnetBeamline {
    /// Ring electron energy in GeV.
    pub electron_energy_gev: f64,
    /// Bending-magnet field in T.
    pub field_t: f64,
    /// Stored ring current in mA.
    pub ring_current_ma: f64,
    /// Source-to-mask distance in m (1 mrad of fan covers `L` mm).
    pub source_distance_m: f64,
    /// Horizontal acceptance in mrad (sets the exposed field width and the
    /// incident power; the flux density does not depend on it).
    pub horizontal_acceptance_mrad: f64,
    /// Vertical scan height at the mask in mm: the scanner sweeps mask and
    /// substrate uniformly through the fan, spreading the vertically
    /// integrated flux over this height.
    pub vertical_scan_mm: f64,
}

impl BendingMagnetBeamline {
    /// Build from a bending-magnet [`SynchrotronSource`] (ring energy, field,
    /// current) plus the exposure geometry. `None` for undulator beamlines.
    pub fn from_synchrotron(
        src: &SynchrotronSource,
        source_distance_m: f64,
        horizontal_acceptance_mrad: f64,
        vertical_scan_mm: f64,
    ) -> Option<Self> {
        match &src.beamline {
            SynchrotronBeamline::BendingMagnet {
                electron_energy_gev,
                field_t,
                ..
            } => Some(Self {
                electron_energy_gev: *electron_energy_gev,
                field_t: *field_t,
                ring_current_ma: src.ring_current_ma,
                source_distance_m,
                horizontal_acceptance_mrad,
                vertical_scan_mm,
            }),
            SynchrotronBeamline::Undulator { .. } => None,
        }
    }

    /// Validate that every parameter is positive and finite.
    pub fn validate(&self) -> crate::error::Result<()> {
        for (name, value) in [
            ("electron_energy_gev", self.electron_energy_gev),
            ("field_t", self.field_t),
            ("ring_current_ma", self.ring_current_ma),
            ("source_distance_m", self.source_distance_m),
            (
                "horizontal_acceptance_mrad",
                self.horizontal_acceptance_mrad,
            ),
            ("vertical_scan_mm", self.vertical_scan_mm),
        ] {
            if !(value.is_finite() && value > 0.0) {
                return Err(crate::error::LithographyError::InvalidParameter {
                    name,
                    value,
                    reason: "must be positive and finite",
                });
            }
        }
        Ok(())
    }

    /// Critical energy `E_c = 0.665 E^2 B` in keV.
    pub fn critical_energy_kev(&self) -> f64 {
        physics::critical_energy_kev(self.electron_energy_gev, self.field_t)
    }

    /// Lorentz factor of the stored beam, `1 + E/(m_e c^2)` - the same
    /// convention as [`SynchrotronSource::gamma`].
    pub fn gamma(&self) -> f64 {
        physics::gamma_from_mev(self.electron_energy_gev * 1e3)
    }

    /// Vertically integrated flux per unit horizontal angle and unit `ln E`
    /// in photons s^-1 mrad^-1: `1e3 x` [`physics::bm_flux_per_mrad`]
    /// (`(sqrt3/2pi) alpha gamma (I/e) G1(E/E_c)` per 0.1% BW, i.e.
    /// `2.457e13 E[GeV] I[A] G1`) - identical to
    /// [`SynchrotronSource::bm_flux_per_mrad`] for the same ring.
    pub fn photons_per_s_mrad_per_ln_e(&self, energy_kev: f64) -> f64 {
        1e3 * physics::bm_flux_per_mrad(
            self.gamma(),
            self.ring_current_ma * 1e-3,
            energy_kev / self.critical_energy_kev(),
        )
    }

    /// Scan-averaged spectral photon flux density at the mask plane in
    /// photons s^-1 mm^-2 keV^-1 (before filters and membrane).
    pub fn flux_density(&self, energy_kev: f64) -> f64 {
        if energy_kev <= 0.0 {
            return 0.0;
        }
        self.photons_per_s_mrad_per_ln_e(energy_kev)
            / energy_kev
            / (self.source_distance_m * self.vertical_scan_mm)
    }

    /// Total radiated power per mrad of horizontal fan in W,
    /// [`physics::bm_power_per_mrad_w`] (`e gamma^4 I / (6 pi eps0 rho)`
    /// per rad, bending radius from the field). The flux spectrum integrates
    /// to the same value within the rounding of `E_c = 0.665 E^2 B`
    /// (`Int_0^inf G1(y) dy = 8 pi / (9 sqrt 3)`).
    pub fn power_per_mrad_w(&self) -> f64 {
        let gamma = self.gamma();
        physics::bm_power_per_mrad_w(
            gamma,
            physics::bending_radius_m(gamma, self.field_t),
            self.ring_current_ma * 1e-3,
        )
    }

    /// Power entering the horizontal acceptance in W (before filters).
    pub fn incident_power_w(&self) -> f64 {
        self.power_per_mrad_w() * self.horizontal_acceptance_mrad
    }

    /// Exposed field width at the mask in mm (`L theta_h`).
    pub fn field_width_mm(&self) -> f64 {
        self.source_distance_m * self.horizontal_acceptance_mrad
    }

    /// Order-of-magnitude vertical beam height at the mask in mm,
    /// `L / gamma` (the characteristic opening angle near `E_c`; harder
    /// photons are more collimated, softer ones less).
    pub fn vertical_beam_height_mm(&self) -> f64 {
        self.source_distance_m * 1e3 / self.gamma()
    }

    /// Human-readable caveats for this geometry (empty when none apply).
    pub fn warnings(&self) -> Vec<String> {
        let mut w = Vec::new();
        let h = self.vertical_beam_height_mm();
        if self.vertical_scan_mm < 2.0 * h {
            w.push(format!(
                "vertical scan {:.2} mm is not much taller than the beam (~L/gamma = {:.2} mm): \
                 the uniform scan-average model overestimates the dose rate",
                self.vertical_scan_mm, h
            ));
        }
        w
    }
}

/// X-ray exposure spectrum for LIGA shadow printing.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum XraySpectrum {
    /// Explicit `(energy, relative flux)` table (e.g. a measured beamline
    /// spectrum, or a single line for a monochromatic study). Each point is a
    /// discrete line carrying the given relative photon number (sample a
    /// continuous spectrum on equal-width bins before building this).
    Tabulated {
        /// Photon energies in keV (ascending).
        energies_kev: Vec<f64>,
        /// Relative photon flux at each energy (arbitrary units).
        relative_flux: Vec<f64>,
    },
    /// Synchrotron bending-magnet white beam, characterized entirely by its
    /// critical energy; photons per unit `ln E` follow `G1(E/E_c)`. Relative
    /// only (no exposure time).
    BendingMagnet {
        /// Critical photon energy in keV (`E_c = 0.665 E^2[GeV] B[T]`).
        critical_energy_kev: f64,
    },
    /// ABSOLUTE spectral photon flux density at the mask plane (contract C5):
    /// photons s^-1 mm^-2 keV^-1 at each energy, before the beam filters and
    /// the membrane. Each table point is a sample carrying the flux of its
    /// Voronoi cell (half-way to its neighbours; the end points use the full
    /// neighbour spacing, so equal spacing = histogram bins). Build with
    /// [`XraySpectrum::from_flux_density`].
    FluxDensity {
        /// Photon energies in keV (strictly ascending, >= 2 points).
        energies_kev: Vec<f64>,
        /// Spectral flux density in photons s^-1 mm^-2 keV^-1.
        photons_per_s_mm2_kev: Vec<f64>,
    },
    /// Absolute bending-magnet beamline (ring parameters + geometry).
    BendingMagnetBeamline(BendingMagnetBeamline),
}

impl XraySpectrum {
    /// Map a bending-magnet synchrotron beamline to a [`XraySpectrum::BendingMagnet`]
    /// via its critical energy. Returns `None` for undulator beamlines (they
    /// have no bending-magnet critical energy / white-beam spectrum).
    pub fn from_synchrotron(src: &SynchrotronSource) -> Option<Self> {
        src.critical_energy_kev()
            .map(|e_c| XraySpectrum::BendingMagnet {
                critical_energy_kev: e_c,
            })
    }

    /// ABSOLUTE bending-magnet spectrum from a [`SynchrotronSource`] (ring
    /// energy, field and current) and the exposure geometry. `None` for
    /// undulators.
    pub fn from_synchrotron_beamline(
        src: &SynchrotronSource,
        source_distance_m: f64,
        horizontal_acceptance_mrad: f64,
        vertical_scan_mm: f64,
    ) -> Option<Self> {
        BendingMagnetBeamline::from_synchrotron(
            src,
            source_distance_m,
            horizontal_acceptance_mrad,
            vertical_scan_mm,
        )
        .map(XraySpectrum::BendingMagnetBeamline)
    }

    /// ABSOLUTE spectrum from an `(E_keV, photons s^-1 mm^-2 keV^-1)` table
    /// at the mask plane (contract C5; e.g. `XrayTubeSource::
    /// spectral_flux_density`). Errors unless there are >= 2 points with
    /// strictly ascending positive energies and non-negative, finite values.
    pub fn from_flux_density(table: &[(f64, f64)]) -> crate::error::Result<Self> {
        if table.len() < 2 {
            return Err(crate::error::LithographyError::InvalidParameter {
                name: "flux_density_table",
                value: table.len() as f64,
                reason: "needs at least 2 (energy, flux density) points",
            });
        }
        for (k, (e, v)) in table.iter().enumerate() {
            if !(e.is_finite() && *e > 0.0) || (k > 0 && *e <= table[k - 1].0) {
                return Err(crate::error::LithographyError::InvalidParameter {
                    name: "energies_kev",
                    value: *e,
                    reason: "energies must be positive and strictly ascending",
                });
            }
            if !(v.is_finite() && *v >= 0.0) {
                return Err(crate::error::LithographyError::InvalidParameter {
                    name: "photons_per_s_mm2_kev",
                    value: *v,
                    reason: "flux density must be finite and non-negative",
                });
            }
        }
        Ok(XraySpectrum::FluxDensity {
            energies_kev: table.iter().map(|(e, _)| *e).collect(),
            photons_per_s_mm2_kev: table.iter().map(|(_, v)| *v).collect(),
        })
    }

    /// Whether this spectrum carries an absolute flux (so exposure times can
    /// be computed).
    pub fn is_absolute(&self) -> bool {
        matches!(
            self,
            XraySpectrum::FluxDensity { .. } | XraySpectrum::BendingMagnetBeamline(_)
        )
    }

    /// A sensible default energy window in keV for sampling this spectrum,
    /// clamped to the attenuation-data range (0.03 keV - 20 MeV).
    ///
    /// For a bending magnet this spans `0.1 E_c` to `8 E_c`: below, the
    /// photons are absorbed in any membrane or filter (and within microns of
    /// the resist surface without one); above, `G1(y) < 3e-3` of its peak.
    /// For a table it is the min/max of the listed energies.
    pub fn default_energy_range_kev(&self) -> (f64, f64) {
        let (lo_lim, hi_lim) = (attenuation::MIN_ENERGY_KEV, attenuation::MAX_ENERGY_KEV);
        let bm = |e_c: f64| {
            let lo = (0.1 * e_c).clamp(lo_lim, hi_lim);
            let hi = (8.0 * e_c).clamp(lo, hi_lim);
            (lo, hi)
        };
        let table = |energies: &[f64]| {
            let lo = energies
                .iter()
                .copied()
                .fold(f64::INFINITY, f64::min)
                .max(lo_lim);
            let hi = energies
                .iter()
                .copied()
                .fold(f64::NEG_INFINITY, f64::max)
                .min(hi_lim);
            (lo.min(hi), hi)
        };
        match self {
            XraySpectrum::BendingMagnet {
                critical_energy_kev,
            } => bm(*critical_energy_kev),
            XraySpectrum::BendingMagnetBeamline(b) => bm(b.critical_energy_kev()),
            XraySpectrum::Tabulated { energies_kev, .. }
            | XraySpectrum::FluxDensity { energies_kev, .. } => table(energies_kev),
        }
    }

    /// Photon-number samples `(energy_kev, weight)` over `[e_min, e_max]`,
    /// unnormalized: for absolute spectra the weight is photons s^-1 mm^-2 in
    /// the sample's energy cell; for relative ones it is in arbitrary units.
    fn raw_samples(&self, n_bins: usize, e_min_kev: f64, e_max_kev: f64) -> Vec<(f64, f64)> {
        // Bins equally spaced in ln E: photons per bin = (dN/d ln E) d(ln E),
        // with `per_ln_e` the photon density per unit ln E (G1 for a bending
        // magnet, up to a constant).
        let log_bins = |per_ln_e: &dyn Fn(f64) -> f64| -> Vec<(f64, f64)> {
            let n = n_bins.max(1);
            if !(e_min_kev > 0.0 && e_max_kev > e_min_kev) {
                return Vec::new();
            }
            let dln = (e_max_kev / e_min_kev).ln() / n as f64;
            (0..n)
                .map(|j| {
                    let e = e_min_kev * ((j as f64 + 0.5) * dln).exp();
                    (e, per_ln_e(e) * dln)
                })
                .collect()
        };
        match self {
            XraySpectrum::BendingMagnet {
                critical_energy_kev,
            } => log_bins(&|e| bm_universal_flux(e / critical_energy_kev)),
            XraySpectrum::BendingMagnetBeamline(b) => {
                // photons s^-1 mm^-2 per log bin = (dN/dtheta/dlnE) dlnE / (L H).
                let area = b.source_distance_m * b.vertical_scan_mm;
                log_bins(&|e| b.photons_per_s_mrad_per_ln_e(e) / area)
            }
            XraySpectrum::Tabulated {
                energies_kev,
                relative_flux,
            } => energies_kev
                .iter()
                .zip(relative_flux.iter())
                .filter(|(e, _)| **e >= e_min_kev && **e <= e_max_kev)
                .map(|(e, f)| (*e, f.max(0.0)))
                .collect(),
            XraySpectrum::FluxDensity {
                energies_kev,
                photons_per_s_mm2_kev,
            } => {
                let n = energies_kev.len();
                (0..n)
                    .filter(|&i| energies_kev[i] >= e_min_kev && energies_kev[i] <= e_max_kev)
                    .map(|i| {
                        let width = if n < 2 {
                            0.0
                        } else if i == 0 {
                            energies_kev[1] - energies_kev[0]
                        } else if i == n - 1 {
                            energies_kev[n - 1] - energies_kev[n - 2]
                        } else {
                            0.5 * (energies_kev[i + 1] - energies_kev[i - 1])
                        };
                        (energies_kev[i], photons_per_s_mm2_kev[i].max(0.0) * width)
                    })
                    .collect()
            }
        }
    }

    /// Sample the spectrum into `(energy_kev, weight)` bins with weights
    /// proportional to photon number and normalized to sum to 1.
    ///
    /// A bending magnet is sampled on `n_bins` bins equally spaced in `ln E`
    /// across `[e_min, e_max]` (geometric bin centres) with weight
    /// `G1(E/E_c)` - the photon number per unit relative bandwidth, hence per
    /// log bin. A table returns its own points inside `[e_min, e_max]` (so a
    /// single-line table stays exactly monochromatic); `n_bins` is then
    /// ignored. A flux-density table weights each point by its cell width.
    pub fn sample(&self, n_bins: usize, e_min_kev: f64, e_max_kev: f64) -> Vec<(f64, f64)> {
        let mut pairs = self.raw_samples(n_bins, e_min_kev, e_max_kev);
        let sum: f64 = pairs.iter().map(|(_, w)| w).sum();
        if sum > 0.0 {
            for (_, w) in &mut pairs {
                *w /= sum;
            }
        }
        pairs
    }

    /// Absolute photon flux per sample in photons s^-1 mm^-2 at the mask
    /// plane (before filters/membrane), for absolute spectra; `None` for the
    /// relative variants.
    pub fn sample_absolute(
        &self,
        n_bins: usize,
        e_min_kev: f64,
        e_max_kev: f64,
    ) -> Option<Vec<(f64, f64)>> {
        self.is_absolute()
            .then(|| self.raw_samples(n_bins, e_min_kev, e_max_kev))
    }
}

/// An attenuating layer in the beam (filter, mask membrane, or mask absorber).
/// Transmission is `exp(-mu(E) t)` with `mu` in 1/um and `t` in um.
#[derive(Debug, Clone, Serialize)]
pub struct BeamFilter {
    /// Material of the layer.
    pub compound: Compound,
    /// Layer thickness in um.
    pub thickness_um: f64,
}

impl BeamFilter {
    /// Construct a beam filter of the given compound and thickness.
    pub fn new(compound: Compound, thickness_um: f64) -> Self {
        Self {
            compound,
            thickness_um,
        }
    }

    /// Intensity transmission `exp(-mu(E) t)` at `energy_kev` (in [0, 1]).
    pub fn transmission(&self, energy_kev: f64) -> f64 {
        (-self.compound.mu_per_um(energy_kev) * self.thickness_um).exp()
    }

    /// Complex amplitude transmission of the layer relative to vacuum,
    /// `exp(-mu t / 2) exp(-i 2 pi delta t / lambda)` (thin-screen /
    /// projection approximation). `mu` is the total attenuation (NIST);
    /// `delta` comes from the Henke `f1` tables of the compound's elements,
    /// with `f1` held at its 30 eV / 30 keV value outside the tables
    /// (`delta ~ lambda^2 f1`).
    pub fn amplitude_transmission(&self, energy_kev: f64) -> Complex64 {
        let e_ev = energy_kev * 1e3;
        let e_tab = e_ev.clamp(henke::HENKE_MIN_EV, henke::HENKE_MAX_EV);
        let delta = henke::delta_beta_from_mass_fractions(
            self.compound.components(),
            self.compound.density_g_cm3,
            e_tab,
        )
        .map(|(d, _)| d * (e_tab / e_ev).powi(2))
        .unwrap_or(0.0);
        let t_nm = self.thickness_um * 1e3;
        let phase = -2.0 * std::f64::consts::PI * delta * t_nm / wavelength_nm(energy_kev);
        Complex64::from_polar(
            (-0.5 * self.compound.mu_per_um(energy_kev) * self.thickness_um).exp(),
            phase,
        )
    }
}

/// How the mask-to-resist proximity gap (and the propagation through the
/// resist) is modelled laterally.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProximityModel {
    /// Scalar angular-spectrum (Fresnel) propagation of the complex absorber
    /// field, per energy bin, over `gap + z` for every depth `z` (default).
    #[default]
    Fresnel,
    /// Legacy fast approximation: the partial-absorber intensity blurred by a
    /// Gaussian of the first-Fresnel-zone width `sigma = sqrt(lambda g)/2`
    /// at the gap only (no spreading inside the resist, no fringes).
    Gaussian,
}

/// Configuration for a LIGA deep X-ray exposure.
///
/// Serializable but not deserializable (its [`Compound`]s reference `&'static`
/// element symbols); build one with [`DeepXrayConfig::pmma_default`] or the
/// struct literal.
#[derive(Debug, Clone, Serialize)]
pub struct DeepXrayConfig {
    /// Exposure spectrum.
    pub spectrum: XraySpectrum,
    /// Beam-conditioning filters upstream of the mask (harden the beam).
    pub filters: Vec<BeamFilter>,
    /// Mask absorber (patterned Au); high-E photons leak through it.
    pub absorber: BeamFilter,
    /// Mask membrane carrying the absorber (e.g. Be or Ti).
    pub membrane: BeamFilter,
    /// Resist compound (e.g. PMMA).
    pub resist: Compound,
    /// Resist thickness in um.
    pub resist_thickness_um: f64,
    /// Absorber-to-resist proximity gap in um (free-space propagation
    /// distance to the resist surface).
    pub proximity_gap_um: f64,
    /// Target absorbed-energy density at the resist bottom in kJ/cm^3 (the
    /// clearing dose the process is scaled to reach in open features).
    pub target_bottom_dose_kj_cm3: f64,
    /// Absorbed-energy density above which the resist/substrate is damaged
    /// (foaming, T-topping); the top dose should stay below this.
    pub damage_dose_kj_cm3: f64,
    /// Whether to add a photoelectron-transport (Grun range) blur.
    pub photoelectron_blur: bool,
    /// Number of energy bins for a bending-magnet spectrum.
    pub energy_bins: usize,
    /// Lateral proximity model (default [`ProximityModel::Fresnel`]).
    pub proximity_model: ProximityModel,
    /// Optional explicit photon-energy window `(min, max)` in keV; `None`
    /// uses [`XraySpectrum::default_energy_range_kev`].
    pub energy_range_kev: Option<(f64, f64)>,
}

impl DeepXrayConfig {
    /// Standard thick-PMMA LIGA preset: 500 um PMMA, 20 um Au absorber
    /// (19.3 g/cm^3) on a 2 um Ti membrane, 100 um proximity gap, 3 kJ/cm^3
    /// bottom clearing dose with a 20 kJ/cm^3 damage ceiling, 100 energy
    /// bins, no upstream filters, Fresnel proximity model.
    pub fn pmma_default(spectrum: XraySpectrum) -> Self {
        Self {
            spectrum,
            filters: Vec::new(),
            absorber: BeamFilter::new(Compound::gold(19.3), 20.0),
            membrane: BeamFilter::new(Compound::titanium(4.51), 2.0),
            resist: Compound::pmma(),
            resist_thickness_um: 500.0,
            proximity_gap_um: 100.0,
            target_bottom_dose_kj_cm3: 3.0,
            damage_dose_kj_cm3: 20.0,
            photoelectron_blur: false,
            energy_bins: 100,
            proximity_model: ProximityModel::Fresnel,
            energy_range_kev: None,
        }
    }

    /// The photon-energy window used for sampling (keV): `energy_range_kev`
    /// (or the spectrum's default), intersected with the attenuation-data
    /// range so no bin is evaluated outside the tables.
    pub fn energy_window_kev(&self) -> (f64, f64) {
        let (lo_lim, hi_lim) = (attenuation::MIN_ENERGY_KEV, attenuation::MAX_ENERGY_KEV);
        let (lo, hi) = self
            .energy_range_kev
            .unwrap_or_else(|| self.spectrum.default_energy_range_kev());
        let lo = lo.clamp(lo_lim, hi_lim);
        (lo, hi.clamp(lo, hi_lim))
    }

    /// Errors if an explicit `energy_range_kev` is not an increasing window
    /// inside the attenuation-data range 0.03 keV - 20 MeV.
    pub fn check_energy_window(&self) -> crate::error::Result<()> {
        if let Some((lo, hi)) = self.energy_range_kev {
            attenuation::check_energy_kev(lo)?;
            attenuation::check_energy_kev(hi)?;
            if lo >= hi {
                return Err(crate::error::LithographyError::InvalidParameter {
                    name: "energy_range_kev",
                    value: hi,
                    reason: "the window maximum must exceed its minimum",
                });
            }
        }
        Ok(())
    }

    /// Combined transmission of all upstream filters plus the membrane at
    /// `energy_kev` (everything the beam crosses before the resist, in the
    /// open areas). The absorber is *not* included: it applies only under the
    /// patterned regions and is handled laterally.
    fn beam_transmission(&self, energy_kev: f64) -> f64 {
        let mut t = self.membrane.transmission(energy_kev);
        for f in &self.filters {
            t *= f.transmission(energy_kev);
        }
        t
    }

    /// Precompute everything a lateral or depth calculation needs per bin.
    fn spectral_bins(&self) -> Vec<SpectralBin> {
        let (lo, hi) = self.energy_window_kev();
        let relative = self.spectrum.sample(self.energy_bins, lo, hi);
        let absolute = self.spectrum.sample_absolute(self.energy_bins, lo, hi);
        relative
            .iter()
            .enumerate()
            .map(|(j, (e, w))| SpectralBin {
                energy_kev: *e,
                lambda_nm: wavelength_nm(*e),
                weight: *w,
                photons_per_s_mm2: absolute.as_ref().map(|a| a[j].1),
                beam_transmission: self.beam_transmission(*e),
                mu_per_um: self.resist.mu_per_um(*e),
                mu_en_per_um: self.resist.mu_en_per_um(*e),
                absorber: self.absorber.amplitude_transmission(*e),
            })
            .collect()
    }
}

/// Per-energy-bin quantities shared by every dose path.
#[derive(Debug, Clone)]
struct SpectralBin {
    energy_kev: f64,
    lambda_nm: f64,
    /// Normalized photon weight (sums to 1 over bins).
    weight: f64,
    /// Absolute photons s^-1 mm^-2 at the mask (absolute spectra only).
    photons_per_s_mm2: Option<f64>,
    /// Filters x membrane intensity transmission.
    beam_transmission: f64,
    /// Resist linear attenuation (1/um): attenuates the beam.
    mu_per_um: f64,
    /// Resist linear energy absorption (1/um): deposits the dose.
    mu_en_per_um: f64,
    /// Absorber complex amplitude transmission.
    absorber: Complex64,
}

impl SpectralBin {
    /// Relative absorbed-energy density at depth `z_um` in an open column:
    /// `w E T mu_en exp(-mu z)`.
    fn dose_weight(&self, z_um: f64) -> f64 {
        self.weight
            * self.energy_kev
            * self.beam_transmission
            * self.mu_en_per_um
            * (-self.mu_per_um * z_um).exp()
    }

    /// Absorber intensity transmission `|a|^2 = exp(-mu t)`.
    fn absorber_intensity(&self) -> f64 {
        self.absorber.norm_sqr()
    }
}

/// Depth-dose profile as `(z_um, relative absorbed-energy density)` from the
/// resist top (`z = 0`) to the bottom (`z = resist_thickness`).
///
/// `u(z) = sum_j w_j E_j T_beam(E_j) mu_en,j exp(-mu_j z)` over the sampled
/// spectrum, with `mu_j` (total attenuation) and `mu_en,j` (energy
/// absorption) of the resist in 1/um. Values are relative (arbitrary units);
/// [`expose_depth`] scales them to absolute dose.
pub fn depth_dose(config: &DeepXrayConfig) -> Vec<(f64, f64)> {
    const DEPTH_SAMPLES: usize = 128;
    let bins = config.spectral_bins();
    let thickness = config.resist_thickness_um;
    let denom = (DEPTH_SAMPLES - 1).max(1) as f64;
    (0..DEPTH_SAMPLES)
        .map(|k| {
            let z = k as f64 / denom * thickness;
            (z, bins.iter().map(|b| b.dose_weight(z)).sum())
        })
        .collect()
}

/// Absolute LIGA depth-dose result: the profile scaled so the resist bottom
/// receives the configured clearing dose, plus (for absolute spectra) the
/// dose rate and exposure time.
#[derive(Debug, Clone, Serialize)]
pub struct LigaExposure {
    /// Depth grid in um (top `z=0` to bottom `z=thickness`).
    pub z_um: Vec<f64>,
    /// Absolute absorbed-energy density in kJ/cm^3 at each depth.
    pub dose_kj_cm3: Vec<f64>,
    /// Multiplicative scale applied to the relative profile to hit the target
    /// bottom dose.
    pub scale: f64,
    /// Dose at the resist top in kJ/cm^3.
    pub top_dose_kj_cm3: f64,
    /// Dose at the resist bottom in kJ/cm^3 (= target when reachable).
    pub bottom_dose_kj_cm3: f64,
    /// Top/bottom dose ratio (>= 1; contrast the process must tolerate).
    pub dose_ratio: f64,
    /// Whether the top dose exceeds the damage ceiling.
    pub exceeds_damage_ceiling: bool,
    /// Exposure time in seconds to reach the target bottom dose; `Some` only
    /// for absolute spectra ([`XraySpectrum::is_absolute`]).
    pub exposure_time_estimate: Option<f64>,
    /// Absorbed dose rate at the resist top in kJ cm^-3 s^-1 (absolute
    /// spectra only).
    pub top_dose_rate_kj_cm3_s: Option<f64>,
    /// Absorbed dose rate at the resist bottom in kJ cm^-3 s^-1 (absolute
    /// spectra only).
    pub bottom_dose_rate_kj_cm3_s: Option<f64>,
    /// Photon power density reaching the resist surface in open areas
    /// (after filters and membrane) in W/mm^2 (absolute spectra only).
    pub resist_power_density_w_mm2: Option<f64>,
    /// Ring current x exposure time in mA h (bending-magnet beamlines only;
    /// the customary LIGA exposure unit).
    pub exposure_charge_ma_h: Option<f64>,
    /// Dose-weighted mean photon energy at the resist top in keV.
    pub mean_energy_top_kev: f64,
    /// Dose-weighted mean photon energy at the resist bottom in keV (larger:
    /// the beam hardens with depth).
    pub mean_energy_bottom_kev: f64,
    /// Fraction of the filter- and membrane-transmitted beam power (open
    /// areas) that lies inside the sampled energy window; 1 means nothing
    /// was clipped. For a bending magnet the full spectrum is integrated
    /// over `[1e-4, 40] E_c` (within the 0.03 keV - 20 MeV data range).
    pub window_power_fraction: f64,
    /// Caveats about the absolute-flux model and the energy window (empty
    /// when none apply).
    pub warnings: Vec<String>,
}

/// Compute the absolute depth-dose exposure for a configuration: run
/// [`depth_dose`], then scale so the bottom of the resist reaches
/// `target_bottom_dose_kj_cm3`. For absolute spectra the absolute dose rate
/// `Ddot(z) = sum_j N_j E_j T_j mu_en,j exp(-mu_j z)` (W mm^-3 = kJ cm^-3
/// s^-1) gives the exposure time `t = D_target / Ddot(bottom)`.
pub fn expose_depth(config: &DeepXrayConfig) -> LigaExposure {
    let profile = depth_dose(config);
    let z_um: Vec<f64> = profile.iter().map(|(z, _)| *z).collect();
    let u: Vec<f64> = profile.iter().map(|(_, u)| *u).collect();

    let u_bottom = *u.last().unwrap_or(&0.0);
    let scale = if u_bottom > 0.0 {
        config.target_bottom_dose_kj_cm3 / u_bottom
    } else {
        0.0
    };

    let dose_kj_cm3: Vec<f64> = u.iter().map(|v| v * scale).collect();
    let top_dose_kj_cm3 = *dose_kj_cm3.first().unwrap_or(&0.0);
    let bottom_dose_kj_cm3 = *dose_kj_cm3.last().unwrap_or(&0.0);
    let ratio = if bottom_dose_kj_cm3 > 0.0 {
        top_dose_kj_cm3 / bottom_dose_kj_cm3
    } else {
        f64::INFINITY
    };

    let bins = config.spectral_bins();
    let thickness = config.resist_thickness_um;
    let mean_energy = |z: f64| {
        let (num, den) = bins.iter().fold((0.0, 0.0), |(n, d), b| {
            let w = b.dose_weight(z);
            (n + w * b.energy_kev, d + w)
        });
        if den > 0.0 {
            num / den
        } else {
            0.0
        }
    };

    // Absolute dose rate (W/mm^3 == kJ cm^-3 s^-1): photons s^-1 mm^-2 x J x
    // mu_en [1/mm].
    let rate = |z: f64| -> Option<f64> {
        bins.iter()
            .map(|b| {
                b.photons_per_s_mm2.map(|n| {
                    n * b.energy_kev
                        * J_PER_KEV
                        * b.beam_transmission
                        * b.mu_en_per_um
                        * 1e3
                        * (-b.mu_per_um * z).exp()
                })
            })
            .sum()
    };
    let top_rate = rate(0.0);
    let bottom_rate = rate(thickness);
    let power_density = bins
        .iter()
        .map(|b| {
            b.photons_per_s_mm2
                .map(|n| n * b.energy_kev * J_PER_KEV * b.beam_transmission)
        })
        .sum::<Option<f64>>();
    let exposure_time = bottom_rate
        .filter(|r| *r > 0.0)
        .map(|r| config.target_bottom_dose_kj_cm3 / r);
    let (charge, mut warnings) = match &config.spectrum {
        XraySpectrum::BendingMagnetBeamline(b) => (
            exposure_time.map(|t| b.ring_current_ma * t / 3600.0),
            b.warnings(),
        ),
        _ => (None, Vec::new()),
    };
    if let Err(e) = config.check_energy_window() {
        let (lo, hi) = config.energy_window_kev();
        warnings.push(format!(
            "energy_range_kev {:?} rejected ({e}); sampled the truncated window \
             {lo:.3}-{hi:.3} keV instead",
            config.energy_range_kev.unwrap_or_default()
        ));
    }
    let window_fraction = window_power_fraction(config);
    if window_fraction < 0.99 {
        let (lo, hi) = config.energy_window_kev();
        warnings.push(format!(
            "{:.1}% of the transmitted beam power lies outside the sampled energy window \
             {lo:.3}-{hi:.3} keV (e.g. soft photons with no membrane or filter); widen \
             energy_range_kev to include it",
            100.0 * (1.0 - window_fraction)
        ));
    }

    LigaExposure {
        z_um,
        dose_kj_cm3,
        scale,
        top_dose_kj_cm3,
        bottom_dose_kj_cm3,
        dose_ratio: ratio,
        exceeds_damage_ceiling: top_dose_kj_cm3 > config.damage_dose_kj_cm3,
        exposure_time_estimate: exposure_time,
        top_dose_rate_kj_cm3_s: top_rate,
        bottom_dose_rate_kj_cm3_s: bottom_rate,
        resist_power_density_w_mm2: power_density,
        exposure_charge_ma_h: charge,
        mean_energy_top_kev: mean_energy(0.0),
        mean_energy_bottom_kev: mean_energy(thickness),
        window_power_fraction: window_fraction,
        warnings,
    }
}

/// Fraction of the transmitted (filters x membrane) beam power that falls
/// inside the configured energy window: bending magnets are integrated on a
/// fine log grid over `[1e-4, 40] E_c` (clamped to the attenuation-data
/// range), tables over all their points.
fn window_power_fraction(config: &DeepXrayConfig) -> f64 {
    let (lo, hi) = config.energy_window_kev();
    let samples: Vec<(f64, f64)> = match &config.spectrum {
        XraySpectrum::BendingMagnet {
            critical_energy_kev: e_c,
        } => wide_bm_grid(*e_c),
        XraySpectrum::BendingMagnetBeamline(b) => wide_bm_grid(b.critical_energy_kev()),
        XraySpectrum::Tabulated { energies_kev, .. }
        | XraySpectrum::FluxDensity { energies_kev, .. } => {
            let e_min = energies_kev.iter().copied().fold(f64::INFINITY, f64::min);
            let e_max = energies_kev
                .iter()
                .copied()
                .fold(f64::NEG_INFINITY, f64::max);
            config.spectrum.raw_samples(0, e_min, e_max)
        }
    };
    let (mut inside, mut total) = (0.0, 0.0);
    for (e, n) in samples {
        let p = n * e * config.beam_transmission(e);
        total += p;
        if (lo..=hi).contains(&e) {
            inside += p;
        }
    }
    if total > 0.0 {
        inside / total
    } else {
        1.0
    }
}

/// `(E, relative photons)` of a bending magnet on 800 log bins spanning
/// `[1e-4, 40] E_c` within the attenuation-data range.
fn wide_bm_grid(e_c: f64) -> Vec<(f64, f64)> {
    let a = (1e-4 * e_c).clamp(attenuation::MIN_ENERGY_KEV, attenuation::MAX_ENERGY_KEV);
    let b = (40.0 * e_c).clamp(a, attenuation::MAX_ENERGY_KEV);
    if b <= a {
        return Vec::new();
    }
    let n = 800;
    let dln = (b / a).ln() / n as f64;
    (0..n)
        .map(|j| {
            let e = a * ((j as f64 + 0.5) * dln).exp();
            (e, bm_universal_flux(e / e_c) * dln)
        })
        .collect()
}

/// Mask open fraction `m(x,y)` in [0, 1] on `grid`: the magnitude of the
/// rasterized amplitude (1 = clear, 0 = absorber; antialiased edge pixels
/// carry their clear-area fraction). LIGA takes only the *geometry* from the
/// mask - the absorber physics comes from [`DeepXrayConfig::absorber`].
fn open_fraction(mask: &Mask, grid: &GridConfig) -> Array2<f64> {
    mask.rasterize(grid).mapv(|c| c.norm().clamp(0.0, 1.0))
}

/// First-Fresnel-zone proximity blur width in nm for photon energy
/// `energy_kev` over distance `distance_um`: `sigma = 0.5 sqrt(lambda d)`
/// (the legacy [`ProximityModel::Gaussian`] kernel).
fn proximity_sigma_nm(energy_kev: f64, distance_um: f64) -> f64 {
    0.5 * (wavelength_nm(energy_kev) * distance_um.max(0.0) * 1e3).sqrt()
}

/// Gruen-range photoelectron blur width in nm for `energy_kev` in a resist of
/// density `density_g_cm3`: `R_G[um] = 0.046 E[keV]^1.75 / rho`, converted to
/// nm. Used as the sigma of an optional Gaussian - a crude, conservative
/// stand-in for photoelectron transport (the true radial energy-deposition
/// kernel is strongly peaked); off by default.
pub fn grun_range_nm(energy_kev: f64, density_g_cm3: f64) -> f64 {
    let r_g_um = 0.046 * energy_kev.powf(1.75) / density_g_cm3;
    r_g_um * 1e3
}

/// Photoelectron blur sigma in nm for a bin, or 0 when disabled.
fn photoelectron_sigma_nm(config: &DeepXrayConfig, energy_kev: f64) -> f64 {
    if config.photoelectron_blur {
        grun_range_nm(energy_kev, config.resist.density_g_cm3)
    } else {
        0.0
    }
}

/// Total legacy Gaussian width in nm: proximity penumbra at the gap combined
/// in quadrature with the optional photoelectron range.
fn gaussian_sigma_nm(config: &DeepXrayConfig, energy_kev: f64) -> f64 {
    let sigma_prox = proximity_sigma_nm(energy_kev, config.proximity_gap_um);
    let sigma_e = photoelectron_sigma_nm(config, energy_kev);
    (sigma_prox * sigma_prox + sigma_e * sigma_e).sqrt()
}

/// Squared spatial frequency `f^2` (1/nm^2) on an `n x n` FFT grid with pixel
/// `pixel_nm`, in the FFT's natural (unshifted) order.
fn freq_sq_grid(n: usize, pixel_nm: f64) -> Array2<f64> {
    let f = |i: usize| {
        let k = if i < n.div_ceil(2) {
            i as f64
        } else {
            i as f64 - n as f64
        };
        k / (n as f64 * pixel_nm)
    };
    Array2::from_shape_fn((n, n), |(i, j)| {
        let (fy, fx) = (f(i), f(j));
        fx * fx + fy * fy
    })
}

/// Free-space angular-spectrum transfer function for squared spatial
/// frequency `f2` (1/nm^2), wavelength `lambda_nm` and distance `d_nm`, with
/// the common phase `exp(i k d)` removed:
/// `exp(i 2 pi d (sqrt(1/lambda^2 - f^2) - 1/lambda))`, written in the
/// cancellation-free form `exp(-i 2 pi d f^2 / (1/lambda + sqrt(...)))`;
/// evanescent components (`f > 1/lambda`) decay.
fn transfer_function(f2: f64, lambda_nm: f64, d_nm: f64) -> Complex64 {
    let inv_l2 = 1.0 / (lambda_nm * lambda_nm);
    let two_pi_d = 2.0 * std::f64::consts::PI * d_nm;
    if f2 <= inv_l2 {
        let phase = -two_pi_d * f2 / (1.0 / lambda_nm + (inv_l2 - f2).sqrt());
        Complex64::from_polar(1.0, phase)
    } else {
        let kappa = (f2 - inv_l2).sqrt();
        Complex64::from_polar((-two_pi_d * kappa).exp(), -two_pi_d / lambda_nm)
    }
}

/// Indices of the bins that matter for a set of depth weights `w[j][k]`:
/// drops the smallest bins while their summed maximum weight stays below
/// `1e-7` of the smallest per-depth total, so no depth loses more than that
/// relative fraction of its dose.
fn significant_bins(weights: &[Vec<f64>]) -> Vec<usize> {
    let nk = weights.first().map_or(0, |w| w.len());
    let min_total = (0..nk)
        .map(|k| weights.iter().map(|w| w[k]).sum::<f64>())
        .fold(f64::INFINITY, f64::min);
    let mut order: Vec<(usize, f64)> = weights
        .iter()
        .enumerate()
        .map(|(j, w)| (j, w.iter().copied().fold(0.0, f64::max)))
        .collect();
    order.sort_by(|a, b| a.1.total_cmp(&b.1));
    let budget = 1e-7 * min_total;
    let mut dropped = 0.0;
    let mut keep = Vec::new();
    for (j, wmax) in order {
        if dropped + wmax <= budget {
            dropped += wmax;
        } else {
            keep.push(j);
        }
    }
    keep.sort_unstable();
    keep
}

/// Fresnel-propagated, spectrally weighted lateral dose sum on the grid for
/// one depth: `sum_j W_j |a_j + (1 - a_j) IFFT[M H_j(d)]|^2` (optionally
/// photoelectron-blurred per bin), where `M = FFT(m)`.
#[allow(clippy::too_many_arguments)]
fn fresnel_slice(
    config: &DeepXrayConfig,
    bins: &[SpectralBin],
    keep: &[usize],
    weights: &[f64],
    open_spectrum: &Array2<Complex64>,
    f2: &Array2<f64>,
    distance_nm: f64,
    fft: &Fft2D,
) -> Array2<f64> {
    let (ny, nx) = open_spectrum.dim();
    let mut acc = Array2::<f64>::zeros((ny, nx));
    let mut acc_spec: Option<Array2<Complex64>> = None;
    let mut field = Array2::<Complex64>::zeros((ny, nx));
    for &j in keep {
        let b = &bins[j];
        let w = weights[j];
        if w <= 0.0 {
            continue;
        }
        ndarray::Zip::from(&mut field)
            .and(open_spectrum)
            .and(f2)
            .for_each(|out, &m, &q| *out = m * transfer_function(q, b.lambda_nm, distance_nm));
        fft.inverse(&mut field);
        let a = b.absorber;
        let one_minus_a = Complex64::new(1.0, 0.0) - a;
        let sigma_e = photoelectron_sigma_nm(config, b.energy_kev);
        if sigma_e > 0.0 {
            // Blur |U|^2 by the photoelectron Gaussian in the frequency domain.
            let mut intensity =
                field.mapv(|v| Complex64::new((a + one_minus_a * v).norm_sqr(), 0.0));
            fft.forward(&mut intensity);
            let spec = acc_spec.get_or_insert_with(|| Array2::zeros((ny, nx)));
            let s2 = 2.0 * std::f64::consts::PI.powi(2) * sigma_e * sigma_e;
            ndarray::Zip::from(spec)
                .and(&intensity)
                .and(f2)
                .for_each(|s, &i, &q| *s += i * (w * (-s2 * q).exp()));
        } else {
            ndarray::Zip::from(&mut acc)
                .and(&field)
                .for_each(|s, &v| *s += w * (a + one_minus_a * v).norm_sqr());
        }
    }
    if let Some(mut spec) = acc_spec {
        fft.inverse(&mut spec);
        ndarray::Zip::from(&mut acc)
            .and(&spec)
            .for_each(|s, &v| *s += v.re);
    }
    acc
}

/// Legacy Gaussian lateral image for one bin: the partial-absorber intensity
/// `T_abs + (1 - T_abs) m` blurred by [`gaussian_sigma_nm`].
fn gaussian_bin_image(
    config: &DeepXrayConfig,
    bin: &SpectralBin,
    open: &Array2<f64>,
    pixel_nm: f64,
) -> Array2<f64> {
    let t_abs = bin.absorber_intensity();
    let img = open.mapv(|p| t_abs + (1.0 - t_abs) * p);
    gaussian_blur_2d(&img, gaussian_sigma_nm(config, bin.energy_kev) / pixel_nm)
}

/// Lateral shadow image at the resist surface, normalized so a fully open
/// area is 1: the surface absorbed-dose map `D(x,y,0) / D_open(0)`.
///
/// With [`ProximityModel::Fresnel`] each energy bin's complex mask field is
/// propagated over the gap and the intensities are summed with the surface
/// dose weights `w_j E_j T_j mu_en,j`; with [`ProximityModel::Gaussian`] the
/// partial-absorber intensity is Gaussian-blurred instead. Absorber-covered
/// areas sit on the dose-weighted high-energy leakage floor `> 0`; Fresnel
/// edge ringing can exceed 1 locally. The field is treated as periodic
/// (Fresnel) or edge-replicated (Gaussian).
pub fn shadow_image(
    config: &DeepXrayConfig,
    mask: &Mask,
    grid: &GridConfig,
) -> crate::error::Result<Array2<f64>> {
    config.check_energy_window()?;
    let open = open_fraction(mask, grid);
    let bins = config.spectral_bins();
    let weights: Vec<f64> = bins.iter().map(|b| b.dose_weight(0.0)).collect();
    let total: f64 = weights.iter().sum();
    if total.is_nan() || total <= 0.0 {
        return Err(crate::error::LithographyError::InvalidParameter {
            name: "spectrum",
            value: total,
            reason: "no photons reach the resist in the sampled energy window",
        });
    }
    let norm: Vec<f64> = weights.iter().map(|w| w / total).collect();
    match config.proximity_model {
        ProximityModel::Gaussian => {
            let mut acc = Array2::<f64>::zeros(open.dim());
            for (b, w) in bins.iter().zip(norm.iter()) {
                if *w > 0.0 {
                    acc.scaled_add(*w, &gaussian_bin_image(config, b, &open, grid.pixel_nm));
                }
            }
            Ok(acc)
        }
        ProximityModel::Fresnel => {
            let fft = Fft2D::new();
            let mut spectrum = open.mapv(|v| Complex64::new(v, 0.0));
            fft.forward(&mut spectrum);
            let f2 = freq_sq_grid(grid.size, grid.pixel_nm);
            let keep = significant_bins(&norm.iter().map(|w| vec![*w]).collect::<Vec<_>>());
            Ok(fresnel_slice(
                config,
                &bins,
                &keep,
                &norm,
                &spectrum,
                &f2,
                config.proximity_gap_um * 1e3,
                &fft,
            ))
        }
    }
}

/// Volumetric absorbed-dose field `D(x,y,z)` over the resist thickness.
///
/// `D = s sum_j W_j(z) I_j(x,y; z)`, `W_j(z) = w_j E_j T_j mu_en,j
/// exp(-mu_j z)`, where `I_j` is the lateral intensity of bin `j`: with
/// [`ProximityModel::Fresnel`] the complex mask field propagated over
/// `gap + z` (so the edge blur grows with depth); with
/// [`ProximityModel::Gaussian`] the legacy gap-only blurred partial-absorber
/// image. The field is scaled (like [`expose_depth`]) so that a fully open
/// column (`m = 1`) reaches `target_bottom_dose_kj_cm3` at the deepest voxel
/// centre, giving per-voxel dose in kJ/cm^3. The `z` axis is stored in nm in
/// the returned grid (`z_min_nm = 0`, `z_max_nm = thickness * 1000`). Depth
/// slices are computed in parallel when the `parallel` feature is on.
pub fn expose_volumetric(
    config: &DeepXrayConfig,
    mask: &Mask,
    grid: &GridConfig,
    nz: usize,
) -> crate::error::Result<Grid3D<f64>> {
    config.check_energy_window()?;
    if config.resist_thickness_um.is_nan() || config.resist_thickness_um <= 0.0 {
        return Err(crate::error::LithographyError::InvalidParameter {
            name: "resist_thickness_um",
            value: config.resist_thickness_um,
            reason: "must be positive",
        });
    }
    if config.proximity_gap_um.is_nan() || config.proximity_gap_um < 0.0 {
        return Err(crate::error::LithographyError::InvalidParameter {
            name: "proximity_gap_um",
            value: config.proximity_gap_um,
            reason: "must be non-negative",
        });
    }
    let open = open_fraction(mask, grid);
    let bins = config.spectral_bins();

    let n = grid.size;
    let half = grid.field_size_nm() / 2.0;
    let thickness_um = config.resist_thickness_um;
    let mut vol = Grid3D::<f64>::new(
        n,
        n,
        nz,
        (-half, half),
        (-half, half),
        (0.0, thickness_um * 1e3),
    )?;

    // Depth centers in um for each z-slice.
    let z_um: Vec<f64> = (0..nz).map(|k| vol.z_at(k) * 1e-3).collect();

    // Open-column dose at the deepest sampled voxel centre, used for absolute
    // scaling (cell centres sit at thickness - dz/2).
    let z_bottom = z_um.last().copied().unwrap_or(thickness_um);
    let open_bottom: f64 = bins.iter().map(|b| b.dose_weight(z_bottom)).sum();
    let scale = if open_bottom > 0.0 {
        config.target_bottom_dose_kj_cm3 / open_bottom
    } else {
        0.0
    };

    // weights[j][k] = s W_j(z_k).
    let weights: Vec<Vec<f64>> = bins
        .iter()
        .map(|b| z_um.iter().map(|z| scale * b.dose_weight(*z)).collect())
        .collect();

    match config.proximity_model {
        ProximityModel::Gaussian => {
            for (b, w) in bins.iter().zip(weights.iter()) {
                if w.iter().all(|v| *v <= 0.0) {
                    continue;
                }
                let lateral = gaussian_bin_image(config, b, &open, grid.pixel_nm);
                for (k, wk) in w.iter().enumerate() {
                    let mut slice = vol.data.index_axis_mut(ndarray::Axis(0), k);
                    slice.scaled_add(*wk, &lateral);
                }
            }
        }
        ProximityModel::Fresnel => {
            let fft = Fft2D::new();
            let mut spectrum = open.mapv(|v| Complex64::new(v, 0.0));
            fft.forward(&mut spectrum);
            let f2 = freq_sq_grid(n, grid.pixel_nm);
            let keep = significant_bins(&weights);
            let gap_nm = config.proximity_gap_um * 1e3;
            let compute = |k: usize| -> Array2<f64> {
                let wk: Vec<f64> = weights.iter().map(|w| w[k]).collect();
                let local_fft = Fft2D::new();
                fresnel_slice(
                    config,
                    &bins,
                    &keep,
                    &wk,
                    &spectrum,
                    &f2,
                    gap_nm + z_um[k] * 1e3,
                    &local_fft,
                )
            };
            #[cfg(feature = "parallel")]
            let slices: Vec<Array2<f64>> = {
                use rayon::prelude::*;
                (0..nz).into_par_iter().map(compute).collect()
            };
            #[cfg(not(feature = "parallel"))]
            let slices: Vec<Array2<f64>> = (0..nz).map(compute).collect();
            for (k, s) in slices.into_iter().enumerate() {
                vol.data.index_axis_mut(ndarray::Axis(0), k).assign(&s);
            }
        }
    }

    Ok(vol)
}

/// Grid-resolution diagnostics for the lateral LIGA models (see
/// [`fresnel_sampling`]).
#[derive(Debug, Clone, Serialize)]
pub struct FresnelSampling {
    /// Lateral pixel in nm.
    pub pixel_nm: f64,
    /// Lateral field size in nm.
    pub field_nm: f64,
    /// Dose-weighted mean Fresnel scale `sqrt(lambda g)` at the resist top
    /// in nm (0 for contact exposure).
    pub fresnel_scale_top_nm: f64,
    /// Dose-weighted mean `sqrt(lambda (g + T))` at the resist bottom in nm.
    pub fresnel_scale_bottom_nm: f64,
    /// Largest lateral spread of the grid's highest spatial frequency,
    /// `lambda (g + z) / (2 dx)` in nm, over depths `z` (top to bottom) and
    /// the bins carrying >= 1e-3 of the dose at that depth. Above half the
    /// field, a non-periodic mask wraps around.
    pub max_spread_nm: f64,
    /// Whether the grid resolves the edge diffraction where it is finest:
    /// `dx <= sqrt(lambda g) / 2` at the top (or at the bottom for contact
    /// exposure).
    pub resolved: bool,
    /// Human-readable warnings (empty when the grid is adequate).
    pub warnings: Vec<String>,
}

impl FresnelSampling {
    /// Error unless [`FresnelSampling::resolved`] - for callers that want the
    /// sampling check to be strict.
    pub fn require_resolved(&self) -> crate::error::Result<()> {
        if self.resolved {
            Ok(())
        } else {
            Err(crate::error::LithographyError::InvalidParameter {
                name: "pixel_nm",
                value: self.pixel_nm,
                reason: "grid too coarse to resolve the Fresnel scale sqrt(lambda g): \
                         use pixel_nm <= fresnel_scale_top_nm / 2, the 1D edge-profile tool, \
                         or the Gaussian proximity model",
            })
        }
    }
}

/// Check whether `grid` can represent the LIGA proximity diffraction of
/// `config`: compares the pixel with the dose-weighted Fresnel scale
/// `sqrt(lambda d)` at the resist top (`d = g`) and bottom (`d = g + T`), and
/// the angular spread of the grid's highest frequency with the field (the 2D
/// propagation is periodic). Always returns a report; the `warnings` explain
/// any shortfall. On an unresolved grid the Fresnel result converges to the
/// pixel-averaged geometric shadow - still a valid dose map at the pixel
/// scale, but without edge ringing or edge shape; use
/// [`fresnel_edge_profile_1d`] for sidewall analysis.
pub fn fresnel_sampling(config: &DeepXrayConfig, grid: &GridConfig) -> FresnelSampling {
    let bins = config.spectral_bins();
    let g_nm = config.proximity_gap_um.max(0.0) * 1e3;
    let t_nm = config.resist_thickness_um.max(0.0) * 1e3;
    let weighted_scale = |z_um: f64, d_nm: f64| {
        let (num, den) = bins.iter().fold((0.0, 0.0), |(n, d), b| {
            let w = b.dose_weight(z_um);
            (n + w * (b.lambda_nm * d_nm).sqrt(), d + w)
        });
        if den > 0.0 {
            num / den
        } else {
            0.0
        }
    };
    let top = weighted_scale(0.0, g_nm);
    let bottom = weighted_scale(config.resist_thickness_um, g_nm + t_nm);
    let pixel = grid.pixel_nm;
    let field = grid.field_size_nm();
    // Spread of the finest grid frequency, lambda (g + z) / (2 dx), for the
    // bins that carry >= 1e-3 of the dose at depth z (soft photons only
    // matter near the surface, where their propagation distance is short).
    let max_spread = (0..=4)
        .map(|k| {
            let z_um = config.resist_thickness_um.max(0.0) * k as f64 / 4.0;
            let peak = bins.iter().map(|b| b.dose_weight(z_um)).fold(0.0, f64::max);
            bins.iter()
                .filter(|b| b.dose_weight(z_um) >= 1e-3 * peak)
                .map(|b| b.lambda_nm * (g_nm + z_um * 1e3) / (2.0 * pixel))
                .fold(0.0, f64::max)
        })
        .fold(0.0, f64::max);
    let finest = if g_nm > 0.0 { top } else { bottom };
    let resolved = pixel <= 0.5 * finest;
    let mut warnings = Vec::new();
    if !resolved {
        warnings.push(format!(
            "pixel {pixel:.1} nm under-resolves the Fresnel scale sqrt(lambda d) = {finest:.1} nm \
             (dose-weighted, at the {}): edge ringing and edge shape are not represented and the \
             lateral dose tends to the pixel-averaged geometric shadow; use pixel <= {:.1} nm or \
             fresnel_edge_profile_1d for sidewall analysis",
            if g_nm > 0.0 {
                "resist top"
            } else {
                "resist bottom"
            },
            0.5 * finest
        ));
    }
    if max_spread > 0.5 * field {
        warnings.push(format!(
            "diffraction spread {max_spread:.0} nm of the finest grid frequency exceeds half the \
             field ({:.0} nm): the 2D propagation is periodic, so a non-periodic mask wraps around \
             - use a field that is a whole number of mask periods",
            0.5 * field
        ));
    }
    FresnelSampling {
        pixel_nm: pixel,
        field_nm: field,
        fresnel_scale_top_nm: top,
        fresnel_scale_bottom_nm: bottom,
        max_spread_nm: max_spread,
        resolved,
        warnings,
    }
}

/// Fresnel integrals `(C(x), S(x)) = Int_0^x (cos, sin)(pi t^2 / 2) dt`.
///
/// Power series for `|x| < 1.5`; above, the complementary error function
/// continued fraction evaluated by the modified Lentz method (the standard
/// algorithm for these integrals). Accuracy ~1e-14 absolute (checked against
/// numerical quadrature and the classical tables, e.g. `C(1) = 0.7798934`,
/// `S(1) = 0.4382591`). Odd: `C(-x) = -C(x)`.
pub fn fresnel_integrals(x: f64) -> (f64, f64) {
    let ax = x.abs();
    let (c, s) = if ax < 1.5 {
        // C = sum_n (-1)^n (pi/2)^(2n) x^(4n+1) / ((2n)! (4n+1)),
        // S = sum_n (-1)^n (pi/2)^(2n+1) x^(4n+3) / ((2n+1)! (4n+3)).
        let p = std::f64::consts::FRAC_PI_2 * ax * ax;
        let (mut c, mut s) = (0.0, 0.0);
        let mut term = ax; // p^k x / k!
        for k in 0..200 {
            let sign = if (k / 2) % 2 == 0 { 1.0 } else { -1.0 };
            let contrib = sign * term / (2 * k + 1) as f64;
            if k % 2 == 0 {
                c += contrib;
            } else {
                s += contrib;
            }
            term *= p / (k + 1) as f64;
            if term < 1e-17 * c.abs().max(s.abs()).max(1e-300) {
                break;
            }
        }
        (c, s)
    } else {
        let pix2 = std::f64::consts::PI * ax * ax;
        let one = Complex64::new(1.0, 0.0);
        let mut b = Complex64::new(1.0, -pix2);
        let mut cc = Complex64::new(1.0 / f64::MIN_POSITIVE, 0.0);
        let mut d = one / b;
        let mut h = d;
        let mut n: i64 = -1;
        for _ in 2..1000 {
            n += 2;
            let a = -(n * (n + 1)) as f64;
            b += Complex64::new(4.0, 0.0);
            d = one / (d * a + b);
            cc = b + Complex64::new(a, 0.0) / cc;
            let del = cc * d;
            h *= del;
            if (del.re - 1.0).abs() + del.im.abs() < 1e-16 {
                break;
            }
        }
        h *= Complex64::new(ax, -ax);
        let cs = Complex64::new(0.5, 0.5) * (one - Complex64::from_polar(1.0, 0.5 * pix2) * h);
        (cs.re, cs.im)
    };
    if x < 0.0 {
        (-c, -s)
    } else {
        (c, s)
    }
}

/// Complex paraxial Fresnel field of an opaque half-plane (open for `x > 0`)
/// at reduced coordinate `v = x sqrt(2 / (lambda d))`, for unit incident
/// amplitude, in this module's `exp(-i pi lambda d f^2)` propagation
/// convention: `F = (1 - i)/2 [(1/2 + C(v)) + i (1/2 + S(v))]`.
pub fn knife_edge_field(v: f64) -> Complex64 {
    let (c, s) = fresnel_integrals(v);
    Complex64::new(0.5, -0.5) * Complex64::new(0.5 + c, 0.5 + s)
}

/// Intensity `|F(v)|^2 = [(1/2 + C)^2 + (1/2 + S)^2] / 2` behind an opaque
/// straight edge (0.25 at the geometric edge `v = 0`, first maximum 1.3704
/// at `v = 1.2172`, tending to 1 in the open region and 0 in the shadow).
pub fn knife_edge_intensity(v: f64) -> f64 {
    knife_edge_field(v).norm_sqr()
}

/// Dose across a single straight absorber edge, from the analytic Fresnel
/// solution (see [`fresnel_edge_profile_1d`]).
#[derive(Debug, Clone, Serialize)]
pub struct EdgeProfile {
    /// Lateral positions in nm relative to the geometric edge (`x > 0` open,
    /// `x < 0` under the absorber).
    pub x_nm: Vec<f64>,
    /// Depths in um below the resist surface.
    pub z_um: Vec<f64>,
    /// Absolute dose in kJ/cm^3: `dose_kj_cm3[k][i]` at depth `z_um[k]`,
    /// position `x_nm[i]`.
    pub dose_kj_cm3: Vec<Vec<f64>>,
    /// Open-field (`x -> +inf`) dose at each depth in kJ/cm^3.
    pub open_dose_kj_cm3: Vec<f64>,
    /// Absorber-side (`x -> -inf`) dose at each depth in kJ/cm^3.
    pub absorber_dose_kj_cm3: Vec<f64>,
}

impl EdgeProfile {
    /// Developed-edge position at each depth for a positive-tone threshold:
    /// the left end (linearly interpolated) of the contiguous above-threshold
    /// region that reaches the open side of the window. `None` when the open
    /// end is below threshold (not developed at that depth) or the whole
    /// window is above it.
    pub fn edge_positions_nm(&self, threshold_kj_cm3: f64) -> Vec<Option<f64>> {
        self.dose_kj_cm3
            .iter()
            .map(|row| {
                let n = row.len();
                if n < 2 || row[n - 1] < threshold_kj_cm3 {
                    return None;
                }
                let mut i = n - 1;
                while i > 0 && row[i - 1] >= threshold_kj_cm3 {
                    i -= 1;
                }
                if i == 0 {
                    return None;
                }
                let (d0, d1) = (row[i - 1], row[i]);
                let t = (threshold_kj_cm3 - d0) / (d1 - d0);
                Some(self.x_nm[i - 1] + t * (self.x_nm[i] - self.x_nm[i - 1]))
            })
            .collect()
    }

    /// Sidewall angle from vertical in degrees, `atan(dx_edge/dz)` from a
    /// least-squares line through the developed-edge positions (needs >= 2
    /// depths with an edge). Positive: the developed opening narrows with
    /// depth (the usual LIGA taper as the dose falls off).
    pub fn sidewall_angle_deg(&self, threshold_kj_cm3: f64) -> Option<f64> {
        let pts: Vec<(f64, f64)> = self
            .z_um
            .iter()
            .zip(self.edge_positions_nm(threshold_kj_cm3))
            .filter_map(|(z, x)| x.map(|x| (z * 1e3, x)))
            .collect();
        if pts.len() < 2 {
            return None;
        }
        let n = pts.len() as f64;
        let (mz, mx) = pts
            .iter()
            .fold((0.0, 0.0), |(a, b), (z, x)| (a + z / n, b + x / n));
        let (num, den) = pts.iter().fold((0.0, 0.0), |(a, b), (z, x)| {
            (a + (z - mz) * (x - mx), b + (z - mz) * (z - mz))
        });
        (den > 0.0).then(|| (num / den).atan().to_degrees())
    }

    /// Maximum `|dD/dx|` (kJ cm^-3 per nm) at each depth - the edge
    /// steepness.
    pub fn max_gradient_kj_cm3_nm(&self) -> Vec<f64> {
        self.dose_kj_cm3
            .iter()
            .map(|row| {
                row.windows(2)
                    .zip(self.x_nm.windows(2))
                    .map(|(d, x)| ((d[1] - d[0]) / (x[1] - x[0])).abs())
                    .fold(0.0, f64::max)
            })
            .collect()
    }
}

/// Fine 1D dose profile across a single straight absorber edge at the given
/// depths, for sidewall analysis at arbitrary (e.g. 10 nm) sampling.
///
/// Exact paraxial scalar diffraction of a half-plane: per energy bin
/// `U = a + (1 - a) F(v)`, `v = x sqrt(2 / (lambda (g + z)))`, with `a` the
/// complex absorber transmission and `F` the Fresnel-integral knife-edge
/// field ([`knife_edge_field`]); intensities are summed with the depth-dose
/// weights and scaled like [`expose_depth`] (the open side at the bottom
/// reaches the target dose). With `photoelectron_blur` each bin's intensity
/// is convolved with its Grun-range Gaussian (edge-replicated, so keep the
/// window a few ranges wide). `x` runs from `x_min_nm` to `x_max_nm` in steps
/// of `dx_nm`. Independent of any grid: this is the reference the 2D path is
/// validated against.
pub fn fresnel_edge_profile_1d(
    config: &DeepXrayConfig,
    x_min_nm: f64,
    x_max_nm: f64,
    dx_nm: f64,
    depths_um: &[f64],
) -> crate::error::Result<EdgeProfile> {
    config.check_energy_window()?;
    if dx_nm.is_nan()
        || dx_nm <= 0.0
        || x_max_nm.is_nan()
        || x_min_nm.is_nan()
        || x_max_nm <= x_min_nm
    {
        return Err(crate::error::LithographyError::InvalidParameter {
            name: "dx_nm",
            value: dx_nm,
            reason: "need dx_nm > 0 and x_max_nm > x_min_nm",
        });
    }
    let nx = ((x_max_nm - x_min_nm) / dx_nm).round() as usize + 1;
    if nx > 1_000_000 {
        return Err(crate::error::LithographyError::InvalidParameter {
            name: "dx_nm",
            value: dx_nm,
            reason: "more than 1e6 samples requested",
        });
    }
    let x_nm: Vec<f64> = (0..nx).map(|i| x_min_nm + i as f64 * dx_nm).collect();
    let bins = config.spectral_bins();
    let thickness = config.resist_thickness_um;
    let open_bottom: f64 = bins.iter().map(|b| b.dose_weight(thickness)).sum();
    let scale = if open_bottom > 0.0 {
        config.target_bottom_dose_kj_cm3 / open_bottom
    } else {
        0.0
    };
    let mut dose = Vec::with_capacity(depths_um.len());
    let mut open_dose = Vec::with_capacity(depths_um.len());
    let mut absorber_dose = Vec::with_capacity(depths_um.len());
    for &z in depths_um {
        let d_nm = (config.proximity_gap_um + z.max(0.0)) * 1e3;
        let mut row = vec![0.0; nx];
        let (mut open, mut floor) = (0.0, 0.0);
        for b in &bins {
            let w = scale * b.dose_weight(z.max(0.0));
            if w <= 0.0 {
                continue;
            }
            open += w;
            floor += w * b.absorber_intensity();
            let a = b.absorber;
            let one_minus_a = Complex64::new(1.0, 0.0) - a;
            let intensity: Vec<f64> = if d_nm > 0.0 {
                let k = (2.0 / (b.lambda_nm * d_nm)).sqrt();
                x_nm.iter()
                    .map(|x| (a + one_minus_a * knife_edge_field(x * k)).norm_sqr())
                    .collect()
            } else {
                x_nm.iter()
                    .map(|x| {
                        if *x > 0.0 {
                            1.0
                        } else if *x < 0.0 {
                            a.norm_sqr()
                        } else {
                            0.25
                        }
                    })
                    .collect()
            };
            let sigma = photoelectron_sigma_nm(config, b.energy_kev);
            let intensity = if sigma > 0.0 {
                gaussian_blur_1d(&intensity, sigma / dx_nm)
            } else {
                intensity
            };
            for (r, i) in row.iter_mut().zip(intensity) {
                *r += w * i;
            }
        }
        dose.push(row);
        open_dose.push(open);
        absorber_dose.push(floor);
    }
    Ok(EdgeProfile {
        x_nm,
        z_um: depths_um.to_vec(),
        dose_kj_cm3: dose,
        open_dose_kj_cm3: open_dose,
        absorber_dose_kj_cm3: absorber_dose,
    })
}

/// Fresnel dose profiles of a periodic 1D mask at the given depths, by 1D
/// angular-spectrum propagation (FFT; the pattern repeats with period
/// `open_fraction.len() * dx_nm`). `open_fraction[i]` in [0, 1] is the clear
/// fraction of sample `i` (1 = open, 0 = absorber). Returns absolute dose in
/// kJ/cm^3, `[depth][sample]`, scaled like [`expose_depth`]. Use it for
/// dense line/space sidewalls at fine sampling (e.g. 10 nm over a 10 um
/// pitch), which a 2D grid cannot afford.
pub fn fresnel_profile_1d(
    config: &DeepXrayConfig,
    open_fraction: &[f64],
    dx_nm: f64,
    depths_um: &[f64],
) -> crate::error::Result<Vec<Vec<f64>>> {
    config.check_energy_window()?;
    let n = open_fraction.len();
    if n < 2 || dx_nm.is_nan() || dx_nm <= 0.0 {
        return Err(crate::error::LithographyError::InvalidParameter {
            name: "open_fraction",
            value: n as f64,
            reason: "need at least 2 samples and dx_nm > 0",
        });
    }
    let mut planner = rustfft::FftPlanner::<f64>::new();
    let fwd = planner.plan_fft_forward(n);
    let inv = planner.plan_fft_inverse(n);
    let mut spectrum: Vec<Complex64> = open_fraction
        .iter()
        .map(|m| Complex64::new(m.clamp(0.0, 1.0), 0.0))
        .collect();
    fwd.process(&mut spectrum);
    let f2: Vec<f64> = (0..n)
        .map(|i| {
            let k = if i < n.div_ceil(2) {
                i as f64
            } else {
                i as f64 - n as f64
            };
            let f = k / (n as f64 * dx_nm);
            f * f
        })
        .collect();
    let bins = config.spectral_bins();
    let thickness = config.resist_thickness_um;
    let open_bottom: f64 = bins.iter().map(|b| b.dose_weight(thickness)).sum();
    let scale = if open_bottom > 0.0 {
        config.target_bottom_dose_kj_cm3 / open_bottom
    } else {
        0.0
    };
    let mut out = Vec::with_capacity(depths_um.len());
    let mut field = vec![Complex64::new(0.0, 0.0); n];
    for &z in depths_um {
        let d_nm = (config.proximity_gap_um + z.max(0.0)) * 1e3;
        let mut row = vec![0.0; n];
        for b in &bins {
            let w = scale * b.dose_weight(z.max(0.0));
            if w <= 0.0 {
                continue;
            }
            for ((out, m), q) in field.iter_mut().zip(&spectrum).zip(&f2) {
                *out = m * transfer_function(*q, b.lambda_nm, d_nm);
            }
            inv.process(&mut field);
            let a = b.absorber;
            let one_minus_a = Complex64::new(1.0, 0.0) - a;
            let intensity: Vec<f64> = field
                .iter()
                .map(|v| (a + one_minus_a * (v / n as f64)).norm_sqr())
                .collect();
            let sigma = photoelectron_sigma_nm(config, b.energy_kev);
            let intensity = if sigma > 0.0 {
                periodic_gaussian_blur_1d(&intensity, sigma, dx_nm)
            } else {
                intensity
            };
            for (r, i) in row.iter_mut().zip(intensity) {
                *r += w * i;
            }
        }
        out.push(row);
    }
    Ok(out)
}

/// Top/bottom dose ratio of a depth-dose exposure (the contrast the resist
/// process must span between the over-exposed top and the just-cleared bottom).
pub fn dose_ratio(exposure: &LigaExposure) -> f64 {
    exposure.dose_ratio
}

/// Maximum achievable aspect ratio: resist thickness divided by minimum
/// printable feature. Both are lengths; the result is dimensionless (e.g.
/// 500 um / 5 um = 100).
pub fn max_aspect_ratio(exposure: &LigaExposure, min_feature_nm: f64) -> f64 {
    let thickness_nm = exposure.z_um.last().copied().unwrap_or(0.0) * 1e3;
    if min_feature_nm > 0.0 {
        thickness_nm / min_feature_nm
    } else {
        f64::INFINITY
    }
}

/// Per-z-slice maximum lateral dose gradient `|dD/dx|` (per nm) along the
/// center row, evaluated at the crossing of `threshold`. This is the sidewall
/// steepness: a larger gradient at the threshold crossing means a sharper,
/// more vertical wall. Slices with no crossing return 0.
pub fn sidewall_dose_gradient(volume: &Grid3D<f64>, threshold: f64) -> Vec<f64> {
    let nx = volume.nx();
    let ny = volume.ny();
    let nz = volume.nz();
    let center_row = ny / 2;
    let dx_nm = volume.pixel_size_x();
    (0..nz)
        .map(|k| {
            let mut max_grad = 0.0_f64;
            for j in 0..nx.saturating_sub(1) {
                let a = volume.data[[k, center_row, j]];
                let b = volume.data[[k, center_row, j + 1]];
                let crosses =
                    (a - threshold) * (b - threshold) <= 0.0 && (a - threshold) != (b - threshold);
                if crosses {
                    let grad = (b - a).abs() / dx_nm;
                    if grad > max_grad {
                        max_grad = grad;
                    }
                }
            }
            max_grad
        })
        .collect()
}

/// Development depth (in nm) reached from the resist top at each `(y, x)`
/// column, for a positive-tone threshold model: scanning `z` downward from the
/// top, the depth of the *contiguous* run of voxels whose dose is at or above
/// `threshold_dose`, stopping at the first voxel below it.
///
/// There is no post-exposure bake: PMMA main-chain scission is a direct
/// radiolytic process, so the above-threshold region dissolves directly.
pub fn develop_depth(volume: &Grid3D<f64>, threshold_dose: f64) -> Array2<f64> {
    let nx = volume.nx();
    let ny = volume.ny();
    let nz = volume.nz();
    let dz_nm = volume.pixel_size_z();
    let mut depth = Array2::<f64>::zeros((ny, nx));
    for i in 0..ny {
        for j in 0..nx {
            let mut count = 0usize;
            for k in 0..nz {
                if volume.data[[k, i, j]] >= threshold_dose {
                    count += 1;
                } else {
                    break;
                }
            }
            depth[[i, j]] = count as f64 * dz_nm;
        }
    }
    depth
}

/// PMMA development rate `R = R0 (D/D0)^q` for dose `dose_kj_cm3`, reference
/// rate `r0_um_min` [um/min] at reference dose `d0_kj_cm3`, and contrast
/// exponent `q`. A power-law fit to GG/PGMEA developer data; returns 0 for
/// non-positive dose.
pub fn pmma_development_rate(dose_kj_cm3: f64, r0_um_min: f64, d0_kj_cm3: f64, q: f64) -> f64 {
    if dose_kj_cm3 <= 0.0 || d0_kj_cm3 <= 0.0 {
        return 0.0;
    }
    r0_um_min * (dose_kj_cm3 / d0_kj_cm3).powf(q)
}

/// Normalized discrete Gaussian kernel of standard deviation `sigma_px`
/// (radius `ceil(3 sigma)`).
fn gaussian_kernel(sigma_px: f64) -> (Vec<f64>, usize) {
    let radius = (3.0 * sigma_px).ceil() as usize;
    let mut kernel: Vec<f64> = (0..2 * radius + 1)
        .map(|i| {
            let d = i as f64 - radius as f64;
            (-d * d / (2.0 * sigma_px * sigma_px)).exp()
        })
        .collect();
    let sum: f64 = kernel.iter().sum();
    for k in &mut kernel {
        *k /= sum;
    }
    (kernel, radius)
}

/// 1D Gaussian blur with edge-replicated boundaries (`sigma_px` in samples).
fn gaussian_blur_1d(data: &[f64], sigma_px: f64) -> Vec<f64> {
    if sigma_px <= 0.0 || data.is_empty() {
        return data.to_vec();
    }
    let (kernel, radius) = gaussian_kernel(sigma_px);
    let n = data.len() as i64;
    (0..n)
        .map(|i| {
            kernel
                .iter()
                .enumerate()
                .map(|(k, kv)| data[(i + k as i64 - radius as i64).clamp(0, n - 1) as usize] * kv)
                .sum()
        })
        .collect()
}

/// 1D periodic Gaussian blur by FFT (`sigma_nm` over sample spacing `dx_nm`).
fn periodic_gaussian_blur_1d(data: &[f64], sigma_nm: f64, dx_nm: f64) -> Vec<f64> {
    let n = data.len();
    let mut planner = rustfft::FftPlanner::<f64>::new();
    let fwd = planner.plan_fft_forward(n);
    let inv = planner.plan_fft_inverse(n);
    let mut buf: Vec<Complex64> = data.iter().map(|v| Complex64::new(*v, 0.0)).collect();
    fwd.process(&mut buf);
    let s2 = 2.0 * std::f64::consts::PI.powi(2) * sigma_nm * sigma_nm;
    for (i, v) in buf.iter_mut().enumerate() {
        let k = if i < n.div_ceil(2) {
            i as f64
        } else {
            i as f64 - n as f64
        };
        let f = k / (n as f64 * dx_nm);
        *v *= (-s2 * f * f).exp() / n as f64;
    }
    inv.process(&mut buf);
    buf.iter().map(|v| v.re).collect()
}

/// Separable Gaussian blur of a 2D field with clamped (edge-replicated)
/// boundaries; `sigma_px` is the standard deviation in pixels. Mirrors
/// [`crate::resist::peb_diffuse`]. A `sigma_px <= 0` is a no-op (returns a copy).
fn gaussian_blur_2d(img: &Array2<f64>, sigma_px: f64) -> Array2<f64> {
    if sigma_px <= 0.0 {
        return img.clone();
    }
    let (kernel, radius) = gaussian_kernel(sigma_px);
    let (ny, nx) = img.dim();
    let mut temp = Array2::<f64>::zeros((ny, nx));
    // Along x.
    for i in 0..ny {
        for j in 0..nx {
            let mut val = 0.0;
            for (k, &kv) in kernel.iter().enumerate() {
                let jj = (j as i64 + k as i64 - radius as i64).clamp(0, nx as i64 - 1) as usize;
                val += img[[i, jj]] * kv;
            }
            temp[[i, j]] = val;
        }
    }
    // Along y.
    let mut out = Array2::<f64>::zeros((ny, nx));
    for i in 0..ny {
        for j in 0..nx {
            let mut val = 0.0;
            for (k, &kv) in kernel.iter().enumerate() {
                let ii = (i as i64 + k as i64 - radius as i64).clamp(0, ny as i64 - 1) as usize;
                val += temp[[ii, j]] * kv;
            }
            out[[i, j]] = val;
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mask::{MaskFeature, MaskType};
    use approx::assert_relative_eq;

    fn mono_config(energy_kev: f64, thickness_um: f64) -> DeepXrayConfig {
        // Monochromatic, no filters, transparent membrane (0 um): isolates the
        // resist exponential.
        DeepXrayConfig {
            spectrum: XraySpectrum::Tabulated {
                energies_kev: vec![energy_kev],
                relative_flux: vec![1.0],
            },
            filters: Vec::new(),
            absorber: BeamFilter::new(Compound::gold(19.3), 20.0),
            membrane: BeamFilter::new(Compound::beryllium(1.85), 0.0),
            resist: Compound::pmma(),
            resist_thickness_um: thickness_um,
            proximity_gap_um: 10.0,
            target_bottom_dose_kj_cm3: 3.0,
            damage_dose_kj_cm3: 20.0,
            photoelectron_blur: false,
            energy_bins: 32,
            proximity_model: ProximityModel::Fresnel,
            energy_range_kev: None,
        }
    }

    fn bm_config() -> DeepXrayConfig {
        DeepXrayConfig::pmma_default(XraySpectrum::BendingMagnet {
            critical_energy_kev: 6.234,
        })
    }

    /// Dark-field mask: one clear vertical stripe of width `w_nm` centred at
    /// x = 0 (absorber elsewhere), full field height.
    fn slit_mask(w_nm: f64) -> Mask {
        Mask {
            mask_type: MaskType::Binary,
            features: vec![MaskFeature::Rect {
                x: 0.0,
                y: 0.0,
                w: w_nm,
                h: 1e9,
            }],
            dark_field: true,
        }
    }

    fn clear_mask() -> Mask {
        Mask {
            mask_type: MaskType::Binary,
            features: Vec::new(),
            dark_field: false,
        }
    }

    // ---- depth dose ----------------------------------------------------

    #[test]
    fn test_monochromatic_depth_dose_exponential() {
        let energy = 5.0;
        let thickness = 200.0;
        let config = mono_config(energy, thickness);
        let exposure = expose_depth(&config);

        // Single energy => u(z) = C exp(-mu z), so top/bottom = exp(mu T).
        let mu = config.resist.mu_per_um(energy);
        let expected = (mu * thickness).exp();
        assert_relative_eq!(exposure.dose_ratio, expected, max_relative = 1e-9);

        // Every sampled point matches the exact exponential.
        let scale = exposure.scale;
        let c0 = exposure.dose_kj_cm3[0] / scale; // relative u(0)
        for (z, d) in exposure.z_um.iter().zip(exposure.dose_kj_cm3.iter()) {
            let u = d / scale;
            assert_relative_eq!(u, c0 * (-mu * z).exp(), max_relative = 1e-9);
        }
        // Bottom hits the target dose; the deposit uses mu_en, not mu.
        assert_relative_eq!(exposure.bottom_dose_kj_cm3, 3.0, max_relative = 1e-9);
        assert_relative_eq!(
            c0,
            energy * config.resist.mu_en_per_um(energy),
            max_relative = 1e-12
        );
        // Relative spectra carry no absolute time.
        assert!(exposure.exposure_time_estimate.is_none());
        assert!(exposure.top_dose_rate_kj_cm3_s.is_none());
    }

    #[test]
    fn test_spectral_hardening_decay_slows_with_depth() {
        let config = bm_config();
        let profile = depth_dose(&config);
        // Local decay rate -d ln u / dz between successive samples.
        let rate = |i: usize| {
            let (z0, u0) = profile[i];
            let (z1, u1) = profile[i + 1];
            -(u1.ln() - u0.ln()) / (z1 - z0)
        };
        let shallow = rate(0);
        let mid = rate(profile.len() / 2);
        let deep = rate(profile.len() - 2);
        // Beam hardens with depth: the soft, strongly-absorbed photons are
        // gone, so the effective decay rate falls monotonically.
        assert!(
            shallow > mid && mid > deep,
            "decay should slow with depth: {shallow} > {mid} > {deep}"
        );
        let exposure = expose_depth(&config);
        assert!(exposure.mean_energy_bottom_kev > exposure.mean_energy_top_kev);
    }

    #[test]
    fn test_filter_hardens_beam_lowers_ratio() {
        let base = bm_config();
        let ratio_base = expose_depth(&base).dose_ratio;

        let mut filtered = bm_config();
        filtered
            .filters
            .push(BeamFilter::new(Compound::beryllium(1.85), 100.0));
        let ratio_filtered = expose_depth(&filtered).dose_ratio;

        assert!(
            ratio_filtered < ratio_base,
            "Be filter should harden the beam and lower the top/bottom ratio: \
             {ratio_filtered} < {ratio_base}"
        );
    }

    #[test]
    fn test_sample_weights_sum_to_one() {
        let bm = XraySpectrum::BendingMagnet {
            critical_energy_kev: 6.234,
        };
        let sum: f64 = bm.sample(50, 0.5, 30.0).iter().map(|(_, w)| w).sum();
        assert_relative_eq!(sum, 1.0, max_relative = 1e-12);

        let tab = XraySpectrum::Tabulated {
            energies_kev: vec![2.0, 5.0, 10.0],
            relative_flux: vec![1.0, 2.0, 1.0],
        };
        let sum: f64 = tab.sample(10, 0.1, 20.0).iter().map(|(_, w)| w).sum();
        assert_relative_eq!(sum, 1.0, max_relative = 1e-12);
    }

    #[test]
    fn test_bm_log_bins_are_photon_numbers() {
        // On log-spaced bins the photon weight is G1(y). Over the whole
        // spectrum the mean photon energy is E_c Int G1 / Int G1/y
        // = E_c (8 pi / 9 sqrt 3) / (5 pi / 3) = 8/(15 sqrt 3) E_c = 0.307920 E_c (closed forms of
        // Int x^2 K_5/3 / 2 and Int x K_5/3). The low-energy tail below
        // y0 = 1e-6 is added analytically from G1 ~ 2.1495 y^(1/3):
        // Int_0^y0 G1/y dy = 6.4485 y0^(1/3).
        let e_c = 5.0;
        let bm = XraySpectrum::BendingMagnet {
            critical_energy_kev: e_c,
        };
        let s = bm.raw_samples(3000, 1e-6 * e_c, 30.0 * e_c);
        let photons: f64 = s.iter().map(|(_, w)| w).sum::<f64>() + 6.4485 * 1e-2;
        let energy: f64 = s.iter().map(|(e, w)| e * w).sum();
        assert_relative_eq!(
            photons,
            5.0 * std::f64::consts::PI / 3.0,
            max_relative = 1e-3
        );
        let int_g1 = 8.0 * std::f64::consts::PI / (9.0 * 3f64.sqrt());
        assert_relative_eq!(energy / e_c, int_g1, max_relative = 1e-3);
        assert_relative_eq!(
            energy / photons / e_c,
            8.0 / (15.0 * 3f64.sqrt()),
            max_relative = 1e-3
        );
    }

    // ---- absolute flux -------------------------------------------------

    fn anka_like() -> BendingMagnetBeamline {
        BendingMagnetBeamline {
            electron_energy_gev: 2.5,
            field_t: 1.5,
            ring_current_ma: 200.0,
            source_distance_m: 15.0,
            horizontal_acceptance_mrad: 5.0,
            vertical_scan_mm: 50.0,
        }
    }

    #[test]
    fn test_bm_flux_constant_fixture() {
        // The X-ray Data Booklet constant 2.457e13 photons s^-1 mrad^-1
        // (0.1% BW)^-1 GeV^-1 A^-1, rebuilt from CODATA constants.
        assert_relative_eq!(bm_flux_constant(), 2.457e13, max_relative = 5e-4);
    }

    #[test]
    fn test_bm_power_matches_radiated_power_formula() {
        // Independent formula: P/theta = 14.08 E^4[GeV] I[A] / rho[m] W/mrad
        // with rho = 3.3356 E/B -> 19.787 W/mrad for 2.5 GeV, 1.5 T, 200 mA.
        let b = anka_like();
        let rho = 3.3356 * 2.5 / 1.5;
        let sands = 14.08 * 2.5f64.powi(4) * 0.2 / rho;
        assert_relative_eq!(b.power_per_mrad_w(), sands, max_relative = 2e-3);
        // Numerical integral of the sampled absolute spectrum agrees.
        let spec = XraySpectrum::BendingMagnetBeamline(BendingMagnetBeamline {
            source_distance_m: 1.0,
            vertical_scan_mm: 1.0,
            ..anka_like()
        });
        let s = spec
            .sample_absolute(2000, 1e-3, 40.0 * b.critical_energy_kev())
            .unwrap();
        let p: f64 = s.iter().map(|(e, n)| e * n * J_PER_KEV).sum();
        assert_relative_eq!(p, b.power_per_mrad_w(), max_relative = 1e-3);
        assert_relative_eq!(
            b.incident_power_w(),
            5.0 * b.power_per_mrad_w(),
            max_relative = 1e-12
        );
        assert_relative_eq!(b.field_width_mm(), 75.0, max_relative = 1e-12);
    }

    #[test]
    fn test_flux_density_scaling_and_geometry() {
        let b = anka_like();
        // Phi = (sqrt3/2pi) alpha gamma (I/e) G1(y) 1e-3 [per mrad] / (E_keV L H)
        // per unit ln E, with gamma = 1 + 2500 MeV / 0.51099895 MeV (written
        // out independently of physics::bm_flux_per_mrad).
        let e = 8.0;
        let y = e / b.critical_energy_kev();
        let gamma = 1.0 + 2500.0 / 0.510_998_95;
        let expected =
            3f64.sqrt() / (2.0 * std::f64::consts::PI) * 7.297_352_569_3e-3 * gamma * 0.2
                / 1.602_176_634e-19
                * 1e-3
                * bm_universal_flux(y)
                / (e * 15.0 * 50.0);
        assert_relative_eq!(b.flux_density(e), expected, max_relative = 1e-12);
        // Identical to the source model's absolute BM flux (per 0.1% BW).
        let src = SynchrotronSource::liga_bending_magnet();
        let from_src = BendingMagnetBeamline::from_synchrotron(&src, 15.0, 5.0, 50.0).unwrap();
        for e in [1.0, 6.0, 20.0] {
            assert_relative_eq!(
                from_src.photons_per_s_mrad_per_ln_e(e) * 1e-3,
                src.bm_flux_per_mrad(e),
                max_relative = 1e-12
            );
        }
        // Doubling the distance or the scan height halves the flux density.
        let far = BendingMagnetBeamline {
            source_distance_m: 30.0,
            ..anka_like()
        };
        assert_relative_eq!(
            far.flux_density(e),
            0.5 * b.flux_density(e),
            max_relative = 1e-12
        );
        assert!(b.warnings().is_empty());
        let short_scan = BendingMagnetBeamline {
            vertical_scan_mm: 2.0,
            ..anka_like()
        };
        assert_eq!(short_scan.warnings().len(), 1);
        assert!(anka_like().validate().is_ok());
        assert!(BendingMagnetBeamline {
            ring_current_ma: 0.0,
            ..anka_like()
        }
        .validate()
        .is_err());
    }

    #[test]
    fn test_flux_density_table_voronoi_weights() {
        // Constant density 1e10 on 5 points 0.5 keV apart: each point carries
        // 0.5 keV of the density (end points use the neighbour spacing).
        let table: Vec<(f64, f64)> = (0..5).map(|i| (1.0 + 0.5 * i as f64, 1e10)).collect();
        let spec = XraySpectrum::from_flux_density(&table).unwrap();
        assert!(spec.is_absolute());
        let s = spec.sample_absolute(0, 0.0, 100.0).unwrap();
        assert_eq!(s.len(), 5);
        for (_, n) in &s {
            assert_relative_eq!(*n, 0.5e10, max_relative = 1e-12);
        }
        // Invalid tables are rejected.
        assert!(XraySpectrum::from_flux_density(&[(1.0, 1.0)]).is_err());
        assert!(XraySpectrum::from_flux_density(&[(2.0, 1.0), (1.0, 1.0)]).is_err());
        assert!(XraySpectrum::from_flux_density(&[(1.0, -1.0), (2.0, 1.0)]).is_err());
    }

    #[test]
    fn test_absolute_dose_rate_and_exposure_time_fixture() {
        // Two lines at 7.999 / 8.001 keV, 0.5e12 photons s^-1 mm^-2 each
        // (density 2.5e14 per keV over 0.002 keV cells), 200 um PMMA, no
        // membrane. Hand calculation with the NIST element tables (PMMA
        // mixture mu_en/rho = 6.1122 cm^2/g, mu/rho = 6.4934 at 8 keV):
        // top rate = sum N E[J] mu_en[1/mm] = 9.3228e-4 kJ cm^-3 s^-1,
        // bottom = 7.9878e-4, t = 3 / 7.9878e-4 = 3755.7 s.
        let mut config = mono_config(8.0, 200.0);
        config.spectrum =
            XraySpectrum::from_flux_density(&[(7.999, 2.5e14), (8.001, 2.5e14)]).unwrap();
        let exposure = expose_depth(&config);
        assert_relative_eq!(
            exposure.top_dose_rate_kj_cm3_s.unwrap(),
            9.3228e-4,
            max_relative = 2e-4
        );
        assert_relative_eq!(
            exposure.bottom_dose_rate_kj_cm3_s.unwrap(),
            7.98784e-4,
            max_relative = 2e-4
        );
        assert_relative_eq!(
            exposure.exposure_time_estimate.unwrap(),
            3755.71,
            max_relative = 2e-4
        );
        // Consistency: rate x time reproduces the scaled top dose.
        assert_relative_eq!(
            exposure.top_dose_rate_kj_cm3_s.unwrap() * exposure.exposure_time_estimate.unwrap(),
            exposure.top_dose_kj_cm3,
            max_relative = 1e-6
        );
        // Power density at the resist: 1e12 photons x 8 keV = 1.2817e-3 W/mm^2.
        assert_relative_eq!(
            exposure.resist_power_density_w_mm2.unwrap(),
            1.28174e-3,
            max_relative = 1e-4
        );
        assert!(exposure.exposure_charge_ma_h.is_none());
    }

    #[test]
    fn test_bending_magnet_exposure_time_scaling() {
        let mut config = bm_config();
        config.spectrum = XraySpectrum::BendingMagnetBeamline(anka_like());
        let e1 = expose_depth(&config);
        let t1 = e1.exposure_time_estimate.unwrap();
        assert!(t1 > 0.0 && t1.is_finite());
        // Same spectral shape as the relative bending magnet: identical
        // relative depth dose.
        let rel = expose_depth(&DeepXrayConfig::pmma_default(XraySpectrum::BendingMagnet {
            critical_energy_kev: anka_like().critical_energy_kev(),
        }));
        assert_relative_eq!(e1.dose_ratio, rel.dose_ratio, max_relative = 1e-9);
        // Doubling the current halves the time; the mA h charge is invariant.
        config.spectrum = XraySpectrum::BendingMagnetBeamline(BendingMagnetBeamline {
            ring_current_ma: 400.0,
            ..anka_like()
        });
        let e2 = expose_depth(&config);
        assert_relative_eq!(
            e2.exposure_time_estimate.unwrap(),
            0.5 * t1,
            max_relative = 1e-9
        );
        assert_relative_eq!(
            e2.exposure_charge_ma_h.unwrap(),
            e1.exposure_charge_ma_h.unwrap(),
            max_relative = 1e-9
        );
        // Doubling the scan height doubles the time.
        config.spectrum = XraySpectrum::BendingMagnetBeamline(BendingMagnetBeamline {
            vertical_scan_mm: 100.0,
            ..anka_like()
        });
        assert_relative_eq!(
            expose_depth(&config).exposure_time_estimate.unwrap(),
            2.0 * t1,
            max_relative = 1e-9
        );
    }

    #[test]
    fn test_bending_magnet_absolute_fixture_independent_numpy() {
        // Independent numpy re-implementation (G1 by fine trapezoid
        // quadrature of Int exp(-y cosh t) cosh(5t/3)/cosh t, NIST tables
        // parsed separately, Henke f2 below 1 keV; gamma = 1 + E/mc^2 as in
        // physics::bm_flux_per_mrad): 2.5 GeV / 1.5 T /
        // 200 mA, 15 m, 50 mm scan, 2 um Ti membrane, 500 um PMMA, 3 kJ/cm^3
        // bottom dose, 100 log bins over [0.1, 8] E_c.
        let mut config = bm_config();
        config.spectrum = XraySpectrum::BendingMagnetBeamline(anka_like());
        let e = expose_depth(&config);
        assert_relative_eq!(e.dose_ratio, 20.77700, max_relative = 1e-5);
        assert_relative_eq!(
            e.exposure_time_estimate.unwrap(),
            605.265,
            max_relative = 1e-5
        );
        assert_relative_eq!(
            e.bottom_dose_rate_kj_cm3_s.unwrap(),
            4.956509e-3,
            max_relative = 1e-5
        );
        assert_relative_eq!(
            e.resist_power_density_w_mm2.unwrap(),
            1.874934e-2,
            max_relative = 1e-5
        );
        assert_relative_eq!(e.mean_energy_top_kev, 2.8520, max_relative = 1e-4);
        assert_relative_eq!(e.mean_energy_bottom_kev, 7.5699, max_relative = 1e-4);
        // A 20 um Al filter hardens the beam: ratio 2.85671, t = 995.933 s.
        config
            .filters
            .push(BeamFilter::new(Compound::aluminum(2.70), 20.0));
        let f = expose_depth(&config);
        assert_relative_eq!(f.dose_ratio, 2.85671, max_relative = 1e-5);
        assert_relative_eq!(
            f.exposure_time_estimate.unwrap(),
            995.933,
            max_relative = 1e-5
        );
    }

    #[test]
    fn test_window_power_fraction_reports_clipping() {
        // Default stack (2 um Ti): the photons below 0.1 E_c never reach the
        // resist; only the tail above 8 E_c (~0.1% of the power: Int_8^inf
        // G1 dy ~ sqrt(4 pi) e^-8 relative to 8 pi / 9 sqrt 3) is clipped.
        let e = expose_depth(&bm_config());
        assert!(
            (0.995..1.0).contains(&e.window_power_fraction),
            "{}",
            e.window_power_fraction
        );
        assert!(e.warnings.is_empty());
        // No membrane, no filter: the soft tail below 0.1 E_c carries ~4.6%
        // of the bending-magnet power (Int_0^0.1 G1 dy / (8 pi / 9 sqrt 3),
        // G1 ~ 2.15 y^(1/3)) and reaches the resist - reported and warned.
        let mut bare = bm_config();
        bare.membrane = BeamFilter::new(Compound::beryllium(1.848), 0.0);
        let e = expose_depth(&bare);
        assert!(
            (0.93..0.97).contains(&e.window_power_fraction),
            "{}",
            e.window_power_fraction
        );
        assert!(e
            .warnings
            .iter()
            .any(|w| w.contains("outside the sampled energy window")));
        // An explicit wide window recovers it.
        bare.energy_range_kev = Some((0.03, 60.0));
        assert!(expose_depth(&bare).window_power_fraction > 0.998);
        // Tables are integrated over all their points.
        let mut tab = mono_config(8.0, 100.0);
        tab.spectrum = XraySpectrum::Tabulated {
            energies_kev: vec![4.0, 8.0],
            relative_flux: vec![1.0, 1.0],
        };
        tab.energy_range_kev = Some((6.0, 10.0));
        let f = expose_depth(&tab).window_power_fraction;
        assert_relative_eq!(f, 8.0 / 12.0, max_relative = 1e-12);
    }

    #[test]
    fn test_from_synchrotron_bending_magnet() {
        let bm = SynchrotronSource::liga_bending_magnet();
        let spectrum = XraySpectrum::from_synchrotron(&bm).unwrap();
        match spectrum {
            XraySpectrum::BendingMagnet {
                critical_energy_kev,
            } => assert_relative_eq!(critical_energy_kev, 6.234, epsilon = 1e-3),
            _ => panic!("expected bending magnet"),
        }
        let abs = XraySpectrum::from_synchrotron_beamline(&bm, 15.0, 5.0, 50.0).unwrap();
        assert!(abs.is_absolute());
        match abs {
            XraySpectrum::BendingMagnetBeamline(b) => {
                assert_relative_eq!(b.ring_current_ma, 200.0);
                assert_relative_eq!(b.critical_energy_kev(), 6.234, epsilon = 1e-3);
            }
            _ => panic!("expected absolute bending magnet"),
        }
        // Undulator beamlines have no white-beam spectrum.
        let und = SynchrotronSource::compact_euv_undulator().unwrap();
        assert!(XraySpectrum::from_synchrotron(&und).is_none());
        assert!(XraySpectrum::from_synchrotron_beamline(&und, 15.0, 5.0, 50.0).is_none());
    }

    // ---- Fresnel diffraction -------------------------------------------

    #[test]
    fn test_fresnel_integrals_fixture() {
        // Values from numerical quadrature (Simpson, 4e6 intervals) and the
        // classical tables: C(1) = 0.7798934004, S(1) = 0.4382591474.
        for (x, c, s) in [
            (0.5, 0.492344225871, 0.064732432860),
            (1.0, 0.779893400377, 0.438259147390),
            (1.4, 0.543095783546, 0.713525077363),
            (1.6, 0.365461683440, 0.638887683509),
            (2.0, 0.488253406075, 0.343415678364),
            (3.0, 0.605720789298, 0.496312998967),
            (8.0, 0.499802180377, 0.460214214393),
        ] {
            let (cc, ss) = fresnel_integrals(x);
            assert!(
                (cc - c).abs() < 1e-11 && (ss - s).abs() < 1e-11,
                "x={x}: {cc} {ss}"
            );
            let (cn, sn) = fresnel_integrals(-x);
            assert_eq!((cn, sn), (-cc, -ss));
        }
        let (c, s) = fresnel_integrals(1e4);
        assert!((c - 0.5).abs() < 1e-4 && (s - 0.5).abs() < 1e-4);
    }

    #[test]
    fn test_knife_edge_intensity_fixtures() {
        // Opaque straight edge: 1/4 of the open intensity at the geometric
        // edge, first maximum 1.3704 at v = 1.2172, first minimum 0.7783 at
        // v = 1.8725 (textbook Fresnel straight-edge values, recomputed with
        // numpy).
        assert_relative_eq!(knife_edge_intensity(0.0), 0.25, max_relative = 1e-12);
        let (mut vmax, mut imax) = (0.0, 0.0);
        let mut v = 1.0;
        while v < 1.4 {
            let i = knife_edge_intensity(v);
            if i > imax {
                (vmax, imax) = (v, i);
            }
            v += 1e-5;
        }
        assert!((vmax - 1.2172).abs() < 1e-3, "first maximum at v = {vmax}");
        assert_relative_eq!(imax, 1.370443, max_relative = 1e-5);
        assert_relative_eq!(knife_edge_intensity(1.87252), 0.778251, max_relative = 1e-5);
        for (v, i) in [
            (-2.0, 0.012328316126),
            (-0.5, 0.094758232941),
            (1.0, 1.259228671995),
            (3.0, 1.107629027898),
        ] {
            assert_relative_eq!(knife_edge_intensity(v), i, max_relative = 1e-7);
        }
        // Far into the open region the field tends to 1, into the shadow 0.
        assert!((knife_edge_field(200.0) - Complex64::new(1.0, 0.0)).norm() < 2e-3);
        assert!(knife_edge_field(-200.0).norm() < 2e-3);
    }

    /// FFT propagation of a wide periodic slit with this module's transfer
    /// function; returns (x_nm, complex field) for unit amplitude.
    fn propagate_slit_1d(
        n: usize,
        dx: f64,
        width: f64,
        lambda: f64,
        d: f64,
    ) -> (Vec<f64>, Vec<Complex64>) {
        let x: Vec<f64> = (0..n)
            .map(|i| (i as f64 - n as f64 / 2.0 + 0.5) * dx)
            .collect();
        let mut buf: Vec<Complex64> = x
            .iter()
            .map(|xi| Complex64::new(if xi.abs() < width / 2.0 { 1.0 } else { 0.0 }, 0.0))
            .collect();
        let mut planner = rustfft::FftPlanner::<f64>::new();
        planner.plan_fft_forward(n).process(&mut buf);
        for (i, v) in buf.iter_mut().enumerate() {
            let k = if i < n.div_ceil(2) {
                i as f64
            } else {
                i as f64 - n as f64
            };
            let f = k / (n as f64 * dx);
            *v *= transfer_function(f * f, lambda, d) / n as f64;
        }
        planner.plan_fft_inverse(n).process(&mut buf);
        (x, buf)
    }

    #[test]
    fn test_knife_edge_field_matches_fft_propagation_including_phase() {
        // A slit 400 Fresnel scales wide: near its left edge (x = -W/2) the
        // FFT field must equal the analytic half-plane field F(v) with
        // v = (x + W/2) sqrt(2/(lambda d)) - amplitude AND phase, which pins
        // the sign convention used for the complex absorber.
        let (lambda, d): (f64, f64) = (0.2, 1e5); // nm: 0.2 nm over 100 um, sqrt = 141 nm
        let (n, dx) = (1 << 16, 2.0);
        let width = 400.0 * (lambda * d).sqrt();
        let (x, field) = propagate_slit_1d(n, dx, width, lambda, d);
        let k = (2.0 / (lambda * d)).sqrt();
        let mut worst = 0.0_f64;
        for (xi, u) in x.iter().zip(field.iter()) {
            let rel = xi + width / 2.0;
            if rel.abs() < 600.0 {
                worst = worst.max((u - knife_edge_field(rel * k)).norm());
            }
        }
        assert!(worst < 5e-3, "max |U_fft - F| = {worst}");
    }

    #[test]
    fn test_edge_profile_opaque_monochromatic_matches_analytic() {
        // 200 um Au at 5 keV is opaque (|a|^2 ~ e^-257): the surface dose
        // across the edge is the open dose times |F(v)|^2.
        let mut config = mono_config(5.0, 50.0);
        config.absorber = BeamFilter::new(Compound::gold(19.3), 200.0);
        config.proximity_gap_um = 100.0;
        let lambda = wavelength_nm(5.0);
        let d = 100.0e3;
        let p = fresnel_edge_profile_1d(&config, -500.0, 800.0, 10.0, &[0.0]).unwrap();
        let open = p.open_dose_kj_cm3[0];
        assert!(p.absorber_dose_kj_cm3[0] < 1e-12 * open);
        let k = (2.0 / (lambda * d)).sqrt();
        for (x, dose) in p.x_nm.iter().zip(p.dose_kj_cm3[0].iter()) {
            assert_relative_eq!(dose / open, knife_edge_intensity(x * k), epsilon = 1e-9);
        }
        // 1/4 at the geometric edge.
        let i0 = p.x_nm.iter().position(|x| x.abs() < 1e-9).unwrap();
        assert_relative_eq!(p.dose_kj_cm3[0][i0] / open, 0.25, max_relative = 1e-9);
    }

    #[test]
    fn test_fresnel_edge_width_vs_gaussian_blur_scale() {
        // Opaque edge 10%-90% rise: Fresnel v in [-0.470752, 0.701322]
        // -> 0.82878 sqrt(lambda d); the legacy Gaussian (sigma = sqrt(lambda
        // g)/2) gives 2 * 1.28155 * sigma = 1.28155 sqrt(lambda g). Same
        // scale, Gaussian ~1.55x wider and without the 1.37 overshoot.
        let mut config = mono_config(8.0, 50.0);
        config.absorber = BeamFilter::new(Compound::gold(19.3), 200.0);
        config.proximity_gap_um = 100.0;
        let s = (wavelength_nm(8.0) * 1e5).sqrt();
        let p = fresnel_edge_profile_1d(&config, -3.0 * s, 3.0 * s, s / 2000.0, &[0.0]).unwrap();
        let open = p.open_dose_kj_cm3[0];
        let cross = |level: f64| {
            let row = &p.dose_kj_cm3[0];
            let i = row.iter().position(|d| d / open >= level).unwrap();
            p.x_nm[i]
        };
        let w_fresnel = cross(0.9) - cross(0.1);
        assert_relative_eq!(w_fresnel / s, 0.828781, max_relative = 2e-3);
        let w_gauss = 2.0 * 1.281_551_565_5 * proximity_sigma_nm(8.0, 100.0);
        assert_relative_eq!(w_gauss / s, 1.281_551_565_5 * 1.0, max_relative = 1e-9);
        assert!((1.3..1.8).contains(&(w_gauss / w_fresnel)));
    }

    #[test]
    fn test_fresnel_2d_matches_1d_periodic_profile() {
        // The same discrete, periodic problem solved by the 2D FFT path (a
        // stripe mask on the LIGA grid) and the 1D FFT tool must agree to
        // round-off: one clear stripe of 64 px in a 256 px field (10 nm px).
        let mut config = mono_config(8.0, 50.0);
        config.proximity_gap_um = 20.0;
        let grid = GridConfig::new(256, 10.0).unwrap();
        let mask = slit_mask(640.0);
        let img = shadow_image(&config, &mask, &grid).unwrap();
        let open = open_fraction(&mask, &grid);
        let row: Vec<f64> = (0..256).map(|j| open[[128, j]]).collect();
        let prof = fresnel_profile_1d(&config, &row, 10.0, &[0.0]).unwrap();
        // shadow_image is normalized to the open dose; the 1D tool is in
        // absolute kJ/cm^3 - normalize by the open-column surface dose.
        let exposure = expose_depth(&config);
        let open_top = exposure.top_dose_kj_cm3;
        for j in 0..256 {
            assert_relative_eq!(img[[128, j]], prof[0][j] / open_top, epsilon = 1e-9);
        }
        // The stripe is open (dose ~1 with ringing), the rest leaks.
        assert!(img[[128, 128]] > 0.8 && img[[128, 10]] < 0.1);
    }

    #[test]
    fn test_energy_window_outside_data_range() {
        // A window reaching below 30 eV is never evaluated there: the
        // Result entry points reject it, expose_depth truncates and warns,
        // and the truncated run equals an explicit (0.03, 60) keV window.
        let mut config = bm_config();
        config.energy_range_kev = Some((0.001, 60.0));
        let grid = GridConfig::new(32, 50.0).unwrap();
        assert!(shadow_image(&config, &clear_mask(), &grid).is_err());
        assert!(expose_volumetric(&config, &clear_mask(), &grid, 4).is_err());
        assert!(fresnel_profile_1d(&config, &[1.0, 0.0], 10.0, &[0.0]).is_err());
        assert_eq!(config.energy_window_kev(), (0.03, 60.0));
        let e = expose_depth(&config);
        assert!(e.warnings.iter().any(|w| w.contains("energy_range_kev")));
        let mut ok = config.clone();
        ok.energy_range_kev = Some((0.03, 60.0));
        assert!(ok.check_energy_window().is_ok());
        assert_eq!(expose_depth(&ok).dose_kj_cm3, e.dose_kj_cm3);
        // Inverted and above-20-MeV windows are rejected too.
        ok.energy_range_kev = Some((10.0, 5.0));
        assert!(ok.check_energy_window().is_err());
        ok.energy_range_kev = Some((1.0, 3.0e4));
        assert!(ok.check_energy_window().is_err());
    }

    #[test]
    fn test_fresnel_open_field_is_unity_and_absorber_floor_exact() {
        let config = DeepXrayConfig {
            energy_bins: 24,
            ..bm_config()
        };
        let grid = GridConfig::new(32, 50.0).unwrap();
        let img = shadow_image(&config, &clear_mask(), &grid).unwrap();
        for v in img.iter() {
            assert_relative_eq!(*v, 1.0, epsilon = 1e-12);
        }
        // Fully absorbing mask: uniform floor = dose-weighted |a|^2.
        let dark = Mask {
            mask_type: MaskType::Binary,
            features: Vec::new(),
            dark_field: true,
        };
        let img = shadow_image(&config, &dark, &grid).unwrap();
        let bins = config.spectral_bins();
        let (num, den) = bins.iter().fold((0.0, 0.0), |(n, d), b| {
            let w = b.dose_weight(0.0);
            (n + w * b.absorber_intensity(), d + w)
        });
        for v in img.iter() {
            assert_relative_eq!(*v, num / den, max_relative = 1e-9);
        }
        assert!(num / den > 0.0 && num / den < 0.1);
    }

    #[test]
    fn test_fresnel_propagation_conserves_mean_intensity() {
        // |H| = 1 for propagating waves: the field-averaged intensity of each
        // bin is independent of distance (Parseval). Monochromatic, so the
        // shadow image mean equals mean(m + (1 - m)|a|^2) at any gap.
        let mut config = mono_config(10.0, 50.0);
        let grid = GridConfig::new(128, 20.0).unwrap();
        let mask = slit_mask(800.0);
        let t_abs = config.absorber.transmission(10.0);
        let open = open_fraction(&mask, &grid);
        let expected = open.iter().map(|m| m + (1.0 - m) * t_abs).sum::<f64>() / open.len() as f64;
        for gap in [0.0, 50.0, 500.0] {
            config.proximity_gap_um = gap;
            let img = shadow_image(&config, &mask, &grid).unwrap();
            let mean = img.iter().sum::<f64>() / img.len() as f64;
            assert_relative_eq!(mean, expected, max_relative = 1e-9);
        }
    }

    #[test]
    fn test_absorber_amplitude_uses_henke_delta() {
        // 20 um Au (19.3 g/cm^3) at 8 keV: CXRO delta = 4.77303e-5 at
        // 19.32 g/cm^3 -> 4.76809e-5 at 19.3; phase = -2 pi delta t / lambda
        // = -38.664 rad; |a|^2 = exp(-mu t) with NIST mu/rho = 207.2.
        let f = BeamFilter::new(Compound::gold(19.3), 20.0);
        let a = f.amplitude_transmission(8.0);
        assert_relative_eq!(
            a.norm_sqr(),
            (-207.2 * 19.3 * 20e-4_f64).exp(),
            max_relative = 1e-9
        );
        assert_relative_eq!(a.norm_sqr(), f.transmission(8.0), max_relative = 1e-12);
        let delta = 4.77303e-5 * 19.3 / 19.32;
        let phase = -2.0 * std::f64::consts::PI * delta * 20e3 / wavelength_nm(8.0);
        let expected = Complex64::from_polar(a.norm(), phase);
        assert!((a - expected).norm() < 2e-3 * a.norm(), "{a} vs {expected}");
        // Above the Henke range f1 is held: delta scales as lambda^2.
        let a40 = f.amplitude_transmission(40.0).arg();
        let a30 = f.amplitude_transmission(30.0).arg();
        assert!(a40.is_finite() && a30.is_finite());
    }

    #[test]
    fn test_edge_blur_grows_with_depth_in_fresnel_volume() {
        // Depth-dependent diffraction: the 10-90% width of the edge grows as
        // sqrt(g + z).
        let mut config = mono_config(8.0, 400.0);
        config.absorber = BeamFilter::new(Compound::gold(19.3), 200.0);
        config.proximity_gap_um = 20.0;
        let depths = [0.0, 400.0];
        let s0 = (wavelength_nm(8.0) * 20e3).sqrt();
        let p =
            fresnel_edge_profile_1d(&config, -40.0 * s0, 40.0 * s0, s0 / 200.0, &depths).unwrap();
        let width = |k: usize| {
            let row = &p.dose_kj_cm3[k];
            let open = p.open_dose_kj_cm3[k];
            let lo = row.iter().position(|d| d / open >= 0.1).unwrap();
            let hi = row.iter().position(|d| d / open >= 0.9).unwrap();
            p.x_nm[hi] - p.x_nm[lo]
        };
        let ratio = width(1) / width(0);
        assert_relative_eq!(ratio, (420.0f64 / 20.0).sqrt(), max_relative = 0.02);
    }

    #[test]
    fn test_edge_profile_sidewall_metrics() {
        let config = DeepXrayConfig {
            energy_bins: 24,
            resist_thickness_um: 200.0,
            ..bm_config()
        };
        let depths = [0.0, 50.0, 100.0, 150.0, 200.0];
        let p = fresnel_edge_profile_1d(&config, -2000.0, 2000.0, 10.0, &depths).unwrap();
        // The open side at the bottom reaches the target dose.
        assert_relative_eq!(p.open_dose_kj_cm3[4], 3.0, max_relative = 1e-9);
        // Threshold 10% below the clearing dose: developed at every depth;
        // the edge moves toward the open side with depth (dose falls with
        // depth).
        let edges = p.edge_positions_nm(2.7);
        assert!(edges.iter().all(|e| e.is_some()), "{edges:?}");
        assert!(
            edges.windows(2).all(|w| w[1].unwrap() > w[0].unwrap()),
            "{edges:?}"
        );
        let angle = p.sidewall_angle_deg(2.7).unwrap();
        assert!(angle > 0.0 && angle < 5.0, "sidewall angle {angle} deg");
        let grads = p.max_gradient_kj_cm3_nm();
        assert!(grads[0] > grads[4], "edge steepness should fall with depth");
    }

    #[test]
    fn test_photoelectron_blur_smooths_but_preserves_open_field() {
        let mut config = mono_config(8.0, 50.0);
        config.photoelectron_blur = true;
        let grid = GridConfig::new(64, 100.0).unwrap();
        let img = shadow_image(&config, &clear_mask(), &grid).unwrap();
        for v in img.iter() {
            assert_relative_eq!(*v, 1.0, epsilon = 1e-9);
        }
        let mask = slit_mask(3200.0);
        let sharp = {
            let mut c = config.clone();
            c.photoelectron_blur = false;
            shadow_image(&c, &mask, &grid).unwrap()
        };
        let blurred = shadow_image(&config, &mask, &grid).unwrap();
        let grad = |img: &Array2<f64>| {
            (0..63)
                .map(|j| (img[[32, j + 1]] - img[[32, j]]).abs())
                .fold(0.0, f64::max)
        };
        assert!(grad(&blurred) < grad(&sharp));
    }

    #[test]
    fn test_fresnel_sampling_check() {
        let config = bm_config();
        let coarse = GridConfig::new(64, 2000.0).unwrap();
        let s = fresnel_sampling(&config, &coarse);
        assert!(!s.resolved && !s.warnings.is_empty());
        assert!(s.require_resolved().is_err());
        // Dose-weighted sqrt(lambda g) ~ 230 nm at the top for a 100 um gap
        // (the surface dose is dominated by 2-5 keV photons that the 2 um Ti
        // membrane passes below its K edge); the bottom scale is larger
        // (g + T = 600 um, hardened beam).
        assert!(
            (100.0..300.0).contains(&s.fresnel_scale_top_nm),
            "{}",
            s.fresnel_scale_top_nm
        );
        assert!(s.fresnel_scale_bottom_nm > s.fresnel_scale_top_nm, "{s:?}");
        let fine = GridConfig::new(64, 20.0).unwrap();
        let f = fresnel_sampling(&config, &fine);
        assert!(f.resolved && f.require_resolved().is_ok());
    }

    // ---- 2D shadow / volume ----------------------------------------------

    #[test]
    fn test_shadow_image_open_brighter_than_absorber() {
        for model in [ProximityModel::Fresnel, ProximityModel::Gaussian] {
            let config = DeepXrayConfig {
                proximity_gap_um: 10.0,
                energy_bins: 24,
                proximity_model: model,
                ..bm_config()
            };
            // Wide lines/spaces so feature centers survive the proximity blur.
            let mask = Mask::line_space(400.0, 800.0).unwrap();
            let grid = GridConfig::new(128, 20.0).unwrap();
            let img = shadow_image(&config, &mask, &grid).unwrap();

            let max = img.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
            let min = img.iter().cloned().fold(f64::INFINITY, f64::min);
            assert!(max > min, "open ({max}) should exceed absorber ({min})");
            // Au is partially transparent to hard photons: the floor is > 0.
            assert!(
                min > 0.0,
                "absorber floor should be > 0 (leakage), got {min}"
            );
            // Open areas approach 1 (Fresnel ringing may overshoot slightly).
            assert!(max > 0.5 && max < 1.5, "{model:?}: open value {max}");
        }
    }

    #[test]
    fn test_volumetric_open_column_hits_target() {
        for model in [ProximityModel::Fresnel, ProximityModel::Gaussian] {
            let config = DeepXrayConfig {
                proximity_gap_um: 5.0,
                resist_thickness_um: 100.0,
                energy_bins: 24,
                proximity_model: model,
                ..bm_config()
            };
            let grid = GridConfig::new(16, 50.0).unwrap();
            let vol = expose_volumetric(&config, &clear_mask(), &grid, 20).unwrap();
            // Bottom slice, center column: open-area dose hits the target.
            let bottom = vol.nz() - 1;
            let center = vol.data[[bottom, 8, 8]];
            assert_relative_eq!(center, 3.0, max_relative = 1e-6);
            // Dose decreases from top to bottom (attenuation).
            let top = vol.data[[0, 8, 8]];
            assert!(top > center, "top {top} should exceed bottom {center}");
        }
    }

    #[test]
    fn test_volumetric_fresnel_slices_match_shadow_image() {
        // Monochromatic: slice k of the Fresnel volume is the shadow image
        // propagated over gap + z_k, times the depth weight.
        let mut config = mono_config(8.0, 40.0);
        config.proximity_gap_um = 10.0;
        let grid = GridConfig::new(64, 20.0).unwrap();
        let mask = slit_mask(640.0);
        let vol = expose_volumetric(&config, &mask, &grid, 4).unwrap();
        for k in [0usize, 3] {
            let z = vol.z_at(k) * 1e-3;
            let mut c = config.clone();
            c.proximity_gap_um = 10.0 + z;
            let img = shadow_image(&c, &mask, &grid).unwrap();
            let open = vol.data[[k, 32, 32]] / img[[32, 32]];
            for j in 0..64 {
                assert_relative_eq!(
                    vol.data[[k, 32, j]],
                    open * img[[32, j]],
                    max_relative = 1e-9
                );
            }
        }
    }

    #[test]
    fn test_sidewall_gradient_detects_edge() {
        let config = DeepXrayConfig {
            proximity_gap_um: 5.0,
            resist_thickness_um: 100.0,
            energy_bins: 24,
            ..bm_config()
        };
        let mask = Mask::line_space(400.0, 800.0).unwrap();
        let grid = GridConfig::new(64, 20.0).unwrap();
        let vol = expose_volumetric(&config, &mask, &grid, 8).unwrap();
        // With a patterned mask there is a lateral dose step, so at least one
        // slice has a nonzero sidewall gradient at a mid-level threshold.
        let grads = sidewall_dose_gradient(&vol, 1.5);
        assert_eq!(grads.len(), vol.nz());
        assert!(
            grads.iter().any(|&g| g > 0.0),
            "expected a nonzero sidewall gradient somewhere"
        );
    }

    // ---- development / metrics -------------------------------------------

    #[test]
    fn test_develop_depth_full_and_zero() {
        let mut vol =
            Grid3D::<f64>::new(4, 4, 10, (-100.0, 100.0), (-100.0, 100.0), (0.0, 500_000.0))
                .unwrap();
        // Column (0,0): full dose everywhere; column (1,1): zero dose.
        for k in 0..vol.nz() {
            vol.data[[k, 0, 0]] = 5.0;
        }
        let depth = develop_depth(&vol, 1.0);
        // Full-dose column develops to the full thickness.
        assert_relative_eq!(depth[[0, 0]], 500_000.0, max_relative = 1e-12);
        // Zero-dose column does not develop at all.
        assert_relative_eq!(depth[[1, 1]], 0.0);
    }

    #[test]
    fn test_max_aspect_ratio_sanity() {
        // 500 um resist, 5 um minimum feature => aspect 100.
        let exposure = LigaExposure {
            z_um: vec![0.0, 250.0, 500.0],
            dose_kj_cm3: vec![0.0; 3],
            scale: 1.0,
            top_dose_kj_cm3: 0.0,
            bottom_dose_kj_cm3: 0.0,
            dose_ratio: 1.0,
            exceeds_damage_ceiling: false,
            exposure_time_estimate: None,
            top_dose_rate_kj_cm3_s: None,
            bottom_dose_rate_kj_cm3_s: None,
            resist_power_density_w_mm2: None,
            exposure_charge_ma_h: None,
            mean_energy_top_kev: 0.0,
            mean_energy_bottom_kev: 0.0,
            window_power_fraction: 1.0,
            warnings: Vec::new(),
        };
        assert_relative_eq!(
            max_aspect_ratio(&exposure, 5000.0),
            100.0,
            max_relative = 1e-12
        );
    }

    #[test]
    fn test_grun_range_fixture() {
        // 0.046 * 8^1.75 / 1.19 ~ 1.47 um for PMMA at 8 keV (formula check).
        let r_g_um = grun_range_nm(8.0, 1.19) / 1e3;
        assert_relative_eq!(
            r_g_um,
            0.046 * 8.0_f64.powf(1.75) / 1.19,
            max_relative = 1e-12
        );
        assert!((1.46..=1.50).contains(&r_g_um), "Gruen range {r_g_um} um");
    }

    #[test]
    fn test_pmma_development_rate_power_law() {
        // R = R0 (D/D0)^q; at D = D0 the rate is R0.
        assert_relative_eq!(
            pmma_development_rate(5.0, 2.0, 5.0, 3.0),
            2.0,
            max_relative = 1e-12
        );
        // Doubling dose with q=3 gives 8x rate.
        assert_relative_eq!(
            pmma_development_rate(10.0, 2.0, 5.0, 3.0),
            2.0 * 8.0,
            max_relative = 1e-12
        );
        // Non-positive dose develops nothing.
        assert_eq!(pmma_development_rate(0.0, 2.0, 5.0, 3.0), 0.0);
    }
}
