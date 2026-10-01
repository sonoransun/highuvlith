//! Synchrotron radiation sources: bending magnets and undulators.
//!
//! Storage-ring synchrotron radiation spans the VUV-to-hard-X-ray range
//! with high average power and excellent stability. Bending magnets emit
//! a smooth broadband spectrum characterized entirely by the critical
//! energy `E_c = 0.665 E^2[GeV^2] B[T]`; undulators emit quasi-
//! monochromatic harmonics at `lambda_n = lambda_u (1 + K^2/2) / (2 n gamma^2)`.
//!
//! The **wavelength is DERIVED from machine parameters** here (electron
//! energy, field, undulator period/strength) — not stored as a free
//! number. So are the absolute flux and power (from the stored ring
//! current) and, when ring emittances are given, the transverse coherent
//! fraction.
//!
//! Bending magnets are the canonical exposure source for LIGA deep X-ray
//! lithography. The deep-X-ray module couples via
//! `deep_xray::XraySpectrum::from_synchrotron`, which reads
//! [`SynchrotronSource::critical_energy_kev`] and then samples
//! `physics::bm_universal_flux` directly;
//! [`SynchrotronSource::bm_spectral_flux`] is a convenience wrapper over
//! the same universal function for external callers, and
//! [`SynchrotronSource::bm_flux_per_mrad`] gives the absolute flux.
//!
//! # Model status
//!
//! Implemented, textbook-exact and fixture-tested: undulator resonance, the
//! on-axis sinc² natural line (FWHM 0.886/(nN)), harmonic factors
//! `F_n(K)` / `Q_n(K)` (Bessel series), Kim's central-cone flux and power
//! (`average_power_w()` = central-cone power of the selected harmonic),
//! total undulator power, bending-magnet critical energy, universal flux
//! function S(y) (Kostroun algorithm), absolute BM flux and power per mrad.
//! Simplified (🔶): the coherent fraction is the matched-beta emittance
//! convolution `(lambda/4 pi)/(eps + lambda/4 pi)` per plane (an upper
//! bound), and it maps to the pupil through the Gaussian–Schell heuristic
//! `sigma_from_coherence`; the flux formulas are filament-beam values
//! (emittance and energy-spread broadening are reported, not folded in).
//!
//! # Key equations
//!
//! - `lambda_n = lambda_u (1 + K^2/2) / (2 n gamma^2)`; line `sinc^2(pi n N d-lambda/lambda)`
//! - `F_n(K) = n^2 K^2 JJ_n^2 / (1 + K^2/2)^2`, `Q_n = (1 + K^2/2) F_n / n`
//! - central-cone flux `pi alpha N Q_n (dw/w) I/e`; power `pi alpha Q_n (I/e) E_ph,1`
//! - coherent fraction `zeta_x zeta_y`, `zeta = (lambda/4 pi) / (eps + lambda/4 pi)`
//! - BM: `dF/dtheta = (sqrt3/2 pi) alpha gamma (dw/w)(I/e) G1(E/E_c)`,
//!   `dP/dtheta = e gamma^4 I / (6 pi eps0 rho)`

use serde::{Deserialize, Serialize};

use super::physics;
use crate::source::{
    evaluate_illumination, evaluate_spectral_weights, sigma_from_coherence, DerivedQuantity,
    IlluminationShape, LithographySource, SpectralShape, SINC2_FWHM_X,
};

/// Which beamline of the storage ring feeds the exposure tool.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SynchrotronBeamline {
    /// Bending-magnet beamline: smooth broadband spectrum with critical
    /// energy E_c; a monochromator selects the working wavelength for
    /// projection imaging (LIGA uses the full white beam instead).
    BendingMagnet {
        /// Ring electron energy in GeV.
        electron_energy_gev: f64,
        /// Bending field in tesla.
        field_t: f64,
        /// Monochromator-selected wavelength in nm (for projection use).
        selected_wavelength_nm: f64,
        /// Monochromator bandwidth FWHM in pm.
        mono_bandwidth_pm: f64,
    },
    /// Undulator beamline: quasi-monochromatic odd harmonics; the
    /// wavelength is derived from the resonance condition.
    Undulator {
        /// Ring electron energy in GeV.
        electron_energy_gev: f64,
        /// Undulator period in mm.
        period_mm: f64,
        /// Dimensionless undulator strength K.
        k: f64,
        /// Number of undulator periods (sets the natural bandwidth 1/(nN)).
        num_periods: usize,
        /// Odd harmonic number (1, 3, 5, ...).
        harmonic: usize,
    },
}

/// Stored-beam parameters of the ring (used for the emittance-derived
/// coherent fraction and the energy-spread broadening check).
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct StorageRingBeam {
    /// Horizontal geometric emittance in nm·rad.
    pub emittance_x_nm_rad: f64,
    /// Vertical geometric emittance in nm·rad.
    pub emittance_y_nm_rad: f64,
    /// Relative rms energy spread.
    pub energy_spread_rel: f64,
}

impl StorageRingBeam {
    /// ILLUSTRATIVE compact (~0.5 GeV) ring: 10 nm·rad horizontal, 1 %
    /// coupling (0.1 nm·rad vertical), 5e-4 energy spread — assumed values
    /// of the right order for compact low-energy rings, not a specific
    /// machine.
    pub fn compact_ring() -> Self {
        Self {
            emittance_x_nm_rad: 10.0,
            emittance_y_nm_rad: 0.1,
            energy_spread_rel: 5e-4,
        }
    }

    /// Coherent fraction per plane `(x, y)` at `wavelength_nm`.
    pub fn coherent_fractions(&self, wavelength_nm: f64) -> (f64, f64) {
        (
            physics::coherent_fraction_per_plane(self.emittance_x_nm_rad * 1e-9, wavelength_nm),
            physics::coherent_fraction_per_plane(self.emittance_y_nm_rad * 1e-9, wavelength_nm),
        )
    }
}

/// Storage-ring synchrotron source.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SynchrotronSource {
    /// Beamline type and machine parameters.
    pub beamline: SynchrotronBeamline,
    /// Stored ring current in mA (sets the absolute flux / power in
    /// `average_power_w()` and the derived quantities).
    pub ring_current_ma: f64,
    /// Number of spectral samples for polychromatic simulation.
    pub spectral_samples: usize,
    /// Illumination pupil shape. Bending magnets are incoherent
    /// (large sigma disk); undulators are partially coherent.
    pub illumination: IlluminationShape,
    /// Stored transverse coherence fraction in [0, 1] — the fallback used
    /// when `ring_beam` is `None` (undulator constructor default 0.2;
    /// bending magnet 0).
    pub transverse_coherence_fraction: f64,
    /// Optional stored-beam emittances / energy spread. When present, an
    /// undulator's coherent fraction is DERIVED from them (replacing the
    /// stored constant). `None` in configs written before the field existed.
    #[serde(default)]
    pub ring_beam: Option<StorageRingBeam>,
}

impl SynchrotronSource {
    /// LIGA-class bending magnet: 2.5 GeV ring, 1.5 T field
    /// (E_c = 6.23 keV, lambda_c = 0.199 nm) — KIT/ANKA-class parameters.
    /// The monochromator selection is only used if this source is fed to
    /// the projection pipeline; LIGA shadow printing consumes the full
    /// white-beam spectrum via [`Self::bm_spectral_flux`].
    pub fn liga_bending_magnet() -> Self {
        Self {
            beamline: SynchrotronBeamline::BendingMagnet {
                electron_energy_gev: 2.5,
                field_t: 1.5,
                selected_wavelength_nm: 0.2,
                mono_bandwidth_pm: 0.2,
            },
            ring_current_ma: 200.0,
            spectral_samples: 5,
            illumination: IlluminationShape::Conventional { sigma: 0.8 },
            transverse_coherence_fraction: 0.0,
            ring_beam: None,
        }
    }

    /// Compact EUV undulator: 538 MeV ring, 20 mm period, K = 1,
    /// 100 periods, first harmonic -> 13.5 nm (sinc² line, 0.886 %
    /// FWHM), on the illustrative compact-ring beam
    /// ([`StorageRingBeam::compact_ring`]) so the coherent fraction is
    /// derived (≈ 0.089).
    pub fn compact_euv_undulator() -> crate::error::Result<Self> {
        Ok(Self::undulator(0.538, 20.0, 1.0, 100, 1)?
            .with_ring_beam(StorageRingBeam::compact_ring()))
    }

    /// Construct an undulator beamline; the emitted wavelength is derived
    /// from the resonance condition. Rejects even harmonics (on-axis
    /// undulator emission contains odd harmonics only). Without ring
    /// emittances the coherence is the stored legacy constant 0.2; attach
    /// a beam with [`Self::with_ring_beam`] to derive it.
    pub fn undulator(
        electron_energy_gev: f64,
        period_mm: f64,
        k: f64,
        num_periods: usize,
        harmonic: usize,
    ) -> crate::error::Result<Self> {
        if harmonic == 0 || harmonic.is_multiple_of(2) {
            return Err(crate::error::LithographyError::InvalidParameter {
                name: "harmonic",
                value: harmonic as f64,
                reason: "on-axis undulator harmonics must be odd (1, 3, 5, ...)",
            });
        }
        for (name, value) in [
            ("electron_energy_gev", electron_energy_gev),
            ("period_mm", period_mm),
            ("k", k),
        ] {
            if value <= 0.0 || value.is_nan() {
                return Err(crate::error::LithographyError::InvalidParameter {
                    name,
                    value,
                    reason: "must be positive",
                });
            }
        }
        if num_periods == 0 {
            return Err(crate::error::LithographyError::InvalidParameter {
                name: "num_periods",
                value: 0.0,
                reason: "must be positive",
            });
        }
        let coherence = 0.2;
        Ok(Self {
            beamline: SynchrotronBeamline::Undulator {
                electron_energy_gev,
                period_mm,
                k,
                num_periods,
                harmonic,
            },
            ring_current_ma: 200.0,
            spectral_samples: 5,
            illumination: IlluminationShape::CoherentGaussian {
                sigma: sigma_from_coherence(coherence, 0.05),
            },
            transverse_coherence_fraction: coherence,
            ring_beam: None,
        })
    }

    /// Attach stored-beam parameters. For an undulator the coherent
    /// fraction is re-derived from the emittances at the emitted
    /// wavelength and the `CoherentGaussian` pupil is rebuilt from it; the
    /// stored `transverse_coherence_fraction` is updated to the same value.
    /// Bending magnets keep their incoherent pupil.
    pub fn with_ring_beam(mut self, beam: StorageRingBeam) -> Self {
        self.ring_beam = Some(beam);
        if matches!(self.beamline, SynchrotronBeamline::Undulator { .. }) {
            let (zx, zy) = beam.coherent_fractions(self.wavelength_nm());
            let coherence = zx * zy;
            self.transverse_coherence_fraction = coherence;
            self.illumination = IlluminationShape::CoherentGaussian {
                sigma: sigma_from_coherence(coherence, 0.05),
            };
        }
        self
    }

    /// Electron Lorentz factor of the ring.
    pub fn gamma(&self) -> f64 {
        let e_gev = match &self.beamline {
            SynchrotronBeamline::BendingMagnet {
                electron_energy_gev,
                ..
            }
            | SynchrotronBeamline::Undulator {
                electron_energy_gev,
                ..
            } => *electron_energy_gev,
        };
        physics::gamma_from_mev(e_gev * 1000.0)
    }

    /// Bending-magnet critical energy in keV, if this is a bending-magnet
    /// beamline (`None` for undulators). Feeds the LIGA depth-dose module.
    pub fn critical_energy_kev(&self) -> Option<f64> {
        match &self.beamline {
            SynchrotronBeamline::BendingMagnet {
                electron_energy_gev,
                field_t,
                ..
            } => Some(physics::critical_energy_kev(*electron_energy_gev, *field_t)),
            SynchrotronBeamline::Undulator { .. } => None,
        }
    }

    /// Universal bending-magnet flux function S(y) at y = E / E_c
    /// (photon flux per unit relative bandwidth, arbitrary units).
    /// Returns 0 for undulator beamlines. Convenience wrapper over
    /// `physics::bm_universal_flux`; the LIGA depth-dose module couples
    /// through `deep_xray::XraySpectrum::from_synchrotron` (which reads
    /// `critical_energy_kev()`) rather than calling this method.
    pub fn bm_spectral_flux(&self, photon_energy_kev: f64) -> f64 {
        match self.critical_energy_kev() {
            Some(e_c) if e_c > 0.0 => physics::bm_universal_flux(photon_energy_kev / e_c),
            _ => 0.0,
        }
    }

    /// Absolute bending-magnet photon flux at `photon_energy_kev`, in
    /// photons/s/mrad/0.1%BW (per mrad of horizontal angle, integrated over
    /// the vertical), from the stored ring current. 0 for undulators.
    pub fn bm_flux_per_mrad(&self, photon_energy_kev: f64) -> f64 {
        match self.critical_energy_kev() {
            Some(e_c) if e_c > 0.0 => physics::bm_flux_per_mrad(
                self.gamma(),
                self.ring_current_ma * 1e-3,
                photon_energy_kev / e_c,
            ),
            _ => 0.0,
        }
    }

    /// Central-cone power (W) of the undulator's selected harmonic within
    /// its natural line; `None` for bending magnets.
    pub fn undulator_line_power_w(&self) -> Option<f64> {
        match &self.beamline {
            SynchrotronBeamline::Undulator {
                period_mm,
                k,
                harmonic,
                ..
            } => {
                let lambda1 = physics::undulator_resonance_nm(*period_mm, *k, self.gamma(), 1);
                Some(physics::undulator_central_cone_power_w(
                    *k,
                    *harmonic,
                    self.ring_current_ma * 1e-3,
                    lambda1,
                ))
            }
            SynchrotronBeamline::BendingMagnet { .. } => None,
        }
    }

    fn undulator_derived(&self, out: &mut Vec<DerivedQuantity>) {
        let SynchrotronBeamline::Undulator {
            period_mm,
            k,
            num_periods,
            harmonic,
            ..
        } = &self.beamline
        else {
            return;
        };
        let (period_mm, k, n_per, n) = (*period_mm, *k, *num_periods, *harmonic);
        let gamma = self.gamma();
        let current_a = self.ring_current_ma * 1e-3;
        let lambda = self.wavelength_nm();
        let b0 = physics::undulator_field_from_k(k, period_mm);
        let length_m = n_per as f64 * period_mm * 1e-3;
        let p_line = self.undulator_line_power_w().unwrap_or(0.0);
        let p_total = physics::undulator_total_power_w(gamma, b0, length_m, current_a);
        let q_n = physics::undulator_qn(n, k);
        let q_1 = physics::undulator_qn(1, k);
        out.extend([
            DerivedQuantity::new("electron_gamma", gamma, "-", "1 + E_kin / m_e c^2"),
            DerivedQuantity::new(
                "undulator_field",
                b0,
                "T",
                "B0 = 2 pi m_e c K / (e lambda_u)",
            ),
            DerivedQuantity::new(
                "natural_bandwidth_first_zero",
                1.0 / (n as f64 * n_per as f64),
                "-",
                "d-lambda/lambda to the first zero of the sinc^2 line, 1/(nN)",
            ),
            DerivedQuantity::new(
                "harmonic_factor_fn",
                physics::undulator_fn(n, k),
                "-",
                "F_n(K) = n^2 K^2 JJ^2 / (1 + K^2/2)^2 (on-axis brightness factor)",
            ),
            DerivedQuantity::new(
                "harmonic_factor_qn",
                q_n,
                "-",
                "Q_n(K) = (1 + K^2/2) F_n / n (central-cone flux factor)",
            ),
            DerivedQuantity::new(
                "harmonic3_over_1",
                physics::undulator_qn(3, k) / q_1,
                "-",
                "central-cone flux of harmonic 3 relative to 1 (per 0.1% BW)",
            ),
            DerivedQuantity::new(
                "harmonic5_over_1",
                physics::undulator_qn(5, k) / q_1,
                "-",
                "central-cone flux of harmonic 5 relative to 1 (per 0.1% BW)",
            ),
            DerivedQuantity::new(
                "central_cone_flux",
                physics::undulator_central_cone_flux(n_per, k, n, current_a),
                "photons/s/0.1%BW",
                "pi alpha N Q_n I/e (Kim), filament beam",
            ),
            DerivedQuantity::new(
                "on_axis_flux_density",
                physics::undulator_on_axis_flux_density(n_per, k, n, gamma, current_a),
                "photons/s/mrad^2/0.1%BW",
                "alpha N^2 gamma^2 F_n I/e, filament beam",
            ),
            DerivedQuantity::new(
                "central_cone_power",
                p_line,
                "W",
                "pi alpha Q_n (I/e) E_ph,1 = average_power_w (usable line power)",
            ),
            DerivedQuantity::new(
                "photon_rate",
                physics::watts_to_photon_rate(p_line, lambda),
                "photons/s",
                "central-cone photons per second in the natural line",
            ),
            DerivedQuantity::new(
                "total_undulator_power",
                p_total,
                "W",
                "all harmonics, all angles: e^3 gamma^2 B0^2 L I / (12 pi eps0 m^2 c^2)",
            ),
            DerivedQuantity::new(
                "line_power_fraction",
                if p_total > 0.0 { p_line / p_total } else { 0.0 },
                "-",
                "central-cone line power / total undulator power",
            ),
            DerivedQuantity::new(
                "hvm_power_ratio",
                p_line / super::throughput::HVM_EUV_POWER_AT_IF_W,
                "-",
                "central-cone power / 250 W (production 13.5 nm HVM at IF)",
            ),
        ]);
        if let Some(beam) = self.ring_beam {
            let (zx, zy) = beam.coherent_fractions(lambda);
            let natural = 1.0 / (n as f64 * n_per as f64);
            out.extend([
                DerivedQuantity::new(
                    "coherent_fraction_x",
                    zx,
                    "-",
                    "(lambda/4pi) / (eps_x + lambda/4pi), matched beta (upper bound)",
                ),
                DerivedQuantity::new(
                    "coherent_fraction_y",
                    zy,
                    "-",
                    "(lambda/4pi) / (eps_y + lambda/4pi), matched beta (upper bound)",
                ),
                DerivedQuantity::new(
                    "coherent_fraction",
                    zx * zy,
                    "-",
                    "zeta_x zeta_y = transverse_coherence() (replaces the stored constant)",
                ),
                DerivedQuantity::new(
                    "coherent_power",
                    p_line * zx * zy,
                    "W",
                    "central-cone power x coherent fraction",
                ),
                DerivedQuantity::new(
                    "energy_spread_broadening_ratio",
                    2.0 * n as f64 * beam.energy_spread_rel / natural,
                    "-",
                    "2 n sigma_E / (1/(nN)); << 1 keeps the natural sinc^2 line",
                ),
            ]);
        }
    }

    fn bending_magnet_derived(&self, out: &mut Vec<DerivedQuantity>) {
        let SynchrotronBeamline::BendingMagnet {
            field_t,
            selected_wavelength_nm,
            mono_bandwidth_pm,
            ..
        } = &self.beamline
        else {
            return;
        };
        let gamma = self.gamma();
        let current_a = self.ring_current_ma * 1e-3;
        let e_c = self.critical_energy_kev().unwrap_or(0.0);
        let rho = physics::bending_radius_m(gamma, *field_t);
        let e_sel_kev = physics::HC_EV_NM / selected_wavelength_nm * 1e-3;
        let flux = self.bm_flux_per_mrad(e_sel_kev);
        let mono_rel = mono_bandwidth_pm * 1e-3 / selected_wavelength_nm;
        let mono_flux = flux * mono_rel / 1e-3;
        out.extend([
            DerivedQuantity::new("critical_energy", e_c, "keV", "0.665 E^2[GeV] B[T]"),
            DerivedQuantity::new(
                "critical_wavelength",
                physics::HC_EV_NM / (e_c * 1e3),
                "nm",
                "hc / E_c",
            ),
            DerivedQuantity::new("bending_radius", rho, "m", "p / (e B)"),
            DerivedQuantity::new(
                "power_per_mrad",
                physics::bm_power_per_mrad_w(gamma, rho, current_a),
                "W/mrad",
                "e gamma^4 I / (6 pi eps0 rho), all photon energies",
            ),
            DerivedQuantity::new(
                "flux_per_mrad_at_selection",
                flux,
                "photons/s/mrad/0.1%BW",
                "(sqrt3/2pi) alpha gamma (dw/w)(I/e) G1(E/E_c) at the mono wavelength",
            ),
            DerivedQuantity::new(
                "mono_photon_rate_per_mrad",
                mono_flux,
                "photons/s/mrad",
                "flux x (mono bandwidth / 0.1%)",
            ),
            DerivedQuantity::new(
                "mono_power_per_mrad",
                physics::photon_rate_to_watts(mono_flux, *selected_wavelength_nm),
                "W/mrad",
                "monochromatized power per mrad of horizontal acceptance",
            ),
        ]);
    }
}

impl Default for SynchrotronSource {
    fn default() -> Self {
        Self::compact_euv_undulator().expect("compact undulator preset is valid")
    }
}

impl LithographySource for SynchrotronSource {
    /// DERIVED from machine parameters for undulators (resonance
    /// condition); monochromator-selected for bending magnets.
    fn wavelength_nm(&self) -> f64 {
        match &self.beamline {
            SynchrotronBeamline::BendingMagnet {
                selected_wavelength_nm,
                ..
            } => *selected_wavelength_nm,
            SynchrotronBeamline::Undulator {
                period_mm,
                k,
                harmonic,
                ..
            } => physics::undulator_resonance_nm(*period_mm, *k, self.gamma(), *harmonic),
        }
    }

    /// Monochromator bandwidth (bending magnet) or the FWHM of the natural
    /// sinc² undulator line, `0.886 lambda / (nN)`.
    fn bandwidth_pm(&self) -> f64 {
        match &self.beamline {
            SynchrotronBeamline::BendingMagnet {
                mono_bandwidth_pm, ..
            } => *mono_bandwidth_pm,
            SynchrotronBeamline::Undulator {
                num_periods,
                harmonic,
                ..
            } => {
                let rel = SINC2_FWHM_X / (*harmonic as f64 * *num_periods as f64);
                rel * self.wavelength_nm() * 1e3
            }
        }
    }

    fn intensity_at(&self, fx_norm: f64, fy_norm: f64) -> f64 {
        evaluate_illumination(&self.illumination, fx_norm, fy_norm)
    }

    fn spectral_weights(&self) -> Vec<(f64, f64)> {
        let shape = match &self.beamline {
            // True on-axis undulator line: sinc^2 with first zeros at 1/(nN).
            SynchrotronBeamline::Undulator { .. } => SpectralShape::SincSquared,
            // Monochromator passband.
            SynchrotronBeamline::BendingMagnet { .. } => SpectralShape::Gaussian,
        };
        evaluate_spectral_weights(
            self.wavelength_nm(),
            self.bandwidth_pm(),
            self.spectral_samples,
            &shape,
        )
    }

    /// Emittance-derived `zeta_x zeta_y` for an undulator with `ring_beam`;
    /// otherwise the stored fraction.
    fn transverse_coherence(&self) -> f64 {
        match (&self.beamline, &self.ring_beam) {
            (SynchrotronBeamline::Undulator { .. }, Some(beam)) => {
                let (zx, zy) = beam.coherent_fractions(self.wavelength_nm());
                zx * zy
            }
            _ => self.transverse_coherence_fraction,
        }
    }

    /// Undulator: central-cone power of the selected harmonic in its
    /// natural line (the usable, quasi-monochromatic output). Bending
    /// magnet: `None` (the usable power depends on the beamline's
    /// horizontal acceptance; see the per-mrad derived quantities).
    fn average_power_w(&self) -> Option<f64> {
        self.undulator_line_power_w()
    }

    // Storage rings are quasi-CW at MHz bunch rates with sub-percent
    // stability: pulse metadata stays None and shot_to_shot_rms 0.

    fn derived_quantities(&self) -> Vec<DerivedQuantity> {
        let mut out = vec![DerivedQuantity::new(
            "photon_energy",
            physics::HC_EV_NM / self.wavelength_nm(),
            "eV",
            "hc / lambda",
        )];
        self.undulator_derived(&mut out);
        self.bending_magnet_derived(&mut out);
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use approx::assert_relative_eq;

    fn dq(src: &SynchrotronSource, name: &str) -> f64 {
        src.derived_quantities()
            .into_iter()
            .find(|q| q.name == name)
            .unwrap_or_else(|| panic!("missing derived quantity {name}"))
            .value
    }

    #[test]
    fn test_undulator_wavelength_is_derived() {
        let src = SynchrotronSource::compact_euv_undulator().unwrap();
        let lambda = src.wavelength_nm();
        assert!(
            (13.3..13.7).contains(&lambda),
            "538 MeV / 20 mm / K=1 should give ~13.5 nm, got {lambda}"
        );

        // Doubling ring energy quarters the wavelength.
        let hot = SynchrotronSource::undulator(1.076, 20.0, 1.0, 100, 1).unwrap();
        assert_relative_eq!(lambda / hot.wavelength_nm(), 4.0, epsilon = 0.02);
    }

    #[test]
    fn test_undulator_natural_bandwidth() {
        let src = SynchrotronSource::compact_euv_undulator().unwrap();
        // sinc^2 FWHM = 0.8859 / (nN) = 0.886 % of 13.5065 nm = 119.65 pm
        let rel = src.bandwidth_pm() / (src.wavelength_nm() * 1e3);
        assert_relative_eq!(rel, 0.008_858_929_413_789_048, epsilon = 1e-12);
        assert_relative_eq!(
            src.bandwidth_pm(),
            119.652_936_021_982_64,
            max_relative = 1e-9
        );
        // First zero of the line is at 1/(nN).
        assert_relative_eq!(
            dq(&src, "natural_bandwidth_first_zero"),
            0.01,
            epsilon = 1e-15
        );
    }

    #[test]
    fn test_undulator_line_is_sinc_squared() {
        let mut src = SynchrotronSource::compact_euv_undulator().unwrap();
        src.spectral_samples = 101;
        let w = src.spectral_weights();
        let lambda = src.wavelength_nm();
        let first_zero = lambda / 100.0; // lambda / (nN)
        let peak = w.iter().map(|(_, v)| *v).fold(0.0, f64::max);
        // Samples within 0.1 % of the first zero are ~0 vs the peak.
        for (wl, v) in &w {
            let d = (wl - lambda).abs();
            if (d - first_zero).abs() < 1e-3 * first_zero {
                assert!(*v < 1e-5 * peak, "sinc^2 not ~0 at first zero: {v}");
            }
        }
        // Half maximum at +-FWHM/2 (interpolated between samples).
        let half = src.bandwidth_pm() * 1e-3 / 2.0;
        let near = w
            .iter()
            .min_by(|a, b| {
                ((a.0 - lambda).abs() - half)
                    .abs()
                    .total_cmp(&((b.0 - lambda).abs() - half).abs())
            })
            .unwrap();
        assert_relative_eq!(near.1 / peak, 0.5, epsilon = 0.05);
        let sum: f64 = w.iter().map(|(_, v)| v).sum();
        assert_relative_eq!(sum, 1.0, epsilon = 1e-12);
    }

    #[test]
    fn test_even_harmonic_rejected() {
        assert!(SynchrotronSource::undulator(0.5, 20.0, 1.0, 100, 2).is_err());
        assert!(SynchrotronSource::undulator(0.5, 20.0, 1.0, 100, 0).is_err());
        assert!(SynchrotronSource::undulator(0.5, 20.0, 1.0, 100, 3).is_ok());
    }

    #[test]
    fn test_liga_bending_magnet_critical_energy() {
        let src = SynchrotronSource::liga_bending_magnet();
        // 0.665 * 2.5^2 * 1.5 = 6.234 keV
        assert_relative_eq!(src.critical_energy_kev().unwrap(), 6.234, epsilon = 1e-3);

        // Flux function peaks below E_c and vanishes far above it.
        let e_c = src.critical_energy_kev().unwrap();
        assert!(src.bm_spectral_flux(0.3 * e_c) > src.bm_spectral_flux(5.0 * e_c));
        assert!(src.bm_spectral_flux(50.0 * e_c) < 1e-6);
    }

    #[test]
    fn test_bending_magnet_absolute_flux_and_power() {
        let src = SynchrotronSource::liga_bending_magnet();
        // 200 mA, 2.5 GeV, 1.5 T: 19.80 W/mrad; at 0.2 nm (y = 0.9944)
        // 8.036e12 photons/s/mrad/0.1%BW (scipy fixture).
        assert_relative_eq!(
            dq(&src, "power_per_mrad"),
            19.797_428_131_162_015,
            max_relative = 1e-6
        );
        assert_relative_eq!(
            dq(&src, "flux_per_mrad_at_selection"),
            8.035_517_717_520_618e12,
            max_relative = 1e-6
        );
        // The 0.2 pm mono at 0.2 nm is exactly a 0.1 % band.
        assert_relative_eq!(
            dq(&src, "mono_photon_rate_per_mrad"),
            8.035_517_717_520_618e12,
            max_relative = 1e-6
        );
        // No usable-power figure without a horizontal acceptance.
        assert!(src.average_power_w().is_none());
        // Flux linear in current.
        let half = SynchrotronSource {
            ring_current_ma: 100.0,
            ..src.clone()
        };
        assert_relative_eq!(
            half.bm_flux_per_mrad(6.0) / src.bm_flux_per_mrad(6.0),
            0.5,
            max_relative = 1e-12
        );
    }

    #[test]
    fn test_undulator_absolute_power() {
        let src = SynchrotronSource::compact_euv_undulator().unwrap();
        // Attwood/Kim central-cone power at 200 mA: 0.2324 W;
        // all-harmonic all-angle total 21.04 W.
        assert_relative_eq!(
            src.average_power_w().unwrap(),
            0.232_371_665_180_677_5,
            max_relative = 1e-6
        );
        assert_relative_eq!(
            dq(&src, "total_undulator_power"),
            21.044_539_286_759_285,
            max_relative = 1e-7
        );
        assert_relative_eq!(
            dq(&src, "central_cone_flux"),
            1.579_968_964_696_903_5e15,
            max_relative = 1e-8
        );
        // K = 1 harmonic content: Q3/Q1 = 0.1623
        assert_relative_eq!(
            dq(&src, "harmonic3_over_1"),
            0.162_297_572_364_459,
            max_relative = 1e-9
        );
        // Three orders of magnitude short of a 250 W HVM source.
        assert!(dq(&src, "hvm_power_ratio") < 1e-3);
    }

    #[test]
    fn test_emittance_derived_coherence() {
        let src = SynchrotronSource::compact_euv_undulator().unwrap();
        // 10 / 0.1 nm rad at 13.5065 nm: 0.09705 x 0.91488 = 0.08879
        assert_relative_eq!(
            src.transverse_coherence(),
            0.088_789_176_375_747_46,
            max_relative = 1e-9
        );
        assert_relative_eq!(
            src.transverse_coherence_fraction,
            0.088_789_176_375_747_46,
            max_relative = 1e-9
        );
        match src.illumination {
            IlluminationShape::CoherentGaussian { sigma } => {
                assert_relative_eq!(sigma, 0.167_799_240_443_566_7, max_relative = 1e-9)
            }
            _ => panic!("expected CoherentGaussian"),
        }
        // Without a ring beam the stored legacy constant is used.
        let legacy = SynchrotronSource::undulator(0.538, 20.0, 1.0, 100, 1).unwrap();
        assert_relative_eq!(legacy.transverse_coherence(), 0.2);
        // Lower emittance -> more coherent.
        let low = SynchrotronSource::undulator(0.538, 20.0, 1.0, 100, 1)
            .unwrap()
            .with_ring_beam(StorageRingBeam {
                emittance_x_nm_rad: 1.0,
                emittance_y_nm_rad: 0.01,
                energy_spread_rel: 5e-4,
            });
        assert!(low.transverse_coherence() > src.transverse_coherence());
        // Energy-spread broadening: 2 sigma_E n N = 2 * 5e-4 * 100 = 0.1
        assert_relative_eq!(
            dq(&src, "energy_spread_broadening_ratio"),
            0.1,
            epsilon = 1e-12
        );
    }

    #[test]
    fn test_undulator_has_no_bm_spectrum() {
        let src = SynchrotronSource::compact_euv_undulator().unwrap();
        assert_eq!(src.critical_energy_kev(), None);
        assert_eq!(src.bm_spectral_flux(1.0), 0.0);
        assert_eq!(src.bm_flux_per_mrad(1.0), 0.0);
    }

    #[test]
    fn test_spectral_weights_sum_to_one() {
        for src in [
            SynchrotronSource::liga_bending_magnet(),
            SynchrotronSource::compact_euv_undulator().unwrap(),
        ] {
            let sum: f64 = src.spectral_weights().iter().map(|(_, w)| w).sum();
            assert_relative_eq!(sum, 1.0, epsilon = 1e-12);
        }
    }

    #[test]
    fn test_undulator_coherent_gaussian_pupil() {
        let src = SynchrotronSource::compact_euv_undulator().unwrap();
        // Graded pupil: center brighter than mid-radius.
        assert!(src.intensity_at(0.0, 0.0) > src.intensity_at(0.1, 0.0));
    }

    #[test]
    fn test_legacy_toml_without_ring_beam_parses() {
        let src = SynchrotronSource::undulator(0.538, 20.0, 1.0, 100, 1).unwrap();
        let mut toml_str = toml::to_string(&src).unwrap();
        assert!(!toml_str.contains("ring_beam"));
        toml_str.push('\n');
        let parsed: SynchrotronSource = toml::from_str(&toml_str).unwrap();
        assert!(parsed.ring_beam.is_none());
        assert_relative_eq!(parsed.transverse_coherence(), 0.2);
    }
}
