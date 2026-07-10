"""Type stubs for the highuvlith._native extension module."""

from typing import Optional

import numpy as np

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
        """Create Ar2 excimer (126nm) source."""
        ...
    @staticmethod
    def lpa_fel_bella_25nm(sigma: float = 0.7) -> SourceConfig:
        """Create an LPA-FEL source with the BELLA 500 MeV target config (~25 nm)."""
        ...
    @staticmethod
    def lpa_fel(
        wavelength_nm: float,
        sigma: float = 0.7,
        electron_energy_mev: float = 500.0,
        bandwidth_pm: float = 25.0,
        pulse_duration_fs: float = 10.0,
        rep_rate_hz: float = 1000.0,
    ) -> SourceConfig:
        """Create a custom LPA-FEL source at the given wavelength (nm)."""
        ...
    @staticmethod
    def lpp_sn_13nm5(sigma: float = 0.9) -> SourceConfig:
        """Sn laser-produced-plasma source at 13.5 nm (NXE-class)."""
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
    ) -> SourceConfig:
        """Synchrotron undulator; wavelength derived from the resonance condition."""
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
    ) -> SourceConfig:
        """High-harmonic generation source (odd harmonics, cutoff-checked)."""
        ...
    @staticmethod
    def hhg_ne_13nm5() -> SourceConfig:
        """Neon HHG preset reaching 13.56 nm at harmonic 59."""
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
    def ics(
        target_wavelength_nm: float = 13.5,
        laser_wavelength_nm: float = 1030.0,
        laser_a0: float = 0.1,
    ) -> SourceConfig:
        """Inverse-Compton source tuned to the target wavelength (theoretical)."""
        ...
    @staticmethod
    def ssmb_euv_13nm5() -> SourceConfig:
        """Steady-state-microbunching EUV design point (theoretical)."""
        ...
    @staticmethod
    def entangled_noon(
        wavelength_nm: float = 157.63, n: int = 2, fidelity: float = 1.0
    ) -> SourceConfig:
        """N-photon entangled NOON-state source (entirely theoretical)."""
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
    def __repr__(self) -> str: ...

class OpticsConfig:
    """Optics configuration (refractive, Schwarzschild, or zone plate)."""

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
    def add_aberration(self, fringe_index: int, coefficient_waves: float) -> None:
        """Add a Zernike aberration coefficient (refractive only)."""
        ...
    @property
    def kind(self) -> str: ...
    @property
    def numerical_aperture(self) -> float: ...
    @property
    def reduction(self) -> float: ...
    @property
    def flare_fraction(self) -> float: ...
    def rayleigh_resolution(self, wavelength_nm: float) -> float: ...
    def __repr__(self) -> str: ...

class MaskConfig:
    """Mask configuration."""

    @staticmethod
    def line_space(cd_nm: float, pitch_nm: float) -> MaskConfig:
        """Create a line/space pattern."""
        ...
    @staticmethod
    def contact_hole(
        diameter_nm: float, pitch_x_nm: float, pitch_y_nm: float
    ) -> MaskConfig:
        """Create a contact hole array."""
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
    """Film stack configuration."""

    def __init__(self) -> None: ...
    def add_layer(
        self, name: str, thickness_nm: float, n_real: float, n_imag: float
    ) -> None:
        """Add a layer to the stack."""
        ...
    def set_substrate(self, n_real: float, n_imag: float) -> None:
        """Set the substrate refractive index."""
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
    @property
    def size(self) -> int: ...
    @property
    def pixel_nm(self) -> float: ...
    def field_size_nm(self) -> float: ...
    def __repr__(self) -> str: ...

class AerialImageResult:
    """Aerial image simulation result."""

    @property
    def intensity(self) -> np.ndarray: ...
    @property
    def x_nm(self) -> np.ndarray: ...
    @property
    def y_nm(self) -> np.ndarray: ...
    def cross_section(
        self, y_nm: float = 0.0
    ) -> tuple[np.ndarray, np.ndarray]:
        """Extract a 1D cross-section along x at y=y_nm."""
        ...
    def image_contrast(self) -> float:
        """Compute image contrast: (Imax - Imin) / (Imax + Imin)."""
        ...
    def nils(self, threshold: float = 0.3) -> Optional[float]:
        """Compute NILS at y=0 cross-section."""
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
    ) -> None: ...
    def compute_aerial_image(
        self, focus_nm: float = 0.0
    ) -> AerialImageResult:
        """Compute aerial image at the given defocus."""
        ...
    def compute_polychromatic(
        self, focus_nm: float = 0.0
    ) -> AerialImageResult:
        """Compute polychromatic aerial image (accounts for VUV chromatic aberration)."""
        ...
    def measure_cd(
        self,
        dose_mj_cm2: float = 30.0,
        focus_nm: float = 0.0,
        threshold: float = 0.3,
    ) -> float:
        """Measure CD at given dose and focus."""
        ...
    def compute_resist_profile(
        self,
        dose_mj_cm2: float = 30.0,
        focus_nm: float = 0.0,
        dev_time_s: float = 60.0,
    ) -> ResistProfileResult:
        """Compute resist profile at given dose and focus."""
        ...
    def image_contrast(self, focus_nm: float = 0.0) -> float:
        """Get image contrast at given focus."""
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
    def resist(self) -> Optional[ResistConfig]: ...
    @property
    def grid(self) -> GridConfig: ...
    def __repr__(self) -> str: ...

class BatchSimulator:
    """Batch simulation with GIL release for parameter sweeps."""

    def __init__(
        self,
        source: SourceConfig,
        optics: OpticsConfig,
        mask: MaskConfig,
        grid: Optional[GridConfig] = None,
        max_kernels: int = 30,
    ) -> None: ...
    def process_window(
        self,
        doses: list[float],
        focuses: list[float],
        cd_threshold: float = 0.3,
        cd_target_nm: float = 65.0,
        cd_tolerance_pct: float = 10.0,
    ) -> ProcessWindowResult:
        """Compute process window: CD vs dose and focus."""
        ...
    def batch_defocus(
        self,
        focuses: list[float],
    ) -> list[tuple[float, np.ndarray]]:
        """Batch compute aerial images at multiple focus values."""
        ...

class ProcessWindowResult:
    """Process window analysis result."""

    @property
    def cd_matrix(self) -> np.ndarray: ...
    @property
    def doses(self) -> np.ndarray: ...
    @property
    def focuses(self) -> np.ndarray: ...
    def depth_of_focus(self) -> float:
        """Depth of focus in nm."""
        ...
    def exposure_latitude(self) -> float:
        """Exposure latitude in percent."""
        ...
    def __repr__(self) -> str: ...

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
    def peak_positions(self) -> list[tuple[float, float]]: ...
    @property
    def total_emission_power(self) -> float: ...
    def coordinates(self) -> tuple[np.ndarray, np.ndarray]: ...
    def cross_section_x(self, y_nm: float) -> tuple[np.ndarray, np.ndarray]: ...
    def cross_section_y(self, x_nm: float) -> tuple[np.ndarray, np.ndarray]: ...

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

    @property
    def values(self) -> np.ndarray:
        """The field values as a (nz, ny, nx) numpy array."""
        ...
    @property
    def x_nm(self) -> np.ndarray: ...
    @property
    def y_nm(self) -> np.ndarray: ...
    @property
    def z_nm(self) -> np.ndarray: ...
    @property
    def shape(self) -> tuple[int, int, int]: ...
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
        """The map values as a (ny, nx) numpy array."""
        ...
    @property
    def x_nm(self) -> np.ndarray: ...
    @property
    def y_nm(self) -> np.ndarray: ...
    @property
    def shape(self) -> tuple[int, int]: ...
    def __repr__(self) -> str: ...

class LigaResult:
    """LIGA deep-X-ray exposure result: depth dose, contrast, and development."""

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
    def volume(self) -> VolumetricResult: ...
    @property
    def developed_depth(self) -> HeightMapResult: ...
    def __repr__(self) -> str: ...

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
) -> VolumetricResult:
    """Fast-marching development of a PAC volume; returns arrival times (s)."""
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
) -> LigaResult:
    """Simulate a LIGA deep-X-ray shadow exposure of thick PMMA."""
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
    """Quantum N-photon aerial image from a classical intensity image."""
    ...
