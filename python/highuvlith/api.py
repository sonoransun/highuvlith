"""High-level convenience API for common simulation workflows."""

from __future__ import annotations

from dataclasses import dataclass
from typing import Any

import numpy as np

import highuvlith as huv
from highuvlith._native import (
    HeightMapResult,
    LigaResult,
    VolumetricResult,
    grayscale_height_map,
    grayscale_transmittance_for_target,
    quantum_aerial_image,
)

# Re-exported through the high-level API namespace (thin native pass-through).
from highuvlith._native import expose_volumetric as expose_volumetric
from highuvlith._native import simulate_interference as _native_simulate_interference
from highuvlith._native import simulate_liga as _native_simulate_liga


@dataclass
class FullResult:
    """Aggregated result from a single simulation run."""

    aerial: huv.AerialImageResult
    contrast: float
    config: dict[str, Any]
    resist_profile: huv.ResistProfileResult | None = None
    cd_nm: float | None = None
    nils: float | None = None


def simulate_line_space(
    cd_nm: float,
    pitch_nm: float,
    *,
    wavelength_nm: float = 157.63,
    na: float = 0.75,
    sigma: float = 0.7,
    focus_nm: float = 0.0,
    dose_mj_cm2: float = 30.0,
    grid_size: int = 256,
    pixel_nm: float = 1.0,
    with_resist: bool = False,
    max_kernels: int = 20,
) -> FullResult:
    """One-liner simulation of a line/space pattern.

    Args:
        cd_nm: Line width at wafer scale (nm).
        pitch_nm: Line+space pitch (nm).
        wavelength_nm: Source wavelength (nm). Default 157.63 (F2 laser).
        na: Numerical aperture. Default 0.75.
        sigma: Partial coherence factor. Default 0.7.
        focus_nm: Defocus (nm). Default 0 (best focus).
        dose_mj_cm2: Exposure dose (mJ/cm²). Default 30.
        grid_size: Simulation grid size (must be power of 2). Default 256.
        pixel_nm: Pixel size (nm). Default 1.0.
        with_resist: If True, also compute resist profile.
        max_kernels: Max SOCS kernels for TCC decomposition. Default 20.

    Returns:
        FullResult with aerial image, contrast, and optionally resist profile.
    """
    from highuvlith._validation import (
        _validate_positive,
        _validate_power_of_two,
        _validate_range,
    )

    _validate_positive("cd_nm", cd_nm)
    _validate_positive("pitch_nm", pitch_nm)
    if cd_nm >= pitch_nm:
        raise ValueError(f"cd_nm ({cd_nm}) must be less than pitch_nm ({pitch_nm})")
    _validate_range("na", na, 0.01, 1.0)
    _validate_range("sigma", sigma, 0.01, 1.0)
    _validate_power_of_two("grid_size", grid_size)
    _validate_positive("pixel_nm", pixel_nm)

    source = huv.SourceConfig(
        wavelength_nm=wavelength_nm,
        sigma_outer=sigma,
    )
    optics = huv.OpticsConfig(numerical_aperture=na)
    mask = huv.MaskConfig.line_space(cd_nm=cd_nm, pitch_nm=pitch_nm)
    resist = huv.ResistConfig.vuv_fluoropolymer()
    grid = huv.GridConfig(size=grid_size, pixel_nm=pixel_nm)

    engine = huv.SimulationEngine(
        source, optics, mask, resist, grid, max_kernels=max_kernels
    )

    aerial = engine.compute_aerial_image(focus_nm=focus_nm)
    contrast = aerial.image_contrast()

    config = {
        "wavelength_nm": wavelength_nm,
        "na": na,
        "sigma": sigma,
        "cd_nm": cd_nm,
        "pitch_nm": pitch_nm,
        "focus_nm": focus_nm,
        "dose_mj_cm2": dose_mj_cm2,
        "grid_size": grid_size,
        "pixel_nm": pixel_nm,
    }

    result = FullResult(aerial=aerial, contrast=contrast, config=config)

    result.nils = aerial.nils(threshold=0.3)

    if with_resist:
        result.resist_profile = engine.compute_resist_profile(
            dose_mj_cm2=dose_mj_cm2, focus_nm=focus_nm
        )

    return result


def simulate_contact_hole(
    diameter_nm: float,
    pitch_x_nm: float,
    pitch_y_nm: float | None = None,
    *,
    wavelength_nm: float = 157.63,
    na: float = 0.75,
    sigma: float = 0.7,
    focus_nm: float = 0.0,
    grid_size: int = 256,
    pixel_nm: float = 1.0,
    max_kernels: int = 20,
) -> FullResult:
    """Simulate a contact hole array pattern.

    Args:
        diameter_nm: Hole diameter at wafer (nm).
        pitch_x_nm: X-direction pitch (nm).
        pitch_y_nm: Y-direction pitch (nm). Defaults to pitch_x_nm.
        wavelength_nm: Source wavelength (nm).
        na: Numerical aperture.
        sigma: Partial coherence factor.
        focus_nm: Defocus (nm).
        grid_size: Simulation grid size.
        pixel_nm: Pixel size (nm).
        max_kernels: Max SOCS kernels.

    Returns:
        FullResult with aerial image and contrast.
    """
    from highuvlith._validation import (
        _validate_positive,
        _validate_power_of_two,
        _validate_range,
    )

    _validate_positive("diameter_nm", diameter_nm)
    _validate_positive("pitch_x_nm", pitch_x_nm)
    if pitch_y_nm is not None:
        _validate_positive("pitch_y_nm", pitch_y_nm)
    _validate_range("na", na, 0.01, 1.0)
    _validate_range("sigma", sigma, 0.01, 1.0)
    _validate_power_of_two("grid_size", grid_size)

    if pitch_y_nm is None:
        pitch_y_nm = pitch_x_nm

    source = huv.SourceConfig(wavelength_nm=wavelength_nm, sigma_outer=sigma)
    optics = huv.OpticsConfig(numerical_aperture=na)
    mask = huv.MaskConfig.contact_hole(diameter_nm, pitch_x_nm, pitch_y_nm)
    grid = huv.GridConfig(size=grid_size, pixel_nm=pixel_nm)

    engine = huv.SimulationEngine(source, optics, mask, grid=grid, max_kernels=max_kernels)
    aerial = engine.compute_aerial_image(focus_nm=focus_nm)

    config = {
        "wavelength_nm": wavelength_nm,
        "na": na,
        "sigma": sigma,
        "diameter_nm": diameter_nm,
        "pitch_x_nm": pitch_x_nm,
        "pitch_y_nm": pitch_y_nm,
        "focus_nm": focus_nm,
        "grid_size": grid_size,
        "pixel_nm": pixel_nm,
    }

    return FullResult(
        aerial=aerial,
        contrast=aerial.image_contrast(),
        config=config,
    )


def sweep_focus(
    cd_nm: float = 65.0,
    pitch_nm: float = 180.0,
    *,
    wavelength_nm: float = 157.63,
    na: float = 0.75,
    sigma: float = 0.7,
    focus_min: float = -300.0,
    focus_max: float = 300.0,
    focus_steps: int = 21,
    grid_size: int = 128,
    pixel_nm: float = 2.0,
) -> dict[str, Any]:
    """Sweep focus and return contrast vs focus data.

    Returns:
        Dict with 'focuses', 'contrasts' arrays and 'best_focus_nm'.
    """
    from highuvlith._validation import (
        _validate_positive,
        _validate_power_of_two,
        _validate_range,
    )

    if focus_min >= focus_max:
        raise ValueError(
            f"focus_min ({focus_min}) must be less than focus_max ({focus_max})"
        )
    if focus_steps < 2:
        raise ValueError(f"focus_steps must be >= 2, got {focus_steps}")
    _validate_positive("cd_nm", cd_nm)
    _validate_positive("pitch_nm", pitch_nm)
    _validate_range("na", na, 0.01, 1.0)
    _validate_range("sigma", sigma, 0.01, 1.0)
    _validate_power_of_two("grid_size", grid_size)

    source = huv.SourceConfig(wavelength_nm=wavelength_nm, sigma_outer=sigma)
    optics = huv.OpticsConfig(numerical_aperture=na)
    mask = huv.MaskConfig.line_space(cd_nm=cd_nm, pitch_nm=pitch_nm)
    grid = huv.GridConfig(size=grid_size, pixel_nm=pixel_nm)

    batch = huv.BatchSimulator(source, optics, mask, grid)
    focuses = np.linspace(focus_min, focus_max, focus_steps).tolist()
    results = batch.batch_defocus(focuses=focuses)

    contrasts = []
    for _f, img in results:
        arr = np.asarray(img)
        i_max = arr.max()
        i_min = arr.min()
        contrasts.append((i_max - i_min) / (i_max + i_min) if (i_max + i_min) > 0 else 0.0)

    contrasts = np.array(contrasts)
    best_idx = np.argmax(contrasts)

    return {
        "focuses": np.array(focuses),
        "contrasts": contrasts,
        "best_focus_nm": focuses[best_idx],
        "best_contrast": contrasts[best_idx],
    }


def simulate_liga(
    critical_energy_kev: float = 6.23,
    resist_thickness_um: float = 500.0,
    *,
    cd_nm: float = 5000.0,
    pitch_nm: float = 10000.0,
    grid_size: int = 128,
    pixel_nm: float = 200.0,
    nz: int = 64,
) -> LigaResult:
    """Simulate a LIGA deep-X-ray shadow exposure of thick PMMA.

    Uses a bending-magnet white beam and the standard thick-PMMA preset
    (20 um Au absorber on a 2 um Ti membrane, 100 um proximity gap, 3 kJ/cm^3
    bottom clearing dose) over a line/space mask. Deep X-rays penetrate
    hundreds of um of resist and print near-geometric shadows, enabling very
    high aspect ratios.

    Args:
        critical_energy_kev: Bending-magnet critical energy (keV). Default 6.23
            (LIGA-class 2.5 GeV / 1.5 T beamline).
        resist_thickness_um: PMMA thickness (um). Default 500.
        cd_nm: Absorber line width at the mask (nm). Default 5000.
        pitch_nm: Line+space pitch (nm). Default 10000.
        grid_size: Lateral grid size (power of 2). Default 128.
        pixel_nm: Lateral pixel size (nm). Default 200.
        nz: Number of depth slices. Default 64.

    Returns:
        LigaResult with ``depth_dose`` (z_um, dose arrays), ``dose_ratio``,
        ``top_dose_kj_cm3``, ``bottom_dose_kj_cm3``, ``exceeds_damage_ceiling``,
        the volumetric dose field ``volume``, and the ``developed_depth`` map.
    """
    from highuvlith._validation import _validate_positive, _validate_power_of_two

    _validate_positive("critical_energy_kev", critical_energy_kev)
    _validate_positive("resist_thickness_um", resist_thickness_um)
    _validate_positive("cd_nm", cd_nm)
    _validate_positive("pitch_nm", pitch_nm)
    if cd_nm >= pitch_nm:
        raise ValueError(f"cd_nm ({cd_nm}) must be less than pitch_nm ({pitch_nm})")
    _validate_power_of_two("grid_size", grid_size)
    _validate_positive("pixel_nm", pixel_nm)
    _validate_positive("nz", nz)

    return _native_simulate_liga(
        critical_energy_kev=critical_energy_kev,
        resist_thickness_um=resist_thickness_um,
        cd_nm=cd_nm,
        pitch_nm=pitch_nm,
        grid_size=grid_size,
        pixel_nm=pixel_nm,
        nz=nz,
    )


def simulate_interference(
    preset: str = "two_beam",
    *,
    wavelength_nm: float = 200.0,
    n_medium: float = 1.6,
    half_angle_deg: float = 30.0,
    nx: int = 128,
    ny: int = 128,
    nz: int = 32,
    x_span_nm: float = 1000.0,
    y_span_nm: float = 1000.0,
    z_span_nm: float = 500.0,
    dose_scale: float = 1.0,
    dill_c: float = 0.02,
    two_photon: bool = False,
) -> VolumetricResult:
    """Simulate multi-beam interference (holographic) lithography.

    Superposes a few coherent plane waves into a periodic standing-wave
    lattice recorded as a PAC volume in a single exposure. The fringe period is
    fixed by the air-side ``half_angle_deg`` and is invariant under the resist
    index ``n_medium``.

    Args:
        preset: Beam geometry. ``"two_beam"`` (1D grating), ``"three_beam_hex"``
            (2D hexagonal lattice), or ``"four_beam_umbrella"`` (FCC-like 3D).
        wavelength_nm: Vacuum wavelength (nm). Default 200.
        n_medium: Resist refractive index. Default 1.6.
        half_angle_deg: Air-side incidence half-angle (deg). Default 30.
        nx, ny, nz: Voxel grid dimensions.
        x_span_nm, y_span_nm, z_span_nm: Physical grid extents (nm).
        dose_scale: Exposure dose scale for the Dill map.
        dill_c: Dill C exposure-rate constant.
        two_photon: Use ``I^2`` (multiphoton) kinetics instead of ``I``.

    Returns:
        VolumetricResult holding the PAC volume (``m = 1`` unexposed, ``m -> 0``
        fully exposed).
    """
    return _native_simulate_interference(
        preset,
        wavelength_nm,
        n_medium,
        half_angle_deg,
        nx,
        ny,
        nz,
        x_span_nm,
        y_span_nm,
        z_span_nm,
        dose_scale,
        dill_c,
        two_photon,
    )


@dataclass
class GrayscaleResult:
    """Round-trip result of a grayscale surface-relief design."""

    target_height_nm: np.ndarray
    transmittance: np.ndarray
    height_map: HeightMapResult


def simulate_grayscale(
    target_height_nm: np.ndarray,
    thickness_nm: float,
    *,
    d_th: float,
    d_clear: float,
    exposure_dose_mj_cm2: float,
    wavelength_nm: float = 157.63,
    na: float = 0.75,
    sigma: float = 0.7,
    pixel_nm: float = 2.0,
    max_kernels: int = 20,
) -> GrayscaleResult:
    """Design and print a grayscale surface relief in a single exposure.

    Synthesizes the continuous intensity-transmittance mask that prints
    ``target_height_nm`` through the log-linear contrast curve fixed by
    ``d_th``/``d_clear``, images it through the partially-coherent optics, and
    maps the resulting dose back to a remaining-height map.

    Args:
        target_height_nm: Desired remaining-height map, a square ``(n, n)``
            array (``n`` a power of 2) in nm. Use :func:`~highuvlith.blazed_grating`
            or :func:`~highuvlith.microlens_array` to generate one.
        thickness_nm: Full resist thickness (nm).
        d_th: Threshold dose (mJ/cm^2), below which no resist clears.
        d_clear: Clearing dose (mJ/cm^2), at/above which the resist clears fully.
        exposure_dose_mj_cm2: Single-exposure dose. Must exceed the maximum
            dose any pixel requires, else a ValueError is raised.
        wavelength_nm: Source wavelength (nm).
        na: Numerical aperture.
        sigma: Partial coherence factor.
        pixel_nm: Pixel size (nm).
        max_kernels: Max SOCS kernels.

    Returns:
        GrayscaleResult with the target, the synthesized ``transmittance``, and
        the printed ``height_map`` (a HeightMapResult).
    """
    from highuvlith._validation import _validate_positive, _validate_power_of_two

    target = np.ascontiguousarray(target_height_nm, dtype=np.float64)
    if target.ndim != 2 or target.shape[0] != target.shape[1]:
        raise ValueError(
            f"target_height_nm must be a square 2D array, got shape {target.shape}"
        )
    n = target.shape[0]
    _validate_power_of_two("target_height_nm size", n)
    _validate_positive("thickness_nm", thickness_nm)
    _validate_positive("exposure_dose_mj_cm2", exposure_dose_mj_cm2)

    transmittance = grayscale_transmittance_for_target(
        target, thickness_nm, d_th, d_clear, exposure_dose_mj_cm2
    )

    source = huv.SourceConfig(wavelength_nm=wavelength_nm, sigma_outer=sigma)
    optics = huv.OpticsConfig(numerical_aperture=na)
    grid = huv.GridConfig(size=n, pixel_nm=pixel_nm)

    height_map = grayscale_height_map(
        source,
        optics,
        grid,
        transmittance,
        exposure_dose_mj_cm2,
        d_th,
        d_clear,
        thickness_nm,
        max_kernels,
    )
    return GrayscaleResult(
        target_height_nm=target,
        transmittance=transmittance,
        height_map=height_map,
    )


@dataclass
class QuantumResult:
    """Classical vs. quantum (N-photon) aerial image comparison."""

    classical: np.ndarray
    quantum: np.ndarray
    classical_contrast: float
    quantum_contrast: float


def simulate_quantum_line_space(
    cd_nm: float,
    pitch_nm: float,
    *,
    n: int = 2,
    fidelity: float = 1.0,
    wavelength_nm: float = 157.63,
    na: float = 0.75,
    sigma: float = 0.7,
    focus_nm: float = 0.0,
    grid_size: int = 256,
    pixel_nm: float = 1.0,
    max_kernels: int = 20,
) -> QuantumResult:
    """Compare classical and quantum N-photon imaging of a line/space pattern.

    Computes the classical partially-coherent aerial image, then applies the
    theoretical NOON-state N-photon absorption ``E_N = fidelity*I^N +
    (1-fidelity)*I``, which sharpens features beyond the classical ``|E|^2``
    image. Quantum lithography is entirely theoretical (extreme flux penalty).

    Args:
        cd_nm: Line width at wafer scale (nm).
        pitch_nm: Line+space pitch (nm).
        n: Number of entangled photons (2 = biphoton).
        fidelity: Entanglement fidelity in [0, 1] (1 = pure quantum).
        wavelength_nm, na, sigma, focus_nm, grid_size, pixel_nm, max_kernels:
            Standard imaging parameters (see :func:`simulate_line_space`).

    Returns:
        QuantumResult with the classical and quantum intensity images and their
        contrasts.
    """
    classical_result = simulate_line_space(
        cd_nm,
        pitch_nm,
        wavelength_nm=wavelength_nm,
        na=na,
        sigma=sigma,
        focus_nm=focus_nm,
        grid_size=grid_size,
        pixel_nm=pixel_nm,
        max_kernels=max_kernels,
    )
    classical = np.asarray(classical_result.aerial.intensity)
    quantum = quantum_aerial_image(
        classical, n=n, wavelength_nm=wavelength_nm, na=na, fidelity=fidelity
    )

    def _contrast(img: np.ndarray) -> float:
        i_max = float(img.max())
        i_min = float(img.min())
        return (i_max - i_min) / (i_max + i_min) if (i_max + i_min) > 0 else 0.0

    return QuantumResult(
        classical=classical,
        quantum=quantum,
        classical_contrast=_contrast(classical),
        quantum_contrast=_contrast(quantum),
    )
