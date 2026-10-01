"""Type stubs for the highuvlith._native extension module.

Kept in lock-step with the PyO3 bindings by ``tests/python/test_stub_drift.py``
(names, member kinds, parameter names/order/defaults). Units: lengths in nm
unless a name says otherwise (``_um``, ``_mm``, ``_m``), doses in mJ/cm²,
energies in eV or keV as named, powers in W.
"""

from typing import Any, Optional

import numpy as np

#: Illumination (pupil-fill) tuple: ("conventional", sigma) |
#: ("annular", sigma_inner, sigma_outer) | ("dipole", sigma_center,
#: sigma_radius, orientation_deg) | ("quadrupole", sigma_center, sigma_radius,
#: opening_angle_deg) | ("coherent_gaussian", sigma).
IlluminationSpec = tuple[Any, ...]

class SourceConfig:
    """Illumination source configuration (VUV excimer, LPA-FEL, ...)."""

    def __init__(
        self,
        wavelength_nm: float = 157.63,
        sigma_outer: float = 0.7,
        bandwidth_pm: float = 1.1,
        spectral_samples: int = 5,
    ) -> None: ...
    @staticmethod
    def f2_laser(sigma: float = 0.7) -> SourceConfig:
        """Create F2 laser (157nm) source."""
        ...
    @staticmethod
    def ar2_laser(sigma: float = 0.7) -> SourceConfig:
        """Create Ar2 excimer (126nm) source (hypothetical preset)."""
        ...
    @staticmethod
    def arf_laser(sigma: float = 0.7) -> SourceConfig:
        """ArF excimer laser, 193.368 nm (representative immersion-class values)."""
        ...
    @staticmethod
    def krf_laser(sigma: float = 0.7) -> SourceConfig:
        """KrF excimer laser, ~248.3 nm (representative values)."""
        ...
    @staticmethod
    def hg_i_line(sigma: float = 0.7) -> SourceConfig:
        """Mercury i-line lamp, 365.0153 nm (NIST air), CW."""
        ...
    @staticmethod
    def hg_h_line(sigma: float = 0.7) -> SourceConfig:
        """Mercury h-line lamp, 404.6563 nm (NIST air), CW."""
        ...
    @staticmethod
    def hg_g_line(sigma: float = 0.7) -> SourceConfig:
        """Mercury g-line lamp, 435.8328 nm (NIST air), CW."""
        ...
    @staticmethod
    def lpa_fel_bella_25nm(sigma: float = 0.7) -> SourceConfig:
        """LPA-FEL at the 25 nm / 500 MeV / 1 kHz design projection (BELLA demonstrated 420 nm at 1 Hz).

        Bandwidth is the derived SASE 2·rho·lambda of its illustrative beam (~266 pm).
        """
        ...
    @staticmethod
    def lpa_fel(
        wavelength_nm: float,
        sigma: float = 0.7,
        electron_energy_mev: float = 500.0,
        bandwidth_pm: Optional[float] = None,
        pulse_duration_fs: float = 10.0,
        rep_rate_hz: float = 1000.0,
        pulse_energy_uj: Optional[float] = None,
        undulator_period_mm: Optional[float] = None,
        undulator_k: Optional[float] = None,
        num_periods: int = 200,
        peak_current_a: Optional[float] = None,
        norm_emittance_um: Optional[float] = None,
        energy_spread_rel: Optional[float] = None,
        beta_m: Optional[float] = None,
    ) -> SourceConfig:
        """Create a custom LPA-FEL source at the given wavelength (nm).

        An undulator (period + K) derives the resonance wavelength (must
        match within 5 %); beam parameters add the derived FEL physics.
        ``bandwidth_pm`` (FWHM) is an explicit override; when omitted it is
        derived as 2·rho·lambda with undulator + beam, else 0.1 % of lambda.
        """
        ...
    @staticmethod
    def lpp_sn_13nm5(sigma: float = 0.9, drive_laser: str = "co2") -> SourceConfig:
        """Sn LPP at 13.5 nm (~250 W at IF); drive_laser co2 / solid_state_1um / thulium_2um."""
        ...
    @staticmethod
    def lpp_sn_13nm5_500w(sigma: float = 0.9) -> SourceConfig:
        """NXE:3800E-class Sn LPP, 500 W at IF (drive chain an assumed scaling of the NXE:3400B)."""
        ...
    @staticmethod
    def lpp_tb_6nm5(sigma: float = 0.9) -> SourceConfig:
        """Tb laser-produced-plasma source at 6.5 nm (beyond-EUV)."""
        ...
    @staticmethod
    def lpp_gd_6nm7(sigma: float = 0.9) -> SourceConfig:
        """Gd laser-produced-plasma source at 6.7 nm (beyond-EUV)."""
        ...
    @staticmethod
    def synchrotron_undulator(
        electron_energy_gev: float = 0.538,
        period_mm: float = 20.0,
        k: float = 1.0,
        num_periods: int = 100,
        harmonic: int = 1,
        ring_current_ma: float = 200.0,
        emittance_x_nm_rad: Optional[float] = None,
        emittance_y_nm_rad: Optional[float] = None,
        energy_spread_rel: float = 5e-4,
    ) -> SourceConfig:
        """Synchrotron undulator; wavelength derived from the resonance condition.

        Ring emittances (nm rad) derive the coherent fraction.
        """
        ...
    @staticmethod
    def synchrotron_compact_euv() -> SourceConfig:
        """Compact EUV undulator on an illustrative compact-ring beam (derived coherence)."""
        ...
    @staticmethod
    def synchrotron_liga_bending_magnet() -> SourceConfig:
        """LIGA-class bending-magnet beamline (2.5 GeV, 1.5 T)."""
        ...
    @staticmethod
    def hhg(
        driver_wavelength_nm: float = 800.0,
        gas: str = "neon",
        driver_intensity_w_cm2: float = 4e14,
        harmonic: int = 59,
        monochromator_bandwidth_pm: float = 15.0,
        full_comb: bool = False,
        driver_average_power_w: Optional[float] = None,
        conversion_efficiency: Optional[float] = None,
        comb_passband_nm: Optional[tuple[float, float] | list[float]] = None,
    ) -> SourceConfig:
        """High-harmonic generation source (odd harmonics, cutoff-checked).

        Power is derived: driver average power (default: an assumed 5 W) x an
        order-of-magnitude conversion efficiency (or ``conversion_efficiency``).
        ``derived_quantity("power_vs_measured_record") > 1`` marks a projection.
        """
        ...
    @staticmethod
    def hhg_ar_30nm() -> SourceConfig:
        """Argon HHG preset: harmonic 27 of 800 nm (29.6 nm); assumed 3 W driver -> 30 µW."""
        ...
    @staticmethod
    def hhg_ne_13nm5() -> SourceConfig:
        """Neon HHG preset reaching 13.56 nm at harmonic 59; assumed 20 W driver -> ~0.9 µW."""
        ...
    @staticmethod
    def xfel_flash_13nm5() -> SourceConfig:
        """FLASH-class SASE XFEL at 13.5 nm."""
        ...
    @staticmethod
    def xfel_fermi_seeded() -> SourceConfig:
        """FERMI-class seeded FEL near 13.5 nm."""
        ...
    @staticmethod
    def xfel_cw_sc_13nm5() -> SourceConfig:
        """PROJECTION: LCLS-II-like CW-SC FEL at 13.5 nm (derived ~135 W)."""
        ...
    @staticmethod
    def xfel_erl_13nm5() -> SourceConfig:
        """PROJECTION: ERL-FEL EUV-lithography design point (derived ~10.5 kW)."""
        ...
    @staticmethod
    def ics(
        target_wavelength_nm: float = 13.5,
        laser_wavelength_nm: float = 1030.0,
        laser_a0: float = 0.1,
        bunch_charge_pc: Optional[float] = None,
        laser_pulse_energy_mj: Optional[float] = None,
        electron_spot_um: Optional[float] = None,
        laser_spot_um: Optional[float] = None,
        rep_rate_hz: Optional[float] = None,
        collection_half_angle_mrad: Optional[float] = None,
    ) -> SourceConfig:
        """Inverse-Compton source tuned to the target wavelength (theoretical).

        Collision parameters derive the photon yield (Thomson luminosity).
        """
        ...
    @staticmethod
    def ics_compact_euv_13nm5() -> SourceConfig:
        """ICS design point with derived yield (~71 uW in the 2 % band)."""
        ...
    @staticmethod
    def ssmb_euv_13nm5() -> SourceConfig:
        """Steady-state-microbunching EUV design point (theoretical)."""
        ...
    @staticmethod
    def ssmb(
        ring_energy_mev: float = 400.0,
        modulation_wavelength_nm: float = 1053.0,
        target_wavelength_nm: float = 13.5,
        average_power_w: float = 1000.0,
        average_current_a: Optional[float] = None,
        peak_current_a: Optional[float] = None,
        bunching_factor: Optional[float] = None,
        radiator_periods: Optional[int] = None,
        radiator_k: Optional[float] = None,
    ) -> SourceConfig:
        """General SSMB source; radiator fields derive the coherent power."""
        ...
    @staticmethod
    def entangled_noon(
        wavelength_nm: float = 157.63,
        n: int = 2,
        fidelity: float = 1.0,
        pair_rate_hz: float = 1e6,
    ) -> SourceConfig:
        """N-photon entangled NOON-state source (entirely theoretical)."""
        ...
    @staticmethod
    def xray_tube(
        anode: str = "W",
        kvp: Optional[float] = None,
        current_ma: Optional[float] = None,
        be_window_um: float = 250.0,
    ) -> SourceConfig:
        """Hard X-ray tube (Kramers continuum + anode lines, Be window); for LIGA."""
        ...
    @staticmethod
    def dpp_xe_13nm5(sigma: float = 0.9) -> SourceConfig:
        """Xe DPP at 13.5 nm (metrology class, 10 W into 2π at the source)."""
        ...
    @staticmethod
    def dpp_sn_13nm5(sigma: float = 0.9) -> SourceConfig:
        """Sn DPP/LDP at 13.5 nm: 360 W into 2π at the source, ~34 W at IF."""
        ...
    @staticmethod
    def sxrl_ar_46nm9() -> SourceConfig:
        """Capillary-discharge Ne-like Ar soft-X-ray laser at 46.9 nm."""
        ...
    @staticmethod
    def sxrl_ag_13nm9() -> SourceConfig:
        """Transient-collisional Ni-like Ag soft-X-ray laser at 13.9 nm."""
        ...
    @staticmethod
    def sxrl(
        scheme: str = "ag_13nm9",
        pulse_energy_uj: Optional[float] = None,
        rep_rate_hz: Optional[float] = None,
        pulse_duration_ps: Optional[float] = None,
        rel_linewidth: Optional[float] = None,
    ) -> SourceConfig:
        """Plasma soft-X-ray laser by scheme (ar_46nm9/ag_13nm9/cd_13nm2/mo_18nm9)."""
        ...
    @staticmethod
    def betatron(
        electron_energy_mev: float = 200.0,
        plasma_density_cm3: float = 1e19,
        betatron_amplitude_um: float = 1.0,
        interaction_length_mm: float = 3.0,
        bunch_charge_pc: float = 50.0,
        rep_rate_hz: float = 10.0,
    ) -> SourceConfig:
        """Laser-wakefield betatron X-ray source (theoretical for lithography)."""
        ...
    @staticmethod
    def smith_purcell(
        target_wavelength_nm: float = 13.5,
        electron_energy_kev: float = 30.0,
        diffraction_order: int = 1,
        observation_angle_deg: float = 90.0,
        grating_period_nm: Optional[float] = None,
        num_periods: int = 100,
        beam_current_na: float = 10.0,
        coupling_efficiency: float = 1e-3,
        impact_height_nm: float = 0.0,
    ) -> SourceConfig:
        """Smith-Purcell free-electron grating source (theoretical)."""
        ...
    @property
    def wavelength_nm(self) -> float: ...
    @property
    def bandwidth_pm(self) -> float: ...
    @property
    def sigma_outer(self) -> float: ...
    @property
    def spectral_samples(self) -> int: ...
    @property
    def kind(self) -> str: ...
    @property
    def electron_energy_mev(self) -> Optional[float]: ...
    @property
    def pulse_duration_fs(self) -> Optional[float]: ...
    @property
    def rep_rate_hz(self) -> float: ...
    @property
    def transverse_coherence_fraction(self) -> Optional[float]: ...
    @property
    def average_power_w(self) -> Optional[float]: ...
    @property
    def shot_to_shot_rms(self) -> float: ...
    def derived_quantities(self) -> list[tuple[str, float, str, str]]:
        """Derived physical quantities as (name, value, unit, note) tuples."""
        ...
    def derived_quantity(self, name: str) -> Optional[float]:
        """Value of one derived quantity by name (None if not reported)."""
        ...
    def wafer_throughput(
        self,
        dose_mj_cm2: float = 30.0,
        optics_transmission: Optional[float] = None,
        n_mirrors: Optional[int] = None,
        mirror_reflectivity: Optional[float] = None,
        mask_efficiency: Optional[float] = None,
        wafer_diameter_mm: Optional[float] = None,
        field_width_mm: Optional[float] = None,
        field_height_mm: Optional[float] = None,
        slit_height_mm: Optional[float] = None,
        fields_per_wafer: Optional[int] = None,
        max_scan_speed_mm_s: Optional[float] = None,
        field_overhead_s: Optional[float] = None,
        wafer_overhead_s: Optional[float] = None,
        cd_nm: Optional[float] = None,
        preset: str = "euv",
    ) -> Optional[dict[str, Any]]:
        """Dose-limited wafer throughput (simplified scanner model, 🔶).

        ``preset`` ("euv" default: 10 Mo/Si mirrors at R 0.70, mask 0.65;
        "beuv": La/B at 0.641; "refractive": optics 0.30, mask 0.90, 8 mm
        slit) supplies every keyword left at None. Keys: preset,
        source_power_w, power_at_wafer_w, optics_transmission,
        mask_efficiency, dose_mj_cm2, fields_per_wafer, exposed_area_cm2,
        dose_limited_scan_speed_mm_s, scan_speed_mm_s, stage_limited,
        exposure_time_per_wafer_s, total_time_per_wafer_s, wafers_per_hour,
        pulses_per_point (+ photons_per_cd_square, relative_shot_noise with
        ``cd_nm``). None when the source reports no average power.
        """
    def xray_spectrum(
        self, n_bins: int = 200
    ) -> Optional[list[tuple[float, float]]]:
        """Relative photon spectrum (E_keV, fraction) for X-ray tube / betatron."""
        ...
    def spectral_flux_density(
        self, distance_mm: float, n_bins: int = 200
    ) -> Optional[list[tuple[float, float]]]:
        """Absolute (E_keV, photons/s/mm^2/keV) for X-ray tube / betatron."""
        ...
    @property
    def illumination(self) -> IlluminationSpec:
        """Pupil fill as a flat tuple, e.g. ("annular", 0.5, 0.8) (sigma in NA/λ)."""
        ...
    def with_illumination(self, shape: str | IlluminationSpec, *params: float) -> SourceConfig:
        """Copy with the pupil fill replaced: ``with_illumination("annular", 0.5, 0.8)``
        or ``with_illumination(("dipole", 0.7, 0.15, 0.0))``; the fill must stay
        inside sigma <= 1. Spectrum, power and derived quantities are unchanged."""
        ...
    def pupil_fill(self, n: int = 101, extent: float = 1.0) -> np.ndarray:
        """Relative source intensity on an (n, n) grid over sigma in
        [-extent, extent]^2 (``numpy.linspace(-extent, extent, n)`` per axis),
        indexed [iy, ix]."""
        ...
    def spectrum(self) -> list[tuple[float, float]]:
        """Spectral samples (wavelength_nm, weight) used for polychromatic images."""
        ...
    @property
    def photon_energy_ev(self) -> float:
        """Photon energy (eV) at the center wavelength."""
        ...
    @property
    def photon_density_per_mj_cm2(self) -> float:
        """Incident photons per nm² at 1 mJ/cm²."""
        ...
    @property
    def pulse_energy_j(self) -> Optional[float]:
        """Pulse energy (J); None for CW / untracked sources."""
        ...
    def __repr__(self) -> str: ...

class OpticsConfig:
    """Optics configuration (refractive dry/immersion, Schwarzschild, zone plate, or EUV projection)."""

    def __init__(
        self,
        numerical_aperture: float = 0.75,
        reduction: float = 4.0,
        flare_fraction: float = 0.02,
    ) -> None: ...
    @staticmethod
    def schwarzschild(
        numerical_aperture: float = 0.33,
        obscuration_ratio: float = 0.25,
        reduction: float = 4.0,
        mirror_reflectivity: float = 0.67,
        flare: float = 0.03,
    ) -> OpticsConfig:
        """Two-mirror reflective objective for EUV/BEUV/soft X-ray."""
        ...
    @staticmethod
    def zone_plate(
        outer_zone_width_nm: float, design_wavelength_nm: float
    ) -> OpticsConfig:
        """Fresnel zone plate (diffractive X-ray focusing)."""
        ...
    @staticmethod
    def immersion(
        numerical_aperture: float = 1.35,
        immersion_index: float = 1.437,
        reduction: float = 4.0,
        flare_fraction: float = 0.02,
    ) -> OpticsConfig:
        """Immersion refractive optics (NA may exceed 1, up to 0.95·immersion_index)."""
        ...
    @staticmethod
    def immersion_193i() -> OpticsConfig:
        """ArF water-immersion preset: NA 1.35, n = 1.437."""
        ...
    @staticmethod
    def euv_projection(
        numerical_aperture: float = 0.33,
        central_obscuration: float = 0.0,
        reduction: float = 4.0,
        flare: float = 0.0,
    ) -> OpticsConfig:
        """EUV projection optics with optional central obscuration (isotropic wafer-side pupil)."""
        ...
    @staticmethod
    def euv_nxe() -> OpticsConfig:
        """0.33-NA EUV projection optics (NXE-class, unobscured)."""
        ...
    @staticmethod
    def euv_high_na() -> OpticsConfig:
        """0.55-NA High-NA EUV projection optics, central obscuration 0.2·NA (assumed)."""
        ...
    def add_aberration(self, fringe_index: int, coefficient_waves: float) -> None:
        """Add a Zernike aberration coefficient (refractive and EUV projection optics)."""
        ...
    def with_multilayer_pupil(
        self,
        mirrors: list[tuple[float, float, float, float]],
        coating: str = "mo_si",
        periods: int = 40,
        period_nm: float = 6.9,
        gamma: float = 0.4,
    ) -> OpticsConfig:
        """Reflective optics with an angle-dependent multilayer pupil; mirrors = [(center_deg, tilt_deg, azimuth_deg, radial_deg), ...]."""
        ...
    @property
    def has_multilayer_pupil(self) -> bool: ...
    @property
    def immersion_index(self) -> float: ...
    @property
    def kind(self) -> str: ...
    @property
    def numerical_aperture(self) -> float: ...
    @property
    def reduction(self) -> float: ...
    @property
    def flare_fraction(self) -> float: ...
    def rayleigh_resolution(self, wavelength_nm: float) -> float: ...
    @property
    def central_obscuration(self) -> float:
        """Central obscuration radius as a fraction of NA (0 if unobscured)."""
        ...
    @property
    def paraxial_defocus(self) -> bool:
        """Whether the paraxial (legacy) defocus phase replaces the exact one."""
        ...
    def with_paraxial_defocus(self, enabled: bool = True) -> OpticsConfig:
        """Copy using the paraxial (True) or exact (False) defocus phase."""
        ...
    def pupil_map(self, wavelength_nm: float, n: int = 65, focus_nm: float = 0.0) -> np.ndarray:
        """Complex pupil P on an (n, n) grid over normalized pupil coordinates
        ``numpy.linspace(-1, 1, n)`` (units of the cutoff), indexed [iy, ix]."""
        ...
    def __repr__(self) -> str: ...

class MaskConfig:
    """Mask configuration (thin-mask model; the simulation field is one unit
    cell of a periodic mask)."""

    @staticmethod
    def line_space(
        cd_nm: float,
        pitch_nm: float,
        orientation: str = "vertical",
        offset_nm: float = 0.0,
    ) -> MaskConfig:
        """Line/space grating: bright field, opaque lines of width ``cd_nm``
        repeating every ``pitch_nm`` (one line centred at ``offset_nm``),
        periodic over the whole field."""
        ...
    @staticmethod
    def contact_hole(
        diameter_nm: float, pitch_x_nm: float, pitch_y_nm: Optional[float] = None
    ) -> MaskConfig:
        """Contact-hole array: dark field, clear square holes on a periodic
        lattice centred on the origin (``pitch_y_nm`` defaults to x)."""
        ...
    @staticmethod
    def from_features(
        features: list[Any],
        dark_field: bool = False,
        mask_type: str = "binary",
        transmission: float = 0.06,
        phase_deg: float = 180.0,
    ) -> MaskConfig:
        """Custom mask from features painted in order (later wins) — the
        canonical way to build an arbitrary mask. Items are feature dicts
        (``rect`` (x, y, w, h), ``gray_rect`` (+ transmittance), ``polygon``
        (vertices), ``line_space`` (cd, pitch, orientation, offset),
        ``rect_array`` (w, h, pitch_x, pitch_y, offset_x, offset_y)) or
        shorthands: an ``(x, y, w, h)`` rectangle (centre and size, nm) or a
        list of >= 3 ``(x, y)`` polygon vertices. ``mask_type``: "binary",
        "att_psm" (absorber sqrt(transmission)·exp(i·phase_deg)) or "alt_psm"."""
        ...
    def features(self) -> list[dict[str, object]]:
        """Feature dicts (inverse of ``from_features``)."""
        ...
    @property
    def dark_field(self) -> bool: ...
    def periodicity(self) -> Optional[tuple[float, Optional[float]]]:
        """Periods the square field must hold a whole number of, or None."""
        ...
    def check_commensurate(self, grid: GridConfig) -> None:
        """Raise ValueError (with a suggested fix) if the field is not a whole
        number of periods of every periodic primitive."""
        ...
    def commensurate_grid(
        self, size: int = 256, target_pixel_nm: float = 1.0
    ) -> GridConfig:
        """Grid of ``size`` pixels commensurate with this mask, pixel as close
        to (and not coarser than) ``target_pixel_nm`` as possible."""
        ...
    def rasterize(self, grid: GridConfig) -> np.ndarray:
        """Area-averaged complex transmittance (complex128, indexed [y, x])."""
        ...
    def rasterize_intensity(self, grid: GridConfig) -> np.ndarray:
        """Area-averaged intensity transmittance <|t|^2> (float64)."""
        ...
    def spectrum(self, grid: GridConfig) -> np.ndarray:
        """Exact mask spectrum in ``numpy.fft.fft2`` layout and scale
        (clear mask: DC = N^2)."""
        ...
    def spectrum_method(self, grid: GridConfig) -> str:
        """"analytic", "mixed" or "raster" — how ``spectrum`` is evaluated."""
        ...
    def validate(self) -> None:
        """Raise ValueError on invalid feature parameters."""
        ...
    def __repr__(self) -> str: ...

class ResistConfig:
    """Photoresist configuration."""

    def __init__(
        self,
        thickness_nm: float = 150.0,
        dill_a: float = 0.2,
        dill_b: float = 0.45,
        dill_c: float = 0.02,
        peb_diffusion_nm: float = 30.0,
        model: str = "mack",
    ) -> None: ...
    @staticmethod
    def vuv_fluoropolymer() -> ResistConfig:
        """Create VUV fluoropolymer resist with default parameters."""
        ...
    @property
    def thickness_nm(self) -> float: ...
    @property
    def dill_a(self) -> float: ...
    @property
    def dill_b(self) -> float: ...
    @property
    def dill_c(self) -> float: ...
    @property
    def peb_diffusion_nm(self) -> float: ...
    def __repr__(self) -> str: ...

class FilmStackConfig:
    """Thin-film stack (layers top to bottom over a substrate, below a
    superstrate); exact characteristic-matrix reflectance and fields."""

    def __init__(
        self,
        layers: Optional[list[tuple[str, float, float, float]]] = None,
        substrate_n: Optional[float] = None,
        substrate_k: Optional[float] = None,
        superstrate_n: float = 1.0,
    ) -> None:
        """No arguments: default VUV stack (150 nm resist on Si, vacuum above).
        ``layers``: (name, thickness_nm, n, k) top to bottom."""
        ...
    def add_layer(
        self, name: str, thickness_nm: float, n_real: float, n_imag: float
    ) -> None:
        """Append a layer below the existing ones."""
        ...
    def set_substrate(self, n_real: float, n_imag: float) -> None:
        """Set the substrate refractive index."""
        ...
    def set_superstrate(self, n_real: float) -> None:
        """Set the (real) superstrate index, e.g. 1.437 for water immersion."""
        ...
    @property
    def layers(self) -> list[tuple[str, float, float, float]]: ...
    @property
    def substrate(self) -> complex: ...
    @property
    def superstrate(self) -> complex: ...
    def reflectance(
        self, wavelength_nm: float, angle_deg: float = 0.0, polarization: str = "unpolarized"
    ) -> float:
        """Power reflectance (polarization "te"/"s", "tm"/"p", "unpolarized")."""
        ...
    def transmittance(
        self, wavelength_nm: float, angle_deg: float = 0.0, polarization: str = "unpolarized"
    ) -> float:
        """Power transmittance into the substrate."""
        ...
    def amplitude_coefficients(
        self, wavelength_nm: float, angle_deg: float = 0.0, polarization: str = "te"
    ) -> tuple[complex, complex]:
        """Complex (r, t) for "te" or "tm" (physics convention n + ik)."""
        ...
    def intensity_profile(
        self,
        wavelength_nm: float,
        z_nm: list[float],
        angle_deg: float = 0.0,
        polarization: str = "te",
    ) -> np.ndarray:
        """Exact |E(z)|² at depths z_nm (nm from the stack top), unit incident intensity."""
        ...
    def __repr__(self) -> str: ...

class ProcessConfig:
    """Process parameters."""

    dose_mj_cm2: float
    focus_nm: float
    development_time_s: float

    def __init__(
        self,
        dose_mj_cm2: float = 30.0,
        focus_nm: float = 0.0,
        development_time_s: float = 60.0,
    ) -> None: ...
    def __repr__(self) -> str: ...

class GridConfig:
    """Grid configuration."""

    def __init__(self, size: int = 512, pixel_nm: float = 1.0) -> None: ...
    @staticmethod
    def commensurate(
        pitch_x_nm: float,
        pitch_y_nm: Optional[float] = None,
        size: int = 256,
        target_pixel_nm: float = 1.0,
    ) -> GridConfig:
        """Grid whose square field is a whole multiple of the pitch(es) (their
        common period), pixel as close to and not coarser than the target."""
        ...
    @property
    def size(self) -> int: ...
    @property
    def pixel_nm(self) -> float: ...
    def field_size_nm(self) -> float: ...
    def periods_in_field(self, pitch_nm: float) -> float:
        """Number of periods of ``pitch_nm`` in the field."""
        ...
    def is_commensurate_with(self, pitch_nm: float) -> bool:
        """True if the field holds a whole number of periods of ``pitch_nm``."""
        ...
    def __repr__(self) -> str: ...

class AerialImageResult:
    """Aerial image simulation result."""

    @property
    def intensity(self) -> np.ndarray:
        """(ny, nx) intensity, a read-only view of the result buffer (no copy)."""
        ...
    @property
    def x_nm(self) -> np.ndarray: ...
    @property
    def y_nm(self) -> np.ndarray: ...
    @property
    def pixel_nm(self) -> float: ...
    @property
    def extent_nm(self) -> tuple[float, float, float, float]:
        """(x_min, x_max, y_min, y_max) field edges in nm (imshow extent, origin="lower")."""
        ...
    @property
    def shape(self) -> tuple[int, int]: ...
    def cross_section(
        self, y_nm: float = 0.0
    ) -> tuple[np.ndarray, np.ndarray]:
        """Extract a 1D cross-section along x at y=y_nm."""
        ...
    def image_contrast(self) -> float:
        """Compute image contrast: (Imax - Imin) / (Imax + Imin)."""
        ...
    def nils(self, threshold: float = 0.3) -> Optional[float]:
        """Legacy NILS at the y=0 cross-section (feature straddling the field
        centre, either tone; slope taken exactly at the crossing)."""
        ...
    def cd(self, threshold: float = 0.3, tone: str = "dark") -> Optional[float]:
        """Printed CD (nm): width of the ``tone`` feature ("dark" = below
        threshold, e.g. a line; "bright" = above, e.g. a space/hole) nearest
        the field centre on the y=0 cross-section, wrap-aware."""
        ...
    def nils_periodic(
        self,
        threshold: float = 0.3,
        tone: str = "dark",
        width_nm: Optional[float] = None,
    ) -> Optional[float]:
        """NILS = w*|dI/dx|/I_threshold of the ``tone`` feature nearest the
        field centre; ``w`` is the measured CD unless ``width_nm`` is given."""
        ...
    def image_log_slope(
        self, threshold: float = 0.3, tone: str = "dark"
    ) -> Optional[float]:
        """Image log-slope |d ln I/dx| (1/nm) at the edges of the ``tone``
        feature nearest the field centre."""
        ...
    def features(
        self, threshold: float = 0.3, tone: str = "dark"
    ) -> list[tuple[float, float, float, float]]:
        """All ``tone`` features as (left_nm, right_nm, width_nm, centre_nm)."""
        ...
    def __repr__(self) -> str: ...

class ResistProfileResult:
    """Resist profile simulation result."""

    @property
    def x_nm(self) -> np.ndarray: ...
    @property
    def height_nm(self) -> np.ndarray: ...
    @property
    def thickness_nm(self) -> float: ...
    def __repr__(self) -> str: ...
    @property
    def status(self) -> str:
        """Capability badge ("✅", "🔶", "🧪" or split) of the model behind this result."""
        ...
    @property
    def notes(self) -> list[str]:
        """The model's stated approximations."""
        ...

class SimulationEngine:
    """Core simulation engine. Precomputes TCC for efficient multi-evaluation."""

    def __init__(
        self,
        source: SourceConfig,
        optics: OpticsConfig,
        mask: MaskConfig,
        resist: Optional[ResistConfig] = None,
        grid: Optional[GridConfig] = None,
        max_kernels: int = 30,
        defocus_model: str = "exact",
        kernel_energy_fraction: float = 1.0,
        source_points_per_axis: Optional[int] = None,
        vector: Optional[VectorSettings] = None,
        normalization: str = "clear_field",
        kernel_cache_capacity: int = 32,
    ) -> None:
        """Build the imaging model (GIL released). ``defocus_model``: "exact"
        or "kernel_phase" (legacy); ``normalization``: "clear_field" or
        "absolute"; ``vector``: vector imaging settings (scalar if None)."""
        ...
    def compute_aerial_image(
        self, focus_nm: float = 0.0, mask: Optional[MaskConfig] = None
    ) -> AerialImageResult:
        """Monochromatic aerial image at defocus ``focus_nm`` (``mask`` overrides the engine's)."""
        ...
    def compute_polychromatic(
        self, focus_nm: float = 0.0, mask: Optional[MaskConfig] = None
    ) -> AerialImageResult:
        """Compute polychromatic aerial image (narrow-band approximation: focus shift per spectral sample)."""
        ...
    def compute_multiwavelength(
        self, focus_nm: float = 0.0, mask: Optional[MaskConfig] = None
    ) -> AerialImageResult:
        """Exact per-wavelength polychromatic image (kernels rebuilt at every spectral sample)."""
        ...
    def compute_through_focus(
        self, focus_nm: list[float], mask: Optional[MaskConfig] = None
    ) -> list[AerialImageResult]:
        """Aerial images at several focus planes."""
        ...
    def compute_noon_ideal_image(
        self,
        num_photons: int = 2,
        fidelity: float = 1.0,
        focus_nm: float = 0.0,
        mask: Optional[MaskConfig] = None,
    ) -> NoonImageResult:
        """Ideal N00N-limit exposure F*I(lambda/N) + (1-F)*I(lambda)^N (🧪).

        The same source fill, optics, mask and grid imaged at lambda/N (Boto et
        al.'s ideal limit, not a simulation of state preparation). Needs
        clear-field normalization and pixel <= lambda/(4 N NA), else ValueError.
        """
        ...
    def compute_from_transmittance(
        self, transmittance: np.ndarray, focus_nm: float = 0.0
    ) -> AerialImageResult:
        """Aerial image of a complex (n, n) transmittance array on the engine grid."""
        ...
    def source_points(self) -> tuple[np.ndarray, np.ndarray, np.ndarray]:
        """(sx, sy, weight) of the engine's source sampling (sigma units, weights sum 1)."""
        ...
    def spectral_samples(self) -> list[tuple[float, float]]:
        """(wavelength_nm, weight) spectral samples of the source."""
        ...
    def kernel_eigenvalues(self, focus_nm: float = 0.0) -> np.ndarray:
        """SOCS eigenvalues (decreasing) of the kernel set at focus_nm."""
        ...
    def kernels(self, focus_nm: float = 0.0) -> tuple[np.ndarray, np.ndarray]:
        """(eigenvalues, kernels) with complex kernels of shape (k, n, n) (fft2 layout)."""
        ...
    def captured_energy_fraction(self) -> float:
        """Captured TCC energy fraction of the in-focus kernel set."""
        ...
    @property
    def wavelength_nm(self) -> float: ...
    @property
    def numerical_aperture(self) -> float: ...
    def imaging_diagnostics(self, focus_nm: float = 0.0) -> dict[str, Any]:
        """Kernel-set bookkeeping: num_kernels, captured_energy_fraction, num_frequencies,
        num_columns, method, support_exceeds_nyquist, clear_field_intensity, normalized,
        imaging_model, num_source_points, defocus_model."""
        ...
    def clear_field_intensity(self, focus_nm: float = 0.0) -> float:
        """Absolute clear-field intensity TCC(0,0); images are divided by it by default."""
        ...
    def measure_cd(
        self,
        dose_mj_cm2: float = 30.0,
        focus_nm: float = 0.0,
        threshold: float = 0.3,
        dose_to_clear_mj_cm2: Optional[float] = None,
        mask: Optional[MaskConfig] = None,
    ) -> float:
        """Printed CD (nm) of the mask's feature tone nearest the field centre.

        Without ``dose_to_clear_mj_cm2``, ``threshold`` is the intensity
        threshold and ``dose_mj_cm2`` does not enter; with it, a
        constant-threshold resist uses ``dose_to_clear_mj_cm2 / dose_mj_cm2``.
        """
        ...
    def compute_resist_profile(
        self,
        dose_mj_cm2: float = 30.0,
        focus_nm: float = 0.0,
        dev_time_s: float = 60.0,
    ) -> ResistProfileResult:
        """Compute resist profile at given dose and focus."""
        ...
    def image_contrast(self, focus_nm: float = 0.0, mask: Optional[MaskConfig] = None) -> float:
        """Image contrast (Imax - Imin)/(Imax + Imin) at focus_nm."""
        ...
    def num_kernels(self) -> int:
        """Number of SOCS kernels in the decomposition."""
        ...
    @property
    def source(self) -> SourceConfig: ...
    @property
    def optics(self) -> OpticsConfig: ...
    @property
    def mask(self) -> MaskConfig: ...
    @property
    def resist(self) -> ResistConfig: ...
    @property
    def grid(self) -> GridConfig: ...
    def __repr__(self) -> str: ...

class BatchSimulator:
    """Process-window and focus sweeps over one imaging engine (GIL released)."""

    def __init__(
        self,
        source: SourceConfig,
        optics: OpticsConfig,
        mask: MaskConfig,
        grid: Optional[GridConfig] = None,
        max_kernels: int = 30,
        defocus_model: str = "exact",
        kernel_energy_fraction: float = 1.0,
        source_points_per_axis: Optional[int] = None,
        vector: Optional[VectorSettings] = None,
        normalization: str = "clear_field",
        kernel_cache_capacity: int = 32,
    ) -> None: ...
    def process_window(
        self,
        doses: list[float],
        focuses: list[float],
        cd_threshold: float = 0.3,
        cd_target_nm: float = 65.0,
        cd_tolerance_pct: float = 10.0,
    ) -> ProcessWindowResult:
        """Dose-aware process window. ``cd_threshold`` is the intensity
        threshold at the nominal dose (median of ``doses``); at dose d the
        printed edge is the contour ``cd_threshold * d_nom / d``."""
        ...
    def process_window_threshold(
        self,
        doses: list[float],
        focuses: list[float],
        dose_to_clear_mj_cm2: float,
        cd_target_nm: float = 65.0,
        cd_tolerance_pct: float = 10.0,
    ) -> ProcessWindowResult:
        """Process window with an explicit constant-threshold resist (a point
        clears where dose * I >= dose_to_clear_mj_cm2)."""
        ...
    def batch_defocus(
        self,
        focuses: list[float],
    ) -> list[tuple[float, np.ndarray]]:
        """Batch compute aerial images at multiple focus values."""
        ...

class ProcessWindowResult:
    """Process window analysis result (constant-threshold resist model or a
    tabulated focus-exposure matrix)."""

    @staticmethod
    def from_cd_matrix(
        doses: list[float],
        focuses: list[float],
        cd_matrix: np.ndarray,
        cd_target_nm: float = 65.0,
        cd_tolerance_pct: float = 10.0,
    ) -> ProcessWindowResult:
        """From a tabulated FEM ``cd_matrix[dose, focus]`` (NaN = no CD);
        CD is linear in dose between rows."""
        ...
    @staticmethod
    def from_profiles(
        x_nm: list[float],
        focuses: list[float],
        profiles: np.ndarray,
        doses: list[float],
        dose_to_clear_mj_cm2: float,
        tone: str = "dark",
        cd_target_nm: float = 65.0,
        cd_tolerance_pct: float = 10.0,
    ) -> ProcessWindowResult:
        """From y=0 intensity cross-sections ``profiles[focus, pixel]`` (one
        field period each, uniform pixel centres ``x_nm``)."""
        ...
    @property
    def cd_matrix(self) -> np.ndarray:
        """CD (nm) per (dose, focus); NaN where nothing prints."""
        ...
    @property
    def doses(self) -> np.ndarray: ...
    @property
    def focuses(self) -> np.ndarray: ...
    @property
    def dose_limits(self) -> list[Optional[tuple[float, float]]]:
        """Per focus: in-spec (dose_min, dose_max) in mJ/cm^2, or None."""
        ...
    @property
    def tone(self) -> Optional[str]: ...
    @property
    def dose_to_clear_mj_cm2(self) -> Optional[float]: ...
    @property
    def cd_target_nm(self) -> float: ...
    @property
    def cd_tolerance_pct(self) -> float: ...
    def depth_of_focus(self) -> float:
        """Depth of focus (nm) at the dose-to-size around best focus."""
        ...
    def exposure_latitude(self) -> float:
        """Exposure latitude (%) at best focus."""
        ...
    def nominal_dose(self) -> float:
        """Median of the swept doses (mJ/cm^2)."""
        ...
    def best_focus(self) -> float:
        """Focus (nm) of zero dCD/dz on the Bossung curve at the dose-to-size."""
        ...
    def dose_to_size(self) -> Optional[float]:
        """Dose (mJ/cm^2) printing the target CD at best focus."""
        ...
    def iso_focal_dose(self) -> Optional[float]:
        """Dose (mJ/cm^2) with the least focus-sensitive CD."""
        ...
    def cd_at(self, dose_mj_cm2: float, focus_nm: float) -> Optional[float]:
        """CD (nm) at any dose and (interpolated) focus."""
        ...
    def el_vs_dof(self, n_points: int = 51) -> tuple[np.ndarray, np.ndarray]:
        """Exposure-defocus curve: (dof_nm, max exposure latitude %) arrays."""
        ...
    def dof_at_el(self, el_pct: float = 5.0) -> Optional[dict[str, float]]:
        """Largest-DOF process-window rectangle with EL >= ``el_pct``."""
        ...
    def el_at_dof(self, dof_nm: float) -> Optional[dict[str, float]]:
        """Largest-EL process-window rectangle of focus extent ``dof_nm``."""
        ...
    def max_area_rectangle(self) -> Optional[dict[str, float]]:
        """Process-window rectangle maximizing DOF x EL."""
        ...
    def spec_limits_nm(self) -> tuple[float, float]:
        """(cd_min_nm, cd_max_nm) spec band."""
        ...
    def dose_to_size_at(self, focus_nm: float) -> Optional[float]:
        """Dose-to-size (mJ/cm^2) at any focus in the sampled range."""
        ...
    def dose_limits_at(self, focus_nm: float) -> Optional[tuple[float, float]]:
        """In-spec dose interval at any focus in the sampled range."""
        ...
    def bossung_curves(self) -> list[tuple[float, list[tuple[float, float]]]]:
        """[(dose_mj_cm2, [(focus_nm, cd_nm), ...]), ...] (NaN = not printed)."""
        ...
    def summary(self) -> dict[str, object]:
        """Key process-window numbers (best focus, DOF, EL, DOF@5%EL, ...)."""
        ...
    def __repr__(self) -> str: ...
    @property
    def status(self) -> str:
        """Capability badge ("✅", "🔶", "🧪" or split) of the model behind this result."""
        ...
    @property
    def notes(self) -> list[str]:
        """The model's stated approximations."""
        ...

class PySpherePacking:
    """Nanosphere lattice packing (HCP, FCC, or simple cubic)."""

    FCC: PySpherePacking
    HCP: PySpherePacking
    SIMPLE_CUBIC: PySpherePacking

class PyNanosphereArrayConfig:
    """Nanosphere array configuration for MNSL simulation."""

    diameter_nm: float
    pitch_nm: float
    orientation_deg: float
    n_real: float
    n_imag: float

    def __init__(
        self,
        diameter_nm: float,
        pitch_nm: float,
        orientation_deg: float = 0.0,
        n_real: float = 1.56,
        n_imag: float = 0.001,
        packing: Optional[PySpherePacking] = None,
    ) -> None: ...
    @staticmethod
    def polystyrene_spheres(
        diameter_nm: float, pitch_nm: float
    ) -> PyNanosphereArrayConfig: ...
    @staticmethod
    def silica_spheres(
        diameter_nm: float, pitch_nm: float
    ) -> PyNanosphereArrayConfig: ...
    def volume_fraction(self) -> float: ...

class PySubstrateCoupling:
    """Substrate coupling parameters for MNSL emission."""

    coupling_strength: float
    enable_nearfield: bool

    def __init__(
        self, coupling_strength: float = 0.5, enable_nearfield: bool = True
    ) -> None: ...

class PyMnslConfig:
    """MNSL (Moire Nanosphere Lithography) simulation configuration."""

    bottom_array: PyNanosphereArrayConfig
    top_array: PyNanosphereArrayConfig
    separation_nm: float
    wavelength_nm: float

    def __init__(
        self,
        bottom_array: PyNanosphereArrayConfig,
        top_array: PyNanosphereArrayConfig,
        separation_nm: float = 100.0,
        substrate: Optional[PySubstrateCoupling] = None,
        wavelength_nm: float = 157.0,
    ) -> None: ...

class PyMnslEngine:
    """MNSL emission-pattern computation engine."""

    def __init__(
        self, config: PyMnslConfig, grid_size: int, pixel_nm: float
    ) -> None: ...
    def compute_emission(self) -> PyMnslResult: ...

class PyMnslResult:
    """MNSL simulation result."""

    @property
    def emission_pattern(self) -> np.ndarray: ...
    @property
    def moire_pattern(self) -> np.ndarray: ...
    @property
    def enhancement_factors(self) -> np.ndarray: ...
    @property
    def moire_period_nm(self) -> float: ...
    @property
    def peak_enhancement(self) -> float: ...
    @property
    def peak_positions(self) -> list[list[float]]:
        """Peak positions as [x_nm, y_nm] pairs."""
        ...
    @property
    def total_emission_power(self) -> float: ...
    def coordinates(self) -> tuple[list[float], list[float]]:
        """(x_coords, y_coords) cell centres in nm."""
        ...
    def cross_section_x(self, y_nm: float) -> list[float]:
        """Emission along x at the row nearest y_nm."""
        ...
    def cross_section_y(self, x_nm: float) -> list[float]:
        """Emission along y at the column nearest x_nm."""
        ...
    @property
    def status(self) -> str:
        """Capability badge ("✅", "🔶", "🧪" or split) of the model behind this result."""
        ...
    @property
    def notes(self) -> list[str]:
        """The model's stated approximations."""
        ...

def py_simulate_moire_emission(
    sphere_diameter_nm: float,
    array_pitch_nm: float,
    rotation_angle_deg: float,
    separation_nm: float = 100.0,
    grid_size: int = 256,
    pixel_nm: float = 2.0,
) -> PyMnslResult:
    """One-call MNSL moire emission simulation."""
    ...

class VolumetricResult:
    """A z-resolved scalar field (PAC, dose, or arrival times) on a 3D grid."""

    @staticmethod
    def from_array(
        values: np.ndarray,
        x_range_nm: tuple[float, float],
        y_range_nm: tuple[float, float],
        z_range_nm: tuple[float, float],
    ) -> VolumetricResult:
        """Wrap a (nz, ny, nx) float64 array with physical extents (nm)."""
        ...
    @property
    def values(self) -> np.ndarray:
        """(nz, ny, nx) values, a read-only view of the result buffer (no copy)."""
        ...
    @property
    def x_nm(self) -> np.ndarray: ...
    @property
    def y_nm(self) -> np.ndarray: ...
    @property
    def z_nm(self) -> np.ndarray: ...
    @property
    def shape(self) -> tuple[int, int, int]: ...
    @property
    def extent_nm(
        self,
    ) -> tuple[tuple[float, float], tuple[float, float], tuple[float, float]]:
        """((x_min, x_max), (y_min, y_max), (z_min, z_max)) in nm."""
        ...
    def cd_at_z(self, threshold: float) -> list[Optional[float]]:
        """Center-row CD in every z-slice at the given threshold (top to bottom)."""
        ...
    def depth_map(self, threshold: float) -> HeightMapResult:
        """Per-column development depth map (nm) thresholding this PAC volume."""
        ...
    def __repr__(self) -> str: ...

class HeightMapResult:
    """A 2D height/depth map (nm) on a lateral grid with physical coordinates."""

    @property
    def values(self) -> np.ndarray:
        """(ny, nx) values, a read-only view of the result buffer (no copy)."""
        ...
    @property
    def x_nm(self) -> np.ndarray: ...
    @property
    def y_nm(self) -> np.ndarray: ...
    @property
    def shape(self) -> tuple[int, int]: ...
    def __repr__(self) -> str: ...

class LigaResult:
    """LIGA deep-X-ray exposure result: depth dose, contrast, development,
    and (for absolute spectra) dose rates and exposure time."""

    @property
    def depth_dose(self) -> tuple[np.ndarray, np.ndarray]:
        """(z_um, dose_kj_cm3) depth-dose profile arrays."""
        ...
    @property
    def dose_ratio(self) -> float: ...
    @property
    def top_dose_kj_cm3(self) -> float: ...
    @property
    def bottom_dose_kj_cm3(self) -> float: ...
    @property
    def exceeds_damage_ceiling(self) -> bool: ...
    @property
    def exposure_time_s(self) -> Optional[float]:
        """Exposure time (s) to reach the bottom dose; absolute spectra only."""
        ...
    @property
    def top_dose_rate_kj_cm3_s(self) -> Optional[float]: ...
    @property
    def bottom_dose_rate_kj_cm3_s(self) -> Optional[float]: ...
    @property
    def resist_power_density_w_mm2(self) -> Optional[float]: ...
    @property
    def exposure_charge_ma_h(self) -> Optional[float]:
        """Ring current x exposure time (mA h); bending-magnet beamlines only."""
        ...
    @property
    def mean_energy_top_kev(self) -> float: ...
    @property
    def mean_energy_bottom_kev(self) -> float: ...
    @property
    def window_power_fraction(self) -> float:
        """Fraction of the transmitted beam power inside the sampled energy window."""
        ...
    @property
    def diffraction(self) -> str:
        """Lateral proximity model used: 'fresnel' or 'gaussian'."""
        ...
    @property
    def spectrum(self) -> str:
        """Spectrum path: flux_density, tabulated, bending_magnet,
        bending_magnet_beamline, xray_tube or betatron."""
        ...
    @property
    def fresnel_scale_nm(self) -> tuple[float, float]:
        """Dose-weighted sqrt(lambda g) at the resist top and bottom (nm)."""
        ...
    @property
    def sampling_resolved(self) -> bool: ...
    @property
    def warnings(self) -> list[str]:
        """Sampling and beamline-model warnings (empty when none apply)."""
        ...
    @property
    def volume(self) -> VolumetricResult: ...
    @property
    def developed_depth(self) -> HeightMapResult: ...
    def __repr__(self) -> str: ...
    @property
    def status(self) -> str:
        """Capability badge ("✅", "🔶", "🧪" or split) of the model behind this result."""
        ...
    @property
    def notes(self) -> list[str]:
        """The model's stated approximations."""
        ...

def expose_volumetric(
    source: SourceConfig,
    optics: OpticsConfig,
    mask: MaskConfig,
    film_stack: FilmStackConfig,
    resist: ResistConfig,
    grid: GridConfig,
    dose_mj_cm2: float = 30.0,
    nz: int = 64,
    n_defocus_planes: int = 8,
    dose_steps: int = 1,
    base_defocus_nm: float = 0.0,
    resist_layer: int = 0,
    max_kernels: int = 20,
) -> VolumetricResult:
    """Compute a z-resolved (volumetric) PAC latent image through the resist."""
    ...

def develop_fast_marching(
    volume: VolumetricResult,
    resist: ResistConfig,
    pixel_xy_nm: float,
    pixel_z_nm: float,
    surface_rate_ratio: float = 1.0,
    inhibition_depth_nm: float = 0.0,
    lateral: str = "reflecting",
) -> VolumetricResult:
    """Fast-marching development of a PAC volume; returns arrival times (s)."""
    ...

class LevelSetResult:
    """Level-set development result: arrival times, final phi, diagnostics."""

    @property
    def arrival_times(self) -> VolumetricResult: ...
    @property
    def phi(self) -> VolumetricResult: ...
    @property
    def dev_time_s(self) -> float: ...
    @property
    def steps(self) -> int: ...
    @property
    def reinitializations(self) -> int: ...
    @property
    def dissolved_thickness_nm(self) -> float: ...
    @property
    def final_rate_factor(self) -> float: ...
    def height_map(self) -> HeightMapResult:
        """Remaining-height map (nm) at the end of development."""
        ...
    def developed(self, time_s: Optional[float] = None) -> VolumetricResult:
        """Developed-region indicator (1/0) at time_s (default: dev_time_s)."""
        ...
    def __repr__(self) -> str: ...
    @property
    def status(self) -> str:
        """Capability badge ("✅", "🔶", "🧪" or split) of the model behind this result."""
        ...
    @property
    def notes(self) -> list[str]:
        """The model's stated approximations."""
        ...

def develop_level_set(
    volume: VolumetricResult,
    resist: ResistConfig,
    dev_time_s: float,
    surface_rate_ratio: float = 1.0,
    inhibition_depth_nm: float = 0.0,
    lateral: str = "periodic",
    depletion: str = "none",
    depletion_time_constant_s: Optional[float] = None,
    loading_capacity_nm: Optional[float] = None,
    loading_length_nm: Optional[float] = None,
    cfl: float = 0.5,
    reinit_interval: int = 4,
    band_cells: float = 6.0,
    max_steps: int = 2000000,
) -> LevelSetResult:
    """Level-set development with the rate re-evaluated at the moving front."""
    ...

def development_rate(
    volume: VolumetricResult,
    resist: ResistConfig,
    surface_rate_ratio: float = 1.0,
    inhibition_depth_nm: float = 0.0,
) -> VolumetricResult:
    """Static dissolution-rate volume (nm/s) with optional surface inhibition."""
    ...

def peb_gaussian(
    volume: VolumetricResult,
    lateral_nm: float,
    vertical_nm: Optional[float] = None,
    vertical_scale: Optional[np.ndarray] = None,
    vertical_surface_ratio: Optional[float] = None,
    vertical_decay_nm: Optional[float] = None,
    lateral: str = "periodic",
) -> VolumetricResult:
    """Anisotropic PEB with exact Gaussian kernels (optional depth-dependent D_z)."""
    ...

class CarPebResult:
    """Chemically amplified resist state after the post-exposure bake."""

    @property
    def protected(self) -> VolumetricResult: ...
    @property
    def acid(self) -> VolumetricResult: ...
    @property
    def quencher(self) -> VolumetricResult: ...
    @property
    def neutralized_total(self) -> float: ...
    @property
    def steps(self) -> int: ...
    @property
    def time_step_s(self) -> float: ...
    def __repr__(self) -> str: ...
    @property
    def status(self) -> str:
        """Capability badge ("✅", "🔶", "🧪" or split) of the model behind this result."""
        ...
    @property
    def notes(self) -> list[str]:
        """The model's stated approximations."""
        ...

def peb_car(
    volume: VolumetricResult,
    peb_time_s: float = 60.0,
    k_amp: float = 0.1,
    k_quench: float = 10.0,
    quencher: float = 0.15,
    acid_diffusivity_nm2_s: float = 2.0,
    quencher_diffusivity_nm2_s: float = 0.5,
    vertical_ratio: float = 1.0,
    lateral: str = "periodic",
    max_time_step_s: Optional[float] = None,
) -> CarPebResult:
    """Chemically amplified resist acid/quencher reaction-diffusion bake."""
    ...

def height_map_from_times(
    times: VolumetricResult, dev_time_s: float
) -> HeightMapResult:
    """Remaining-height map (nm) after dev_time_s from a fast-marching volume."""
    ...

def simulate_liga(
    critical_energy_kev: float = 6.23,
    resist_thickness_um: float = 500.0,
    cd_nm: float = 5000.0,
    pitch_nm: float = 10000.0,
    grid_size: int = 128,
    pixel_nm: float = 200.0,
    nz: int = 64,
    diffraction: str = "fresnel",
    proximity_gap_um: float = 100.0,
    energy_bins: int = 100,
    photoelectron_blur: bool = False,
    filters: Optional[list[tuple[str, float]]] = None,
    absorber: str = "Au",
    absorber_thickness_um: float = 20.0,
    membrane: str = "Ti",
    membrane_thickness_um: float = 2.0,
    target_bottom_dose_kj_cm3: float = 3.0,
    damage_dose_kj_cm3: float = 20.0,
    source: Optional[SourceConfig] = None,
    source_distance_m: Optional[float] = None,
    horizontal_acceptance_mrad: float = 5.0,
    vertical_scan_mm: Optional[float] = None,
    flux_density: Optional[list[tuple[float, float]]] = None,
    spectrum_table: Optional[list[tuple[float, float]]] = None,
    strict_sampling: bool = False,
) -> LigaResult:
    """Simulate a LIGA deep-X-ray shadow exposure of thick PMMA.

    ``source``: a synchrotron bending magnet (absolute with source_distance_m
    and vertical_scan_mm), an X-ray tube or a betatron (absolute with
    source_distance_m: the source's spectral flux density at the mask).
    """
    ...

def grayscale_height_map(
    source: SourceConfig,
    optics: OpticsConfig,
    grid: GridConfig,
    transmittance: np.ndarray,
    dose_mj_cm2: float,
    d_th: float,
    d_clear: float,
    thickness_nm: float,
    max_kernels: int = 20,
) -> HeightMapResult:
    """Grayscale 2.5D height map from a continuous intensity-transmittance mask."""
    ...

def blazed_grating(
    n: int, period_px: int, depth_nm: float, thickness_nm: float
) -> np.ndarray:
    """Target surface-height map (nm) for a blazed (sawtooth) grating."""
    ...

def microlens_array(
    n: int, pitch_px: int, sag_nm: float, thickness_nm: float
) -> np.ndarray:
    """Target surface-height map (nm) for a square array of parabolic microlenses."""
    ...

def grayscale_transmittance_for_target(
    target_height: np.ndarray,
    thickness_nm: float,
    d_th: float,
    d_clear: float,
    exposure_dose: float,
) -> np.ndarray:
    """Intensity-transmittance mask that prints target_height in one exposure."""
    ...

def simulate_interference(
    preset: str,
    wavelength_nm: float,
    n_medium: float,
    half_angle_deg: float,
    nx: int,
    ny: int,
    nz: int,
    x_span_nm: float,
    y_span_nm: float,
    z_span_nm: float,
    dose_scale: float = 1.0,
    dill_c: float = 0.02,
    two_photon: bool = False,
) -> VolumetricResult:
    """Multi-beam interference lithography: expose a lattice into a PAC volume."""
    ...

def quantum_aerial_image(
    classical: np.ndarray,
    n: int = 2,
    wavelength_nm: float = 157.63,
    na: float = 0.75,
    fidelity: float = 1.0,
) -> np.ndarray:
    """Deprecated alias of n_photon_absorption_image: classical I^N (🧪).

    fidelity / wavelength_nm / na are validated but do not change the result
    (classical N-photon absorption involves no entanglement). For the N00N
    model use TwoBeamNPhoton or SimulationEngine.compute_noon_ideal_image.
    """
    ...

# --- Vector (polarized) pupil model (optics::vector) ---

class VectorSettings:
    """Vector (polarized) imaging settings.

    polarization: "unpolarized", "x", "y", "te" (azimuthal), "tm" (radial),
    or "linear" (at angle_deg from x). image_index is the homogeneous image
    medium index (1.0 = air); obliquity switches the radiometric factor
    (cos θ_obj / cos θ_img)^½; reduction is the projection reduction ratio;
    film_n / film_k give an optional film entrance interface (n + ik).
    """

    def __init__(
        self,
        polarization: str = "unpolarized",
        angle_deg: float = 0.0,
        image_index: float = 1.0,
        obliquity: bool = True,
        reduction: float = 4.0,
        film_n: Optional[float] = None,
        film_k: float = 0.0,
    ) -> None: ...
    @property
    def polarization(self) -> str: ...
    @property
    def angle_deg(self) -> Optional[float]:
        """Linear polarization angle (deg); None unless polarization == "linear"."""
        ...
    @property
    def image_index(self) -> float: ...
    @property
    def obliquity(self) -> bool: ...
    @property
    def reduction(self) -> float: ...
    @property
    def film_n(self) -> Optional[float]: ...
    @property
    def film_k(self) -> Optional[float]: ...
    def validate(self, na: float) -> None:
        """Raise ValueError if the settings are inconsistent with the image-side NA."""
        ...
    def columns_per_source_point(self) -> int:
        """Imaging-matrix columns per source point (3 field components × states)."""
        ...
    def __repr__(self) -> str: ...

def vector_pupil_field(
    settings: VectorSettings,
    na: float,
    px: float,
    py: float,
    sx: float = 0.0,
    sy: float = 0.0,
) -> np.ndarray:
    """Image-side field (n_states, 3) complex of the order at pupil (px, py) in NA units."""
    ...

def vector_pupil_map(
    settings: VectorSettings,
    na: float,
    n: int = 65,
    sx: float = 0.0,
    sy: float = 0.0,
) -> np.ndarray:
    """Pupil polarization map, complex (n_states, 3, n, n) indexed [state, component, iy, ix]."""
    ...

def radiometric_factor(
    pupil_na: float, image_index: float = 1.0, reduction: float = 4.0
) -> float:
    """Radiometric amplitude factor (cos θ_obj / cos θ_img)^½ at pupil_na = NA·|p|."""
    ...

def fresnel_coefficients(
    n1: float, n2_real: float, n2_imag: float, sin_theta1: float
) -> dict[str, complex | float]:
    """Fresnel r_s, r_p, t_s, t_p, sin/cos θ₂ (complex) and R_s, R_p, T_s, T_p (real)."""
class LigaEdgeProfile:
    """Dose across a single straight absorber edge (analytic Fresnel solution)."""

    @property
    def x_nm(self) -> np.ndarray: ...
    @property
    def z_um(self) -> np.ndarray: ...
    @property
    def dose_kj_cm3(self) -> np.ndarray:
        """(n_depths, n_x) absolute dose in kJ/cm^3."""
        ...
    @property
    def open_dose_kj_cm3(self) -> np.ndarray: ...
    @property
    def absorber_dose_kj_cm3(self) -> np.ndarray: ...
    def edge_positions_nm(self, threshold_kj_cm3: float) -> list[Optional[float]]:
        """Developed-edge position (nm) at each depth; None if not developed."""
        ...
    def sidewall_angle_deg(self, threshold_kj_cm3: float) -> Optional[float]:
        """Least-squares sidewall angle from vertical (deg)."""
        ...
    def max_gradient_kj_cm3_nm(self) -> list[float]: ...
    def __repr__(self) -> str: ...
    @property
    def status(self) -> str:
        """Capability badge ("✅", "🔶", "🧪" or split) of the model behind this result."""
        ...
    @property
    def notes(self) -> list[str]:
        """The model's stated approximations."""
        ...

def liga_edge_profile(
    critical_energy_kev: float = 6.23,
    resist_thickness_um: float = 500.0,
    proximity_gap_um: float = 100.0,
    x_min_nm: float = -3000.0,
    x_max_nm: float = 3000.0,
    dx_nm: float = 10.0,
    depths_um: Optional[list[float]] = None,
    energy_bins: int = 100,
    photoelectron_blur: bool = False,
    filters: Optional[list[tuple[str, float]]] = None,
    absorber: str = "Au",
    absorber_thickness_um: float = 20.0,
    membrane: str = "Ti",
    membrane_thickness_um: float = 2.0,
    target_bottom_dose_kj_cm3: float = 3.0,
    source: Optional[SourceConfig] = None,
    spectrum_table: Optional[list[tuple[float, float]]] = None,
    flux_density: Optional[list[tuple[float, float]]] = None,
) -> LigaEdgeProfile:
    """Fine 1D Fresnel dose profile across a straight LIGA absorber edge."""
    ...

class MultilayerMirror:
    """Periodic EUV/BEUV multilayer mirror (Parratt + Nevot-Croce, Henke constants)."""

    @staticmethod
    def mo_si(periods: int = 40, period_nm: float = 6.9, gamma: float = 0.4) -> MultilayerMirror:
        """Mo/Si for 13.5 nm (Si on Mo, Mo fraction gamma, SiO2 substrate)."""
        ...
    @staticmethod
    def la_b4c(periods: int = 200, period_nm: float = 3.37, gamma: float = 0.4) -> MultilayerMirror:
        """La/B4C for 6.x nm (B4C on La, La fraction gamma, Si substrate)."""
        ...
    @staticmethod
    def la_b(periods: int = 200, period_nm: float = 3.33, gamma: float = 0.4) -> MultilayerMirror:
        """La/B for 6.6-6.7 nm (B on La, La fraction gamma, Si substrate)."""
        ...
    def with_capping(
        self, formula: str, thickness_nm: float, density_g_cm3: Optional[float] = None
    ) -> MultilayerMirror: ...
    def with_roughness(self, sigma_nm: float) -> MultilayerMirror: ...
    @property
    def period_nm(self) -> float: ...
    @property
    def periods(self) -> int: ...
    def reflectance(
        self, wavelength_nm: float, angle_deg: float = 0.0, polarization: str = "unpolarized"
    ) -> float: ...
    def reflectance_curve(
        self, wavelengths_nm: list[float] | np.ndarray, angle_deg: float = 0.0, polarization: str = "unpolarized"
    ) -> np.ndarray: ...
    def reflectance_vs_angle(
        self, wavelength_nm: float, angles_deg: list[float] | np.ndarray, polarization: str = "unpolarized"
    ) -> np.ndarray: ...
    def amplitude(self, wavelength_nm: float, angle_deg: float = 0.0, polarization: str = "te") -> complex: ...
    def peak(
        self, lambda_min_nm: float, lambda_max_nm: float, angle_deg: float = 0.0, polarization: str = "unpolarized"
    ) -> tuple[float, float]: ...
    def bandwidth_fwhm_nm(
        self, lambda_min_nm: float, lambda_max_nm: float, angle_deg: float = 0.0, polarization: str = "unpolarized"
    ) -> float: ...
    def angular_acceptance_fwhm_deg(self, wavelength_nm: float, polarization: str = "unpolarized") -> float: ...
    def pupil_response(self, wavelength_nm: float, angle_deg: float) -> dict[str, complex | float]: ...
    def __repr__(self) -> str: ...

def tune_multilayer_period(
    family: str,
    target_nm: float,
    periods: int,
    gamma: float = 0.4,
    angle_deg: float = 0.0,
    polarization: str = "te",
) -> float:
    """Period (nm) that puts a 'mo_si' / 'la_b4c' / 'la_b' mirror's peak at target_nm."""
    ...

def xray_optical_constants(
    formula: str, wavelengths_nm: list[float] | np.ndarray, density_g_cm3: Optional[float] = None
) -> tuple[np.ndarray, np.ndarray]:
    """Henke/CXRO (delta, beta) of a formula; n = 1 - delta + i beta."""

# --- Optimization / patterning (ILT, OPC, SRAF, SADP/SAQP) -------------------

class IltResult:
    """Inverse-lithography result: optimized mask, images, and optimizer history."""

    @property
    def mask(self) -> np.ndarray:
        """Optimized continuous mask transmission in (0, 1), shape (n, n)."""
        ...
    @property
    def aerial_image(self) -> np.ndarray: ...
    @property
    def binary_mask(self) -> np.ndarray: ...
    @property
    def binary_aerial_image(self) -> np.ndarray: ...
    @property
    def cost_history(self) -> np.ndarray:
        """Cost of the initial mask followed by the cost after each iteration."""
        ...
    @property
    def gradient_norm_history(self) -> np.ndarray: ...
    @property
    def stage_starts(self) -> list[int]: ...
    @property
    def iterations(self) -> int: ...
    @property
    def converged(self) -> bool: ...
    @property
    def termination(self) -> str:
        """'converged', 'max_iterations' or 'line_search_failed'."""
        ...
    @property
    def final_cost(self) -> float: ...
    @property
    def pattern_error(self) -> int: ...
    @property
    def binary_pattern_error(self) -> int: ...
    @property
    def snapshots(self) -> list[tuple[int, np.ndarray]]: ...
    def __repr__(self) -> str: ...
    @property
    def status(self) -> str:
        """Capability badge ("✅", "🔶", "🧪" or split) of the model behind this result."""
        ...
    @property
    def notes(self) -> list[str]:
        """The model's stated approximations."""
        ...

class OpcResult:
    """Fragment-based model OPC result."""

    @property
    def mask(self) -> MaskConfig: ...
    @property
    def polygons(self) -> list[list[tuple[float, float]]]: ...
    @property
    def converged(self) -> bool: ...
    @property
    def iterations(self) -> int: ...
    @property
    def epe_rms_history(self) -> np.ndarray: ...
    @property
    def epe_max_history(self) -> np.ndarray: ...
    @property
    def final_epe_rms_nm(self) -> float: ...
    @property
    def final_epe_max_nm(self) -> float: ...
    @property
    def final_image(self) -> np.ndarray: ...
    @property
    def fragment_biases(self) -> np.ndarray: ...
    @property
    def fragment_epes(self) -> np.ndarray: ...
    @property
    def fragment_kinds(self) -> list[str]: ...
    @property
    def fragment_points(self) -> list[tuple[float, float]]: ...
    def __repr__(self) -> str: ...
    @property
    def status(self) -> str:
        """Capability badge ("✅", "🔶", "🧪" or split) of the model behind this result."""
        ...
    @property
    def notes(self) -> list[str]:
        """The model's stated approximations."""
        ...

class SrafResult:
    """SRAF insertion result after the model-based print check."""

    @property
    def mask(self) -> MaskConfig: ...
    @property
    def assists(self) -> list[tuple[float, float, float, float]]:
        """Surviving assists as (x, y, w, h) rectangles in nm."""
        ...
    @property
    def placed(self) -> int: ...
    @property
    def removed(self) -> int: ...
    @property
    def shrink_steps(self) -> int: ...
    @property
    def worst_margin(self) -> float: ...
    def __repr__(self) -> str: ...
    @property
    def status(self) -> str:
        """Capability badge ("✅", "🔶", "🧪" or split) of the model behind this result."""
        ...
    @property
    def notes(self) -> list[str]:
        """The model's stated approximations."""
        ...

class DofComparison:
    """Depth of focus of a line with and without assists."""

    @property
    def dof_without_nm(self) -> float: ...
    @property
    def dof_with_nm(self) -> float: ...
    @property
    def gain(self) -> float: ...
    @property
    def focus_nm(self) -> np.ndarray: ...
    @property
    def cd_without_nm(self) -> np.ndarray: ...
    @property
    def cd_with_nm(self) -> np.ndarray: ...
    @property
    def thresholds(self) -> tuple[float, float]: ...
    @property
    def windows_nm(self) -> tuple[tuple[float, float], tuple[float, float]]: ...
    def __repr__(self) -> str: ...
    @property
    def status(self) -> str:
        """Capability badge ("✅", "🔶", "🧪" or split) of the model behind this result."""
        ...
    @property
    def notes(self) -> list[str]:
        """The model's stated approximations."""
        ...

class SpacerPatterningResult:
    """Geometric SADP/SAQP result over one mandrel period."""

    @property
    def lines(self) -> list[tuple[float, float]]: ...
    @property
    def period_nm(self) -> float: ...
    @property
    def line_cds_nm(self) -> list[float]: ...
    @property
    def spaces_nm(self) -> list[float]: ...
    @property
    def pitches_nm(self) -> list[float]: ...
    @property
    def nominal_pitch_nm(self) -> float: ...
    @property
    def pitch_walk_nm(self) -> float: ...
    @property
    def cd_range_nm(self) -> float: ...
    def __repr__(self) -> str: ...
    @property
    def status(self) -> str:
        """Capability badge ("✅", "🔶", "🧪" or split) of the model behind this result."""
        ...
    @property
    def notes(self) -> list[str]:
        """The model's stated approximations."""
        ...

def mask_from_features(
    rects: Optional[list[tuple[float, float, float, float]]] = None,
    polygons: Optional[list[list[tuple[float, float]]]] = None,
    dark_field: bool = False,
    attenuated_transmission: Optional[float] = None,
    attenuated_phase_deg: float = 180.0,
) -> MaskConfig:
    """DEPRECATED (DeprecationWarning): use ``MaskConfig.from_features``, which
    takes the same (x, y, w, h) rectangles and vertex-list polygons."""
    ...

def ilt_contact_target(
    size_nm: float,
    pitch_x_nm: float,
    pitch_y_nm: float,
    count_x: int,
    count_y: int,
    grid: GridConfig,
    shape: str = "round",
) -> np.ndarray:
    """ILT target: centred array of holes (1 inside), area-averaged edges."""
    ...

def optimize_ilt(
    source: SourceConfig,
    optics: OpticsConfig,
    grid: GridConfig,
    target: np.ndarray,
    cost: str = "image",
    threshold: float = 0.3,
    steepness: float = 50.0,
    gradient: str = "adjoint",
    optimizer: str = "cg",
    max_iterations: int = 50,
    learning_rate: float = 0.5,
    tv_weight: float = 0.01,
    binarization_weight: float = 0.0,
    binarization_after: int = 0,
    min_feature_nm: float = 0.0,
    sigmoid_steepness: float = 4.0,
    conditions: Optional[list[tuple[float, float, float]]] = None,
    absorber_transmission: float = 0.0,
    absorber_phase_deg: float = 180.0,
    init: str = "target",
    init_level: float = 0.5,
    initial_mask: Optional[np.ndarray] = None,
    snapshot_every: int = 0,
    convergence_tol: float = 1e-4,
    max_kernels: int = 16,
    illumination: Optional[IlluminationSpec] = None,
) -> IltResult:
    """Inverse lithography with the exact adjoint gradient (or the legacy proxy)."""
    ...

def fragment_opc(
    source: SourceConfig,
    optics: OpticsConfig,
    mask: MaskConfig,
    grid: GridConfig,
    threshold: float,
    max_iterations: Optional[int] = None,
    tolerance_nm: Optional[float] = None,
    max_epe_tolerance_nm: Optional[float] = None,
    feedback_gain: Optional[float] = None,
    smoothing: Optional[float] = None,
    max_bias_nm: Optional[float] = None,
    max_fragment_nm: Optional[float] = None,
    corner_fragment_nm: Optional[float] = None,
    min_jog_nm: Optional[float] = None,
    bias_grid_nm: float = 0.0,
    correct_corners: bool = False,
    conditions: Optional[list[tuple[float, float, float]]] = None,
    max_kernels: int = 16,
    illumination: Optional[IlluminationSpec] = None,
) -> OpcResult:
    """Fragment-based model OPC (per-fragment EPE feedback)."""
    ...

def insert_srafs(
    source: SourceConfig,
    optics: OpticsConfig,
    mask: MaskConfig,
    grid: GridConfig,
    line_cd_nm: float,
    sigma_center: float,
    threshold: float,
    defocus_nm: float = 150.0,
    dose_excursion: float = 0.08,
    margin: float = 0.05,
    max_kernels: int = 16,
    illumination: Optional[IlluminationSpec] = None,
) -> SrafResult:
    """Rule-based SRAF placement plus model-based print check."""
    ...

def dose_to_size_threshold(
    source: SourceConfig,
    optics: OpticsConfig,
    mask: MaskConfig,
    grid: GridConfig,
    target_cd_nm: float,
    x_center_nm: float = 0.0,
    y_nm: float = 0.0,
    half_width_nm: Optional[float] = None,
    dark_line: bool = True,
    max_kernels: int = 16,
    illumination: Optional[IlluminationSpec] = None,
) -> float:
    """Threshold at which the line prints at target_cd_nm in focus."""
    ...

def compare_sraf_dof(
    source: SourceConfig,
    optics: OpticsConfig,
    mask_without: MaskConfig,
    mask_with: MaskConfig,
    grid: GridConfig,
    target_cd_nm: float,
    x_center_nm: float = 0.0,
    y_nm: float = 0.0,
    half_width_nm: Optional[float] = None,
    dark_line: bool = True,
    cd_tolerance_pct: float = 10.0,
    focus_range_nm: float = 300.0,
    focus_steps: int = 13,
    exposure_latitude_pct: float = 0.0,
    max_kernels: int = 16,
    illumination: Optional[IlluminationSpec] = None,
) -> DofComparison:
    """Depth of focus of a line with and without assists."""
    ...

def sadp(
    mandrel_pitch_nm: float,
    mandrel_cd_nm: float,
    spacer_nm: float,
    tone: str = "spacer_is_line",
) -> SpacerPatterningResult:
    """Geometric self-aligned double patterning."""
    ...

def saqp(
    mandrel_pitch_nm: float,
    mandrel_cd_nm: float,
    spacer1_nm: float,
    spacer2_nm: float,
    tone: str = "spacer_is_line",
) -> SpacerPatterningResult:
    """Geometric self-aligned quadruple patterning."""
# --- Talbot / EUV interference lithography ---------------------------------

class TalbotResult:
    """Talbot carpet, coherent/DTL/ATL wafer images, and characteristic lengths."""

    @property
    def carpet(self) -> np.ndarray:
        """Coherent Talbot carpet I(x, z) along y = 0, shape (nz, nx)."""
        ...
    @property
    def carpet_x_nm(self) -> np.ndarray: ...
    @property
    def carpet_z_nm(self) -> np.ndarray: ...
    @property
    def x_nm(self) -> np.ndarray: ...
    @property
    def y_nm(self) -> np.ndarray: ...
    @property
    def coherent_image(self) -> np.ndarray:
        """Coherent intensity at the gap, shape (ny, nx)."""
        ...
    @property
    def stationary_image(self) -> np.ndarray:
        """Infinite-scan DTL (stationary) image, shape (ny, nx)."""
        ...
    @property
    def dtl_image(self) -> np.ndarray:
        """DTL image averaged over [gap, gap + scan_length], shape (ny, nx)."""
        ...
    @property
    def atl_image(self) -> Optional[np.ndarray]:
        """Spectrally averaged (ATL) image, or None for monochromatic runs."""
        ...
    @property
    def talbot_length_nm(self) -> float: ...
    @property
    def talbot_length_exact_nm(self) -> Optional[float]: ...
    @property
    def achromatic_distance_nm(self) -> Optional[float]: ...
    @property
    def period_nm(self) -> float: ...
    @property
    def wavelength_nm(self) -> float: ...
    @property
    def gap_nm(self) -> float: ...
    @property
    def scan_length_nm(self) -> float: ...
    @property
    def efficiencies(self) -> list[tuple[int, int, float]]: ...
    @property
    def intensity_volume(self) -> Optional[VolumetricResult]: ...
    @property
    def pac_volume(self) -> Optional[VolumetricResult]: ...
    def __repr__(self) -> str: ...
    @property
    def status(self) -> str:
        """Capability badge ("✅", "🔶", "🧪" or split) of the model behind this result."""
        ...
    @property
    def notes(self) -> list[str]:
        """The model's stated approximations."""
        ...

class EuvIlResult:
    """Two-grating EUV interference-lithography fringes (period p/(2m))."""

    @property
    def fringe_period_nm(self) -> float: ...
    @property
    def diffraction_angle_deg(self) -> float: ...
    @property
    def visibility(self) -> float: ...
    @property
    def beam_intensities(self) -> tuple[float, float]: ...
    @property
    def intensity(self) -> VolumetricResult: ...
    @property
    def pac(self) -> VolumetricResult: ...
    def __repr__(self) -> str: ...
    @property
    def status(self) -> str:
        """Capability badge ("✅", "🔶", "🧪" or split) of the model behind this result."""
        ...
    @property
    def notes(self) -> list[str]:
        """The model's stated approximations."""
        ...

def simulate_talbot(
    wavelength_nm: float = 13.5,
    period_nm: float = 100.0,
    grating: str = "amplitude",
    duty_cycle: float = 0.5,
    phase_rad: Optional[float] = None,
    hole_diameter_nm: Optional[float] = None,
    max_order: int = 10,
    propagation: str = "exact",
    gap_nm: Optional[float] = None,
    n_periods: int = 2,
    nx: int = 128,
    ny: Optional[int] = None,
    carpet_z_max_nm: Optional[float] = None,
    carpet_nz: int = 128,
    scan_length_nm: Optional[float] = None,
    bandwidth_nm: float = 0.0,
    spectrum: str = "gaussian",
    spectral_bins: int = 256,
    resist_thickness_nm: float = 0.0,
    resist_index: float = 1.0,
    absorption_per_nm: float = 0.0,
    nz: int = 16,
    exposure: str = "dtl",
    dose_scale: float = 1.0,
    dill_c: float = 0.0,
) -> TalbotResult:
    """Talbot / DTL / ATL lithography of a transmission grating."""
    ...

def simulate_euv_il(
    grating_period_nm: float = 100.0,
    wavelength_nm: float = 13.5,
    order: int = 1,
    grating: str = "amplitude",
    duty_cycle: float = 0.5,
    phase_rad: Optional[float] = None,
    intensity_ratio: float = 1.0,
    n_medium: float = 1.0,
    absorption_per_nm: float = 0.0,
    n_fringes: int = 8,
    nx: int = 128,
    ny: int = 4,
    nz: int = 16,
    z_span_nm: float = 50.0,
    dose_scale: float = 1.0,
    dill_c: float = 0.02,
) -> EuvIlResult:
    """Two-grating EUV interference lithography (fringe period p/(2m))."""
    ...

# --- LELE double patterning and directed self-assembly ----------------------

class LeleResult:
    """LELE double patterning: two exposures of complementary 2x-pitch gratings."""

    @property
    def aerial1(self) -> np.ndarray: ...
    @property
    def aerial2(self) -> np.ndarray: ...
    @property
    def aerial2_overlay(self) -> np.ndarray:
        """Exposure-2 image shifted by the overlay error (Fourier shift)."""
        ...
    @property
    def combined_aerial(self) -> np.ndarray:
        """Double-exposure composite (d1 I1 + d2 I2')/(d1 + d2); NOT the LELE result."""
        ...
    @property
    def combined_contrast(self) -> float: ...
    @property
    def doses_mj_cm2(self) -> tuple[float, float]: ...
    @property
    def x_nm(self) -> np.ndarray: ...
    @property
    def masks(self) -> tuple[MaskConfig, MaskConfig]: ...
    def printed_pattern(self, threshold_mj_cm2: float) -> np.ndarray:
        """1 where a resist line remains (d1 I1 < E_th or d2 I2' < E_th)."""
        ...
    def lines(self, threshold_mj_cm2: float, row: Optional[int] = None) -> list[tuple[float, float]]:
        """Printed line intervals (x0_nm, x1_nm) along an image row (default: centre)."""
        ...
    def __repr__(self) -> str: ...
    @property
    def status(self) -> str:
        """Capability badge ("✅", "🔶", "🧪" or split) of the model behind this result."""
        ...
    @property
    def notes(self) -> list[str]:
        """The model's stated approximations."""
        ...

def simulate_lele(
    source: SourceConfig,
    optics: OpticsConfig,
    grid: GridConfig,
    cd_nm: float,
    pitch_nm: float,
    overlay_x_nm: float = 0.0,
    overlay_y_nm: float = 0.0,
    dose1_mj_cm2: float = 30.0,
    dose2_mj_cm2: float = 30.0,
    focus1_nm: float = 0.0,
    focus2_nm: float = 0.0,
    max_kernels: int = 16,
    illumination: Optional[IlluminationSpec] = None,
) -> LeleResult:
    """LELE of cd_nm lines at pitch_nm (grid commensurate with 2*pitch_nm)."""
    ...

class DsaResult:
    """Analytic directed self-assembly result (not SCFT)."""

    @property
    def pattern(self) -> np.ndarray:
        """(ny, nx) A-block fraction (1 = A, 0 = B or guiding wall)."""
        ...
    @property
    def heuristic_defect_index(self) -> float:
        """ILLUSTRATIVE index 100*min(10*max|W/L0 - n|, 1); NaN if not evaluated."""
        ...
    @property
    def is_defect_free(self) -> bool: ...
    @property
    def assembled_cd_nm(self) -> float: ...
    def commensurability(self) -> list[dict[str, float]]:
        """Per trench: confinement_nm, periods, period_nm, strain, free_energy_ratio."""
        ...
    def __repr__(self) -> str: ...
    @property
    def status(self) -> str:
        """Capability badge ("✅", "🔶", "🧪" or split) of the model behind this result."""
        ...
    @property
    def notes(self) -> list[str]:
        """The model's stated approximations."""
        ...

def simulate_dsa(
    template: np.ndarray,
    pixel_nm: float,
    l0_nm: float = 28.0,
    chi_n: float = 20.0,
    volume_fraction: float = 0.5,
    morphology: str = "lamellar",
    interface_width_nm: Optional[float] = None,
) -> DsaResult:
    """Analytic DSA on a template (> 0.5 = open trench): lamellar / cylindrical / spherical."""
    ...

def dsa_confined_lamellae(trench_nm: float, l0_nm: float) -> dict[str, float]:
    """Free-energy-optimal lamellae count in a trench (strong segregation)."""
    ...

# --- Stochastic LER/LWR and photon counting -------------------------------

class LerResult:
    """Monte-Carlo shot-noise LER/LWR of the feature at the field centre."""

    @property
    def ler_3sigma_nm(self) -> float: ...
    @property
    def lwr_3sigma_nm(self) -> float: ...
    @property
    def cd_mean_nm(self) -> float: ...
    @property
    def cd_sigma_nm(self) -> float: ...
    @property
    def left_edges_nm(self) -> np.ndarray: ...
    @property
    def right_edges_nm(self) -> np.ndarray: ...
    @property
    def measured_realizations(self) -> int: ...
    @property
    def num_realizations(self) -> int: ...
    @property
    def photon_density_per_mj_cm2(self) -> float: ...
    @property
    def dose_jitter_rms(self) -> float: ...
    def __repr__(self) -> str: ...
    @property
    def status(self) -> str:
        """Capability badge ("✅", "🔶", "🧪" or split) of the model behind this result."""
        ...
    @property
    def notes(self) -> list[str]:
        """The model's stated approximations."""
        ...

def compute_ler_lwr(
    aerial: AerialImageResult,
    dose_mj_cm2: float,
    threshold: float = 0.3,
    source: Optional[SourceConfig] = None,
    photon_density_per_mj_cm2: Optional[float] = None,
    dose_jitter_rms: Optional[float] = None,
    pulses_per_exposure: Optional[float] = None,
    absorbed_fraction: Optional[float] = None,
    num_realizations: int = 100,
) -> LerResult:
    """Poisson shot-noise LER/LWR (🔶); give a source or photon_density_per_mj_cm2."""
    ...

def resist_absorbed_fraction(thickness_nm: float, absorption_per_um: float) -> float:
    """Beer-Lambert absorbed fraction 1 - exp(-alpha d) (alpha in 1/um, d in nm)."""
    ...

def photons_per_square(dose_mj_cm2: float, wavelength_nm: float, side_nm: float) -> float:
    """Mean photons into a side_nm square: D a^2 / (h c / lambda)."""
    ...

def relative_shot_noise(dose_mj_cm2: float, wavelength_nm: float, side_nm: float) -> float:
    """1/sqrt(N) for N = photons_per_square(...)."""
    ...

# --- Materials database ----------------------------------------------------

def refractive_index(material: str, wavelength_nm: float) -> complex:
    """n + ik of a named material ("CaF2", "Si", "Mo_euv", "henke:<formula>[@rho]", ...).

    Never extrapolates: each entry has a data range (VUV tables 125-161 nm incl. a
    1 nm end hold, Sellmeier fits e.g. CaF2 125-2000 nm, Henke 0.0413-41.3 nm).
    "Si", "Cr" and "SiO2" fall back to the Henke value (same as "henke:Si") in
    0.0413-41.3 nm. Raises ValueError naming the range and the alternative key
    otherwise.
    """
    ...

def material_names() -> list[str]:
    """Built-in material names (sorted)."""
    ...

# --- Quantum (N-photon) research module (🧪 Theoretical) --------------------

def n_photon_absorption_image(classical: np.ndarray, num_photons: int = 2) -> np.ndarray:
    """Classical N-photon absorption I^N of a classical image (sharpening at the
    classical period; no resolution gain)."""
    ...

def quantum_flux_budget(wavelength_nm: float = 157.63, num_photons: int = 2) -> dict[str, float]:
    """Entangled N-photon flux / exposure-time budget vs an HVM classical exposure.

    Keys: num_photons, photon_energy_ev, hvm_photon_rate_per_s,
    entangled_rate_ceiling_per_s, relative_flux, exposure_time_ratio_bound
    (a LOWER bound), exposure_time_ratio_tightest_bound,
    exposure_time_ratio_claimed (optimistic).
    """
    ...

class TwoBeamNPhoton:
    """Two-beam N-photon exposure in closed form (🧪): ideal N00N fringe
    1 + cos(N(Kx + phi)) (period lambda/(2N sin theta)), classical N-photon fringe
    (1 + cos(Kx + phi))^N / m_N (period lambda/(2 sin theta)), and the mixture
    F*E_NOON + (1-F)*E_cl, all unit mean. Positions in nm."""

    def __init__(
        self,
        wavelength_nm: float,
        half_angle_deg: float,
        num_photons: int = 2,
        fidelity: float = 1.0,
        phase_rad: float = 0.0,
    ) -> None: ...
    @property
    def wavelength_nm(self) -> float: ...
    @property
    def half_angle_deg(self) -> float: ...
    @property
    def num_photons(self) -> int: ...
    @property
    def fidelity(self) -> float: ...
    @property
    def phase_rad(self) -> float: ...
    def fringe_wavenumber_per_nm(self) -> float:
        """K = 4 pi sin(theta) / lambda (rad/nm)."""
        ...
    def classical_period_nm(self) -> float:
        """lambda / (2 sin theta) (nm)."""
        ...
    def noon_period_nm(self) -> float:
        """lambda / (2 N sin theta) (nm)."""
        ...
    def noon_exposure(self, x_nm: float) -> float: ...
    def classical_exposure(self, x_nm: float) -> float: ...
    def exposure(self, x_nm: float) -> float: ...
    def profile(self, x_nm: np.ndarray) -> np.ndarray:
        """Mixed exposure at every position of a 1D float64 array (nm)."""
        ...
    def harmonic_amplitude(self, j: int) -> float:
        """Exact cosine amplitude of the j-th harmonic of the mixed exposure."""
        ...
    def flux_budget(self) -> dict[str, float]:
        """Flux / exposure-time budget at this wavelength and N."""
        ...
    @property
    def status(self) -> str: ...
    @property
    def notes(self) -> list[str]: ...
    def __repr__(self) -> str: ...

class NoonImageResult:
    """Result of SimulationEngine.compute_noon_ideal_image (🧪); every image is a
    clear-field normalized AerialImageResult."""

    @property
    def classical(self) -> AerialImageResult: ...
    @property
    def n_photon_absorption(self) -> AerialImageResult: ...
    @property
    def noon_limit(self) -> AerialImageResult: ...
    @property
    def exposure(self) -> AerialImageResult: ...
    @property
    def num_photons(self) -> int: ...
    @property
    def fidelity(self) -> float: ...
    @property
    def wavelength_nm(self) -> float: ...
    @property
    def effective_wavelength_nm(self) -> float: ...
    @property
    def classical_resolution_nm(self) -> float: ...
    @property
    def noon_limit_resolution_nm(self) -> float: ...
    @property
    def flux(self) -> dict[str, float]: ...
    @property
    def status(self) -> str: ...
    @property
    def notes(self) -> list[str]: ...
    def __repr__(self) -> str: ...
