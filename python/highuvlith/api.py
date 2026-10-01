"""High-level convenience API for common simulation workflows."""

from __future__ import annotations

from dataclasses import dataclass, field
from typing import Any

import numpy as np

import highuvlith as huv
from highuvlith._native import (
    HeightMapResult,
    LigaResult,
    VolumetricResult,
    grayscale_height_map,
    grayscale_transmittance_for_target,
    quantum_aerial_image as quantum_aerial_image,
)

# Quantum (N-photon) research module (🧪): thin native pass-throughs.
from highuvlith._native import NoonImageResult as NoonImageResult
from highuvlith._native import TwoBeamNPhoton as TwoBeamNPhoton
from highuvlith._native import n_photon_absorption_image as n_photon_absorption_image
from highuvlith._native import quantum_flux_budget as quantum_flux_budget

# Re-exported through the high-level API namespace (thin native pass-through).
from highuvlith._native import expose_volumetric as expose_volumetric
from highuvlith._native import simulate_interference as _native_simulate_interference
from highuvlith._native import simulate_liga as _native_simulate_liga
from highuvlith._native import EuvIlResult, TalbotResult
from highuvlith._native import simulate_euv_il as _native_simulate_euv_il
from highuvlith._native import simulate_talbot as _native_simulate_talbot
from highuvlith._native import CarPebResult, LevelSetResult
from highuvlith._native import develop_fast_marching as _native_develop_fast_marching
from highuvlith._native import develop_level_set as _native_develop_level_set
from highuvlith._native import development_rate as development_rate
from highuvlith._native import peb_car as _native_peb_car
from highuvlith._native import peb_gaussian as _native_peb_gaussian

# X-ray materials / LIGA diffraction (WP-E1): thin native pass-throughs.
from highuvlith._native import LigaEdgeProfile as LigaEdgeProfile
from highuvlith._native import MultilayerMirror as MultilayerMirror
from highuvlith._native import liga_edge_profile as _native_liga_edge_profile
from highuvlith._native import tune_multilayer_period as tune_multilayer_period
from highuvlith._native import xray_optical_constants as xray_optical_constants


@dataclass
class FullResult:
    """Aggregated result from a single simulation run."""

    aerial: huv.AerialImageResult
    contrast: float
    config: dict[str, Any]
    resist_profile: huv.ResistProfileResult | None = None
    cd_nm: float | None = None
    nils: float | None = None
    #: Capability badge of the imaging model (see docs/capability-matrix.md).
    status: str = "✅"
    #: Stated approximations of the models behind this result.
    notes: list[str] = field(default_factory=list)


#: Water immersion index at 193 nm (``OpticsConfig.immersion_193i``).
WATER_INDEX_193 = 1.437


def _imaging_optics(na: float, immersion: bool | float) -> tuple[huv.OpticsConfig, float]:
    """Projection optics for ``na`` (dry, or immersion with index
    ``WATER_INDEX_193`` for ``True`` or the given float), and the image-space
    index."""
    from highuvlith._validation import _validate_range

    if immersion is False or immersion is None:
        _validate_range("na", na, 0.01, 0.99)
        return huv.OpticsConfig(numerical_aperture=na), 1.0
    index = WATER_INDEX_193 if immersion is True else float(immersion)
    if index < 1.0:
        raise ValueError(f"immersion index must be >= 1, got {index}")
    _validate_range("na", na, 0.01, 0.95 * index)
    return huv.OpticsConfig.immersion(numerical_aperture=na, immersion_index=index), index


def _vector_settings(imaging: str, polarization: str, angle_deg: float) -> huv.VectorSettings | None:
    """``VectorSettings`` for ``imaging="vector"`` (None for scalar)."""
    if imaging == "scalar":
        return None
    if imaging != "vector":
        raise ValueError(f"imaging must be 'scalar' or 'vector', got {imaging!r}")
    return huv.VectorSettings(polarization=polarization, angle_deg=angle_deg)


def _grid_config(grid: huv.GridConfig, requested_pixel_nm: float, pitch_nm: float) -> dict[str, Any]:
    """Grid entries of a result ``config`` dict (the pixel actually used,
    the requested one, and the commensurate field)."""
    return {
        "grid_size": grid.size,
        "pixel_nm": grid.pixel_nm,
        "pixel_nm_requested": requested_pixel_nm,
        "field_nm": grid.field_size_nm(),
        "periods_in_field": round(grid.periods_in_field(pitch_nm)),
    }


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
    immersion: bool | float = False,
    imaging: str = "scalar",
    polarization: str = "unpolarized",
    polarization_angle_deg: float = 0.0,
    illumination: tuple[Any, ...] | None = None,
    source: huv.SourceConfig | None = None,
) -> FullResult:
    """One-liner simulation of a line/space pattern.

    The mask is an infinite grating of opaque lines of width ``cd_nm``
    (bright field, one line centred at x = 0). Because the imaging FFTs make
    the field periodic, the grid is made commensurate: the field is set to a
    whole number of pitches and ``pixel_nm`` becomes the largest pixel not
    coarser than the requested one (see ``GridConfig.commensurate``). The
    pixel actually used is ``config["pixel_nm"]``; the request is kept in
    ``config["pixel_nm_requested"]``.

    Args:
        cd_nm: Line width at wafer scale (nm).
        pitch_nm: Line+space pitch (nm).
        wavelength_nm: Source wavelength (nm). Default 157.63 (F2 laser).
        na: Numerical aperture. Default 0.75.
        sigma: Partial coherence factor. Default 0.7.
        focus_nm: Defocus (nm). Default 0 (best focus).
        dose_mj_cm2: Exposure dose (mJ/cm²). Default 30.
        grid_size: Simulation grid size (must be power of 2). Default 256.
        pixel_nm: Target pixel size (nm); adjusted for commensurability.
            Default 1.0.
        with_resist: If True, also compute resist profile.
        max_kernels: Max SOCS kernels for TCC decomposition. Default 20.
        immersion: False (dry, NA < 1), True (water, n = 1.437 — the 193i
            index) or an immersion index; NA may then reach 0.95·n and
            defocus is applied in the medium.
        imaging: "scalar" (default) or "vector" (polarized high-NA imaging;
            TE/TM per diffraction order, radiometric obliquity, the
            immersion medium as image space).
        polarization: Vector illumination polarization: "unpolarized",
            "x", "y", "te" (azimuthal), "tm" (radial) or "linear" (at
            ``polarization_angle_deg`` from x). Ignored for scalar imaging.
        polarization_angle_deg: Angle for ``polarization="linear"`` (deg).
        illumination: Optional pupil fill replacing ``sigma``, as accepted by
            ``SourceConfig.with_illumination``, e.g. ``("annular", 0.5, 0.8)``.
        source: Optional ``SourceConfig`` (e.g. ``SourceConfig.arf_laser()``);
            overrides ``wavelength_nm`` and ``sigma`` (``illumination`` still
            applies on top).

    Returns:
        FullResult with aerial image, contrast, the printed line width
        ``cd_nm`` and ``nils`` of the dark line at the 0.3 intensity contour
        (None if no line prints), and optionally the resist profile.
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
    _validate_range("sigma", sigma, 0.01, 1.0)
    _validate_power_of_two("grid_size", grid_size)
    _validate_positive("pixel_nm", pixel_nm)

    optics, image_index = _imaging_optics(na, immersion)
    vector = _vector_settings(imaging, polarization, polarization_angle_deg)
    if source is None:
        source = huv.SourceConfig(
            wavelength_nm=wavelength_nm,
            sigma_outer=sigma,
        )
    if illumination is not None:
        source = source.with_illumination(tuple(illumination))
    mask = huv.MaskConfig.line_space(cd_nm=cd_nm, pitch_nm=pitch_nm)
    resist = huv.ResistConfig.vuv_fluoropolymer()
    grid = mask.commensurate_grid(size=grid_size, target_pixel_nm=pixel_nm)

    engine = huv.SimulationEngine(
        source, optics, mask, resist, grid, max_kernels=max_kernels, vector=vector
    )

    aerial = engine.compute_aerial_image(focus_nm=focus_nm)
    contrast = aerial.image_contrast()

    config = {
        "wavelength_nm": source.wavelength_nm,
        "na": na,
        "sigma": sigma,
        "illumination": source.illumination,
        "immersion_index": image_index,
        "imaging": imaging,
        "polarization": polarization if vector is not None else None,
        "cd_nm": cd_nm,
        "pitch_nm": pitch_nm,
        "focus_nm": focus_nm,
        "dose_mj_cm2": dose_mj_cm2,
        **_grid_config(grid, pixel_nm, pitch_nm),
    }

    result = FullResult(
        aerial=aerial,
        contrast=contrast,
        config=config,
        notes=[
            "Kirchhoff thin mask (no mask 3D); 2% uniform flare; clear-field normalized intensity",
            "cd_nm / nils at a constant 0.3 intensity threshold (no resist model)",
        ],
    )

    result.cd_nm = aerial.cd(threshold=0.3, tone="dark")
    result.nils = aerial.nils_periodic(threshold=0.3, tone="dark")

    if with_resist:
        result.resist_profile = engine.compute_resist_profile(
            dose_mj_cm2=dose_mj_cm2, focus_nm=focus_nm
        )
        # The resist model is depth-averaged Dill + centre-row development.
        result.status = "✅/🔶"
        result.notes.extend(result.resist_profile.notes)

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

    The mask is an infinite dark-field lattice of clear square holes (one at
    the origin). The grid is made commensurate with both pitches (the field
    is a whole number of their common period, pixel not coarser than
    requested unless one common period does not fit); ``config["pixel_nm"]``
    is the pixel used and ``config["pixel_nm_requested"]`` the request.

    Args:
        diameter_nm: Hole side at wafer (nm).
        pitch_x_nm: X-direction pitch (nm).
        pitch_y_nm: Y-direction pitch (nm). Defaults to pitch_x_nm.
        wavelength_nm: Source wavelength (nm).
        na: Numerical aperture.
        sigma: Partial coherence factor.
        focus_nm: Defocus (nm).
        grid_size: Simulation grid size.
        pixel_nm: Target pixel size (nm); adjusted for commensurability.
        max_kernels: Max SOCS kernels.

    Returns:
        FullResult with aerial image, contrast and the printed hole width
        ``cd_nm`` along x at the 0.3 intensity contour (None if none prints).
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

    _validate_positive("pixel_nm", pixel_nm)
    if diameter_nm >= min(pitch_x_nm, pitch_y_nm):
        raise ValueError(
            f"diameter_nm ({diameter_nm}) must be less than both pitches "
            f"({pitch_x_nm}, {pitch_y_nm})"
        )

    source = huv.SourceConfig(wavelength_nm=wavelength_nm, sigma_outer=sigma)
    optics = huv.OpticsConfig(numerical_aperture=na)
    mask = huv.MaskConfig.contact_hole(diameter_nm, pitch_x_nm, pitch_y_nm)
    grid = mask.commensurate_grid(size=grid_size, target_pixel_nm=pixel_nm)

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
        **_grid_config(grid, pixel_nm, pitch_x_nm),
        "periods_in_field_y": round(grid.periods_in_field(pitch_y_nm)),
    }

    return FullResult(
        aerial=aerial,
        contrast=aerial.image_contrast(),
        config=config,
        cd_nm=aerial.cd(threshold=0.3, tone="bright"),
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

    The line/space grid is made commensurate with ``pitch_nm`` exactly as in
    :func:`simulate_line_space` (``pixel_nm`` is a target, never exceeded
    unless one pitch does not fit in ``grid_size`` pixels).

    Returns:
        Dict with 'focuses', 'contrasts' arrays, 'best_focus_nm',
        'best_contrast', and the grid actually used: 'pixel_nm',
        'pixel_nm_requested', 'field_nm', 'periods_in_field'.
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

    _validate_positive("pixel_nm", pixel_nm)
    if cd_nm >= pitch_nm:
        raise ValueError(f"cd_nm ({cd_nm}) must be less than pitch_nm ({pitch_nm})")

    source = huv.SourceConfig(wavelength_nm=wavelength_nm, sigma_outer=sigma)
    optics = huv.OpticsConfig(numerical_aperture=na)
    mask = huv.MaskConfig.line_space(cd_nm=cd_nm, pitch_nm=pitch_nm)
    grid = mask.commensurate_grid(size=grid_size, target_pixel_nm=pixel_nm)

    batch = huv.BatchSimulator(source, optics, mask, grid)
    focuses = np.linspace(focus_min, focus_max, focus_steps).tolist()
    results = batch.batch_defocus(focuses=focuses)

    contrast_list = []
    for _f, img in results:
        arr = np.asarray(img)
        i_max = arr.max()
        i_min = arr.min()
        contrast_list.append((i_max - i_min) / (i_max + i_min) if (i_max + i_min) > 0 else 0.0)

    contrasts = np.array(contrast_list)
    best_idx = np.argmax(contrasts)

    grid_info = _grid_config(grid, pixel_nm, pitch_nm)
    return {
        "focuses": np.array(focuses),
        "contrasts": contrasts,
        "best_focus_nm": focuses[best_idx],
        "best_contrast": contrasts[best_idx],
        "pixel_nm": grid_info["pixel_nm"],
        "pixel_nm_requested": pixel_nm,
        "field_nm": grid_info["field_nm"],
        "periods_in_field": grid_info["periods_in_field"],
    }


def _liga_spectrum_kwargs(
    critical_energy_kev: float | None,
    source: Any,
    flux_density: Any,
    spectrum_table: Any,
) -> dict[str, Any]:
    """Validate the mutually exclusive LIGA spectrum selectors."""
    given = [
        name
        for name, value in (
            ("critical_energy_kev", critical_energy_kev),
            ("source", source),
            ("flux_density", flux_density),
            ("spectrum_table", spectrum_table),
        )
        if value is not None
    ]
    if len(given) > 1:
        raise ValueError(f"give only one spectrum selector, got {given}")
    kwargs: dict[str, Any] = {}
    if critical_energy_kev is not None:
        from highuvlith._validation import _validate_positive

        _validate_positive("critical_energy_kev", critical_energy_kev)
        kwargs["critical_energy_kev"] = critical_energy_kev
    if source is not None:
        kwargs["source"] = source
    if flux_density is not None:
        kwargs["flux_density"] = [(float(e), float(v)) for e, v in flux_density]
    if spectrum_table is not None:
        kwargs["spectrum_table"] = [(float(e), float(w)) for e, w in spectrum_table]
    return kwargs


def simulate_liga(
    critical_energy_kev: float | None = None,
    resist_thickness_um: float = 500.0,
    *,
    cd_nm: float = 5000.0,
    pitch_nm: float = 10000.0,
    grid_size: int = 128,
    pixel_nm: float = 200.0,
    nz: int = 64,
    diffraction: str = "fresnel",
    proximity_gap_um: float = 100.0,
    energy_bins: int = 100,
    photoelectron_blur: bool = False,
    filters: list[tuple[str, float]] | None = None,
    absorber: str = "Au",
    absorber_thickness_um: float = 20.0,
    membrane: str = "Ti",
    membrane_thickness_um: float = 2.0,
    target_bottom_dose_kj_cm3: float = 3.0,
    damage_dose_kj_cm3: float = 20.0,
    source: huv.SourceConfig | None = None,
    source_distance_m: float | None = None,
    horizontal_acceptance_mrad: float = 5.0,
    vertical_scan_mm: float | None = None,
    flux_density: list[tuple[float, float]] | None = None,
    spectrum_table: list[tuple[float, float]] | None = None,
    strict_sampling: bool = False,
    warn: bool = True,
) -> LigaResult:
    """Simulate a LIGA deep-X-ray shadow exposure of thick PMMA.

    A bending-magnet white beam (or a supplied spectrum) exposes PMMA through
    an Au absorber on a membrane across a proximity gap. The lateral dose uses
    scalar Fresnel (angular-spectrum) propagation of the complex absorber
    field over gap + depth for every energy bin (``diffraction="fresnel"``,
    default) or the legacy Gaussian blur ``sigma = sqrt(lambda g)/2``
    (``"gaussian"``).

    Args:
        critical_energy_kev: Bending-magnet critical energy (keV); default
            6.23 (LIGA-class 2.5 GeV / 1.5 T) when no other spectrum is given.
            Relative spectrum: no exposure time.
        resist_thickness_um: PMMA thickness (um). Default 500.
        cd_nm: Absorber line width at the mask (nm). Default 5000.
        pitch_nm: Line+space pitch (nm). Default 10000.
        grid_size: Lateral grid size (power of 2). Default 128.
        pixel_nm: Lateral pixel size (nm). Default 200 (coarser than the
            Fresnel scale ~0.2 um: the result is then the pixel-averaged
            shadow and ``warnings`` says so; use ``liga_edge_profile`` for
            sidewalls).
        nz: Number of depth slices. Default 64.
        diffraction: ``"fresnel"`` (default) or ``"gaussian"``.
        proximity_gap_um: Absorber-to-resist gap (um). Default 100.
        energy_bins: Spectral bins (log-spaced for a bending magnet).
        photoelectron_blur: Add a Grun-range Gaussian (crude upper bound).
        filters: Upstream filters as ``(material, thickness_um)``; material is
            a preset (``"Be"``, ``"Al"``, ``"Kapton"``, ``"Ti"``, ...) or
            ``"<formula>@<density>"``.
        absorber, absorber_thickness_um: Mask absorber (default 20 um Au).
        membrane, membrane_thickness_um: Mask membrane (default 2 um Ti).
        target_bottom_dose_kj_cm3: Clearing dose at the resist bottom.
        damage_dose_kj_cm3: Top-dose damage ceiling.
        source: A synchrotron bending-magnet ``SourceConfig``; with both
            ``source_distance_m`` and ``vertical_scan_mm`` the flux is
            absolute (ring energy, field and current from the source).
        source_distance_m: Source-to-mask distance (m).
        horizontal_acceptance_mrad: Horizontal fan accepted (mrad).
        vertical_scan_mm: Height of the uniform vertical scan (mm).
        flux_density: Absolute ``(E_keV, photons s^-1 mm^-2 keV^-1)`` table at
            the mask (e.g. an X-ray tube) - gives an exposure time.
        spectrum_table: Relative ``(E_keV, weight)`` lines.
        strict_sampling: Raise instead of warning when the grid cannot
            resolve the Fresnel scale.
        warn: Emit Python ``UserWarning`` s for the result's ``warnings``.

    Returns:
        LigaResult with ``depth_dose``, ``dose_ratio``, top/bottom doses,
        ``exceeds_damage_ceiling``, ``volume``, ``developed_depth``, and -
        for absolute spectra - ``exposure_time_s``, dose rates,
        ``resist_power_density_w_mm2``, ``exposure_charge_ma_h``; plus
        ``fresnel_scale_nm``, ``sampling_resolved`` and ``warnings``.
    """
    from highuvlith._validation import _validate_positive, _validate_power_of_two

    spectrum_kwargs = _liga_spectrum_kwargs(critical_energy_kev, source, flux_density, spectrum_table)
    _validate_positive("resist_thickness_um", resist_thickness_um)
    _validate_positive("cd_nm", cd_nm)
    _validate_positive("pitch_nm", pitch_nm)
    if cd_nm >= pitch_nm:
        raise ValueError(f"cd_nm ({cd_nm}) must be less than pitch_nm ({pitch_nm})")
    _validate_power_of_two("grid_size", grid_size)
    _validate_positive("pixel_nm", pixel_nm)
    _validate_positive("nz", nz)
    _validate_positive("energy_bins", energy_bins)
    if proximity_gap_um < 0:
        raise ValueError(f"proximity_gap_um must be >= 0, got {proximity_gap_um}")
    if diffraction not in ("fresnel", "gaussian"):
        raise ValueError(f"diffraction must be 'fresnel' or 'gaussian', got {diffraction!r}")
    if (source_distance_m is None) != (vertical_scan_mm is None):
        raise ValueError("absolute exposure needs both source_distance_m and vertical_scan_mm")
    if source_distance_m is not None and source is None:
        raise ValueError("source_distance_m / vertical_scan_mm need a synchrotron `source`")

    result = _native_simulate_liga(
        resist_thickness_um=resist_thickness_um,
        cd_nm=cd_nm,
        pitch_nm=pitch_nm,
        grid_size=grid_size,
        pixel_nm=pixel_nm,
        nz=nz,
        diffraction=diffraction,
        proximity_gap_um=proximity_gap_um,
        energy_bins=energy_bins,
        photoelectron_blur=photoelectron_blur,
        filters=filters,
        absorber=absorber,
        absorber_thickness_um=absorber_thickness_um,
        membrane=membrane,
        membrane_thickness_um=membrane_thickness_um,
        target_bottom_dose_kj_cm3=target_bottom_dose_kj_cm3,
        damage_dose_kj_cm3=damage_dose_kj_cm3,
        source_distance_m=source_distance_m,
        horizontal_acceptance_mrad=horizontal_acceptance_mrad,
        vertical_scan_mm=vertical_scan_mm,
        strict_sampling=strict_sampling,
        **spectrum_kwargs,
    )
    if warn:
        import warnings

        for message in result.warnings:
            warnings.warn(message, UserWarning, stacklevel=2)
    return result


def liga_edge_profile(
    critical_energy_kev: float | None = None,
    resist_thickness_um: float = 500.0,
    *,
    proximity_gap_um: float = 100.0,
    x_min_nm: float = -3000.0,
    x_max_nm: float = 3000.0,
    dx_nm: float = 10.0,
    depths_um: list[float] | None = None,
    energy_bins: int = 100,
    photoelectron_blur: bool = False,
    filters: list[tuple[str, float]] | None = None,
    absorber: str = "Au",
    absorber_thickness_um: float = 20.0,
    membrane: str = "Ti",
    membrane_thickness_um: float = 2.0,
    target_bottom_dose_kj_cm3: float = 3.0,
    source: huv.SourceConfig | None = None,
    spectrum_table: list[tuple[float, float]] | None = None,
    flux_density: list[tuple[float, float]] | None = None,
) -> LigaEdgeProfile:
    """Fine 1D Fresnel dose profile across a straight LIGA absorber edge.

    Exact paraxial straight-edge diffraction (Fresnel integrals) per energy
    bin, summed with the depth-dose weights, at arbitrary sampling (default
    10 nm) and at ``depths_um`` (default: 5 depths top to bottom). Use
    ``edge_positions_nm(threshold)`` and ``sidewall_angle_deg(threshold)``
    for sidewall analysis. Arguments match :func:`simulate_liga`.
    """
    from highuvlith._validation import _validate_positive

    spectrum_kwargs = _liga_spectrum_kwargs(critical_energy_kev, source, flux_density, spectrum_table)
    _validate_positive("resist_thickness_um", resist_thickness_um)
    _validate_positive("dx_nm", dx_nm)
    if x_max_nm <= x_min_nm:
        raise ValueError(f"x_max_nm ({x_max_nm}) must exceed x_min_nm ({x_min_nm})")
    if proximity_gap_um < 0:
        raise ValueError(f"proximity_gap_um must be >= 0, got {proximity_gap_um}")
    return _native_liga_edge_profile(
        resist_thickness_um=resist_thickness_um,
        proximity_gap_um=proximity_gap_um,
        x_min_nm=x_min_nm,
        x_max_nm=x_max_nm,
        dx_nm=dx_nm,
        depths_um=None if depths_um is None else [float(z) for z in depths_um],
        energy_bins=energy_bins,
        photoelectron_blur=photoelectron_blur,
        filters=filters,
        absorber=absorber,
        absorber_thickness_um=absorber_thickness_um,
        membrane=membrane,
        membrane_thickness_um=membrane_thickness_um,
        target_bottom_dose_kj_cm3=target_bottom_dose_kj_cm3,
        **spectrum_kwargs,
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
    """Classical vs. N-photon exposure of a line/space pattern (🧪 Theoretical).

    ``quantum`` is the N-photon exposure of the chosen ``model``:
    ``"n_photon_absorption"`` — classical N-photon absorption ``I^N``
    (sharpening at the classical period; no resolution gain) — or
    ``"noon"`` — the ideal N00N limit ``F·I_{λ/N} + (1 − F)·I^N``
    (Boto et al. 2000; assumes the required entangled states exist).
    """

    classical: np.ndarray
    quantum: np.ndarray
    classical_contrast: float
    quantum_contrast: float
    model: str = "n_photon_absorption"
    num_photons: int = 2
    fidelity: float = 1.0
    classical_resolution_nm: float = 0.0
    noon_limit_resolution_nm: float = 0.0
    #: Exposure-time ratio entangled / classical at the loosest ETPA bound —
    #: a LOWER bound (see ``quantum_flux_budget``).
    exposure_time_ratio_bound: float = 1.0
    status: str = "🧪"
    notes: list[str] = field(default_factory=list)


def _contrast(img: np.ndarray) -> float:
    i_max = float(img.max())
    i_min = float(img.min())
    return (i_max - i_min) / (i_max + i_min) if (i_max + i_min) > 0 else 0.0


def simulate_quantum_line_space(
    cd_nm: float,
    pitch_nm: float,
    *,
    n: int = 2,
    fidelity: float = 1.0,
    model: str = "n_photon_absorption",
    wavelength_nm: float = 157.63,
    na: float = 0.75,
    sigma: float = 0.7,
    focus_nm: float = 0.0,
    grid_size: int = 256,
    pixel_nm: float = 1.0,
    max_kernels: int = 20,
) -> QuantumResult:
    """Compare classical and N-photon exposure of a line/space pattern (🧪).

    Two physically different models (``model``):

    - ``"n_photon_absorption"`` (default): classical N-photon absorption
      ``I^N`` of the classical partially coherent image. It sharpens lines
      and raises contrast but keeps the classical period — a pitch the
      optics does not transmit stays unresolved. ``fidelity`` has no effect
      (no entanglement is involved).
    - ``"noon"``: the ideal N00N limit — the same source fill, optics, mask
      and grid imaged at ``λ/N`` — mixed with classical N-photon absorption:
      ``F·I_{λ/N} + (1 − F)·I^N`` (``F`` = ``fidelity``). An idealization
      (Boto et al., PRL 85, 2733, 2000): it assumes entangled states that
      write the pattern exist; needs ``pixel_nm ≤ λ/(4·N·NA)``.

    Both are theoretical: no N-photon resist has recorded an entangled
    sub-Rayleigh pattern, and the flux penalty is enormous
    (``exposure_time_ratio_bound`` ≈ 4.5e10 at N = 2, 157.63 nm — itself a lower bound).

    Args:
        cd_nm: Line width at wafer scale (nm).
        pitch_nm: Line+space pitch (nm).
        n: Photons per absorption event N (2 = biphoton).
        fidelity: N00N fraction F in [0, 1] (``model="noon"`` only).
        model: ``"n_photon_absorption"`` or ``"noon"``.
        wavelength_nm, na, sigma, focus_nm, grid_size, pixel_nm, max_kernels:
            Standard imaging parameters (see :func:`simulate_line_space`).

    Returns:
        QuantumResult with the classical image, the N-photon exposure, their
        contrasts, the resolution figures and the flux-budget ratio.
    """
    _check_choice("model", model, ("n_photon_absorption", "noon"))
    budget = quantum_flux_budget(wavelength_nm, n)
    if model == "n_photon_absorption":
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
        quantum = n_photon_absorption_image(classical, n)
        notes = [
            "classical N-photon absorption I^N: sharpening at the classical period, no resolution gain",
            "fidelity has no effect (no entanglement involved)",
        ]
        classical_res = 0.61 * wavelength_nm / na
        noon_res = classical_res / n
    else:
        from highuvlith._validation import _validate_positive, _validate_power_of_two

        _validate_positive("cd_nm", cd_nm)
        _validate_positive("pitch_nm", pitch_nm)
        _validate_power_of_two("grid_size", grid_size)
        source = huv.SourceConfig(wavelength_nm=wavelength_nm, sigma_outer=sigma)
        optics = huv.OpticsConfig(numerical_aperture=na)
        mask = huv.MaskConfig.line_space(cd_nm=cd_nm, pitch_nm=pitch_nm)
        grid = mask.commensurate_grid(size=grid_size, target_pixel_nm=pixel_nm)
        engine = huv.SimulationEngine(source, optics, mask, grid=grid, max_kernels=max_kernels)
        image = engine.compute_noon_ideal_image(num_photons=n, fidelity=fidelity, focus_nm=focus_nm)
        classical = np.asarray(image.classical.intensity)
        quantum = np.asarray(image.exposure.intensity)
        notes = list(image.notes)
        classical_res = image.classical_resolution_nm
        noon_res = image.noon_limit_resolution_nm
    return QuantumResult(
        classical=classical,
        quantum=quantum,
        classical_contrast=_contrast(classical),
        quantum_contrast=_contrast(quantum),
        model=model,
        num_photons=n,
        fidelity=fidelity,
        classical_resolution_nm=classical_res,
        noon_limit_resolution_nm=noon_res,
        exposure_time_ratio_bound=budget["exposure_time_ratio_bound"],
        notes=notes,
    )


# --- Vector (polarized) pupil model (optics::vector) -------------------------

from highuvlith._native import VectorSettings as VectorSettings  # noqa: E402
from highuvlith._native import vector_pupil_field as _native_vector_pupil_field  # noqa: E402
from highuvlith._native import vector_pupil_map as _native_vector_pupil_map  # noqa: E402


@dataclass
class PupilPolarizationMap:
    """Image-side field over the whole pupil for one source point.

    ``field`` is complex with shape ``(n_states, 3, n, n)``, indexed
    ``[state, component, iy, ix]`` (components ``E_x, E_y, E_z``; states are
    mutually incoherent, weights folded in as amplitudes). ``px``/``py`` are
    the pupil coordinates in units of NA; points outside the pupil are zero.
    """

    px: np.ndarray
    py: np.ndarray
    field: np.ndarray
    settings: VectorSettings
    na: float

    @property
    def intensity(self) -> np.ndarray:
        """Total ``|E|²`` per pupil point, summed over states and components."""
        return np.sum(np.abs(self.field) ** 2, axis=(0, 1))

    @property
    def longitudinal_fraction(self) -> np.ndarray:
        """``|E_z|² / |E|²`` per pupil point (0 outside the pupil)."""
        ez = np.sum(np.abs(self.field[:, 2]) ** 2, axis=0)
        total = self.intensity
        return np.divide(ez, total, out=np.zeros_like(total), where=total > 0)


def pupil_polarization_map(
    na: float,
    polarization: str = "x",
    *,
    n: int = 65,
    sx: float = 0.0,
    sy: float = 0.0,
    angle_deg: float = 0.0,
    image_index: float = 1.0,
    obliquity: bool = True,
    reduction: float = 4.0,
    film_n: float | None = None,
    film_k: float = 0.0,
) -> PupilPolarizationMap:
    """Evaluate the vector pupil (TE/TM rotation, obliquity, film) on an n×n grid.

    Args:
        na: Image-side numerical aperture (must be below ``image_index``).
        polarization: ``"unpolarized"``, ``"x"``, ``"y"``, ``"te"``, ``"tm"``,
            or ``"linear"`` (with ``angle_deg``).
        n: Grid points per pupil axis (``p = -1 … 1``).
        sx, sy: Source point in σ units (sets the TE/TM illumination basis).
        angle_deg: Linear polarization angle from x (only for ``"linear"``).
        image_index: Index of the homogeneous image medium (1.0 = air).
        obliquity: Apply the radiometric factor ``(cos θ_obj / cos θ_img)^½``.
        reduction: Projection reduction ratio used by the obliquity factor.
        film_n, film_k: Optional film entrance interface ``n + ik``.

    Returns:
        PupilPolarizationMap with the complex field and pupil coordinates.
    """
    settings = VectorSettings(
        polarization,
        angle_deg=angle_deg,
        image_index=image_index,
        obliquity=obliquity,
        reduction=reduction,
        film_n=film_n,
        film_k=film_k,
    )
    field = _native_vector_pupil_map(settings, na, n=n, sx=sx, sy=sy)
    coords = np.linspace(-1.0, 1.0, n)
    return PupilPolarizationMap(px=coords, py=coords.copy(), field=field, settings=settings, na=na)


def vector_two_beam_contrast(
    na: float,
    polarization: str = "x",
    *,
    p0: float = 1.0,
    sx: float = 0.5,
    sy: float = 0.0,
    angle_deg: float = 0.0,
    image_index: float = 1.0,
    film_n: float | None = None,
    film_k: float = 0.0,
) -> float:
    """Fringe contrast of two orders at pupil ``(±p0, 0)`` under the vector model.

    Closed forms for orders along x (so ``"y"`` is TE and ``"x"`` is TM):
    TE → 1, TM → ``|cos 2θ|``, unpolarized → ``cos²θ`` with
    ``sin θ = NA·p0 / image_index`` (inside a film, θ is the refracted angle).
    The obliquity factor is common to both orders and cancels.

    Returns:
        ``2·|Σ E₁·E₂*| / Σ(|E₁|² + |E₂|²)`` summed over incoherent states.
    """
    settings = VectorSettings(
        polarization,
        angle_deg=angle_deg,
        image_index=image_index,
        film_n=film_n,
        film_k=film_k,
    )
    e1 = _native_vector_pupil_field(settings, na, p0, 0.0, sx, sy)
    e2 = _native_vector_pupil_field(settings, na, -p0, 0.0, sx, sy)
    cross = np.sum(e1 * np.conj(e2))
    power = np.sum(np.abs(e1) ** 2 + np.abs(e2) ** 2)
    return float(2.0 * abs(cross) / power) if power > 0 else 0.0


# --- Optimization / patterning: ILT, OPC, SRAF, SADP/SAQP ---------------------

from highuvlith._native import (  # noqa: E402
    DofComparison,
    IltResult,
    OpcResult,
    SpacerPatterningResult,
    SrafResult,
)
from highuvlith._native import compare_sraf_dof as _native_compare_sraf_dof  # noqa: E402
from highuvlith._native import dose_to_size_threshold as _native_dose_to_size  # noqa: E402
from highuvlith._native import fragment_opc as _native_fragment_opc  # noqa: E402
from highuvlith._native import ilt_contact_target as _native_ilt_contact_target  # noqa: E402
from highuvlith._native import insert_srafs as _native_insert_srafs  # noqa: E402
from highuvlith._native import mask_from_features as mask_from_features  # noqa: E402
from highuvlith._native import optimize_ilt as _native_optimize_ilt  # noqa: E402
from highuvlith._native import sadp as _native_sadp  # noqa: E402
from highuvlith._native import saqp as _native_saqp  # noqa: E402


def contact_target(
    size_nm: float,
    pitch_nm: float,
    count: int,
    grid: huv.GridConfig,
    *,
    shape: str = "round",
) -> np.ndarray:
    """ILT target for a centred ``count x count`` hole array (1 inside holes).

    Args:
        size_nm: Hole diameter (round) or side (square) in nm.
        pitch_nm: Hole pitch in x and y (nm).
        count: Holes per row and column.
        grid: Simulation grid (the target has shape ``(grid.size, grid.size)``).
        shape: ``"round"`` (curvilinear target) or ``"square"``.
    """
    from highuvlith._validation import _validate_positive

    _validate_positive("size_nm", size_nm)
    _validate_positive("pitch_nm", pitch_nm)
    _validate_positive("count", count)
    return _native_ilt_contact_target(size_nm, pitch_nm, pitch_nm, count, count, grid, shape)


def optimize_ilt(
    source: huv.SourceConfig,
    optics: huv.OpticsConfig,
    grid: huv.GridConfig,
    target: np.ndarray,
    **kwargs: Any,
) -> IltResult:
    """Inverse lithography with the exact adjoint gradient through the SOCS kernels.

    Thin wrapper over :func:`highuvlith._native.optimize_ilt` that checks the
    target shape. Useful keywords: ``cost="resist"`` with ``threshold`` and
    ``steepness`` (sigmoid resist contour), ``gradient="proxy"`` (legacy
    local approximation, for comparison), ``conditions=[(defocus_nm, dose,
    weight), ...]`` (process-window ILT), ``binarization_weight`` with
    ``binarization_after`` (continuation), ``min_feature_nm`` (Gaussian
    density filter), ``snapshot_every`` (mask evolution), ``illumination``.
    """
    target = np.ascontiguousarray(target, dtype=np.float64)
    if target.shape != (grid.size, grid.size):
        raise ValueError(
            f"target shape {target.shape} must match the grid ({grid.size}, {grid.size})"
        )
    return _native_optimize_ilt(source, optics, grid, target, **kwargs)


def fragment_opc(
    source: huv.SourceConfig,
    optics: huv.OpticsConfig,
    mask: huv.MaskConfig,
    grid: huv.GridConfig,
    threshold: float,
    **kwargs: Any,
) -> OpcResult:
    """Fragment-based model OPC of the rectangles / rectilinear polygons of ``mask``.

    ``threshold`` is the print threshold on the aerial image; unset options
    take resolution-scaled defaults (fragments 0.3 lambda/NA, corner fragments
    0.25 lambda/NA, rms tolerance 0.005 lambda/NA, ...). Pass
    ``conditions=[(defocus_nm, dose, weight), ...]`` for process-window OPC.
    """
    from highuvlith._validation import _validate_positive

    _validate_positive("threshold", threshold)
    return _native_fragment_opc(source, optics, mask, grid, threshold, **kwargs)


def insert_srafs(
    source: huv.SourceConfig,
    optics: huv.OpticsConfig,
    mask: huv.MaskConfig,
    grid: huv.GridConfig,
    *,
    line_cd_nm: float,
    sigma_center: float,
    threshold: float,
    **kwargs: Any,
) -> SrafResult:
    """Place scattering bars with the heuristic rule deck, then remove/shrink
    every assist that prints at the (focus, dose) corners."""
    from highuvlith._validation import _validate_positive

    _validate_positive("line_cd_nm", line_cd_nm)
    _validate_positive("sigma_center", sigma_center)
    _validate_positive("threshold", threshold)
    return _native_insert_srafs(
        source, optics, mask, grid, line_cd_nm, sigma_center, threshold, **kwargs
    )


def compare_sraf_dof(
    source: huv.SourceConfig,
    optics: huv.OpticsConfig,
    mask_without: huv.MaskConfig,
    mask_with: huv.MaskConfig,
    grid: huv.GridConfig,
    *,
    target_cd_nm: float,
    **kwargs: Any,
) -> DofComparison:
    """Depth of focus of a line with and without assists (each at its own
    dose-to-size threshold). The gain depends on the engine's defocus model."""
    from highuvlith._validation import _validate_positive

    _validate_positive("target_cd_nm", target_cd_nm)
    return _native_compare_sraf_dof(
        source, optics, mask_without, mask_with, grid, target_cd_nm, **kwargs
    )


@dataclass
class SrafStudy:
    """Isolated-line SRAF study: dose-to-size threshold, assists, and DOF."""

    threshold: float
    sraf: SrafResult
    dof: DofComparison

    @property
    def status(self) -> str:
        """Capability badge: the rule deck and the DOF comparison are both 🔶."""
        return self.sraf.status if self.sraf.status == self.dof.status else "🔶"

    @property
    def notes(self) -> list[str]:
        """Stated approximations of the SRAF placement and the DOF comparison."""
        return list(self.sraf.notes) + [n for n in self.dof.notes if n not in self.sraf.notes]


def isolated_line_sraf_study(
    cd_nm: float = 80.0,
    *,
    illumination: tuple = ("annular", 0.5, 0.8),
    sigma_center: float = 0.65,
    na: float = 0.75,
    grid_size: int = 64,
    pixel_nm: float = 12.0,
    defocus_nm: float = 150.0,
    dose_excursion: float = 0.08,
    focus_range_nm: float = 300.0,
    focus_steps: int = 13,
    max_kernels: int = 16,
) -> SrafStudy:
    """One-call SRAF demo: an isolated opaque line on a bright-field F2 mask,
    anchored to print ``cd_nm`` in focus, scattering bars inserted and print
    checked at ``+/-defocus_nm`` x ``1 +/- dose_excursion``, and the DOF
    (+/-10 % CD) compared with and without the bars."""
    from highuvlith._validation import _validate_positive, _validate_power_of_two

    _validate_positive("cd_nm", cd_nm)
    _validate_power_of_two("grid_size", grid_size)
    _validate_positive("pixel_nm", pixel_nm)
    source = huv.SourceConfig.f2_laser(0.5)
    optics = huv.OpticsConfig(numerical_aperture=na)
    grid = huv.GridConfig(size=grid_size, pixel_nm=pixel_nm)
    field = grid_size * pixel_nm
    bare = huv.MaskConfig.from_features([(0.0, 0.0, cd_nm, field)])
    threshold = _native_dose_to_size(
        source, optics, bare, grid, cd_nm, max_kernels=max_kernels, illumination=illumination
    )
    sraf = _native_insert_srafs(
        source,
        optics,
        bare,
        grid,
        cd_nm,
        sigma_center,
        threshold,
        defocus_nm=defocus_nm,
        dose_excursion=dose_excursion,
        max_kernels=max_kernels,
        illumination=illumination,
    )
    dof = _native_compare_sraf_dof(
        source,
        optics,
        bare,
        sraf.mask,
        grid,
        cd_nm,
        focus_range_nm=focus_range_nm,
        focus_steps=focus_steps,
        max_kernels=max_kernels,
        illumination=illumination,
    )
    return SrafStudy(threshold=threshold, sraf=sraf, dof=dof)


def sadp(
    mandrel_pitch_nm: float,
    mandrel_cd_nm: float,
    spacer_nm: float,
    *,
    tone: str = "spacer_is_line",
) -> SpacerPatterningResult:
    """Geometric SADP: pitch P, mandrel CD W, spacer t (pitch walk 2W + 2t - P)."""
    return _native_sadp(mandrel_pitch_nm, mandrel_cd_nm, spacer_nm, tone)


def saqp(
    mandrel_pitch_nm: float,
    mandrel_cd_nm: float,
    spacer1_nm: float,
    spacer2_nm: float,
    *,
    tone: str = "spacer_is_line",
) -> SpacerPatterningResult:
    """Geometric SAQP (two spacer steps); uniform at W = 3P/8, t1 = t2 = P/8."""
    return _native_saqp(mandrel_pitch_nm, mandrel_cd_nm, spacer1_nm, spacer2_nm, tone)
# ---------------------------------------------------------------------------
# Talbot / EUV interference lithography
# ---------------------------------------------------------------------------

_TALBOT_GRATINGS = ("amplitude", "phase", "sinusoidal", "holes_square", "holes_hex")


def simulate_talbot(
    wavelength_nm: float = 13.5,
    period_nm: float = 100.0,
    *,
    grating: str = "amplitude",
    duty_cycle: float = 0.5,
    phase_rad: float | None = None,
    hole_diameter_nm: float | None = None,
    max_order: int = 10,
    propagation: str = "exact",
    gap_nm: float | None = None,
    n_periods: int = 2,
    nx: int = 128,
    ny: int | None = None,
    carpet_z_max_nm: float | None = None,
    carpet_nz: int = 128,
    scan_length_nm: float | None = None,
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
    """Talbot, displacement-Talbot (DTL) and achromatic-Talbot (ATL) lithography.

    A transmission grating under a normally incident plane wave re-images
    itself every Talbot length ``z_T = 2 p^2 / lambda`` (paraxial). Scanning
    the gap over ``z_T`` (DTL) or illuminating with a broadband source beyond
    the achromatic distance ``z_A = 2 p^2 / delta_lambda`` (ATL) prints a
    gap-independent "stationary" image; for a 1D grating its period is
    ``p / 2``. Scalar thin-mask model with exact angular-spectrum (default) or
    paraxial Fresnel propagation; intensities are normalized to the incident
    wave.

    Args:
        wavelength_nm: Vacuum (centre) wavelength (nm). Default 13.5 (EUV).
        period_nm: Grating period along x (nm); for ``holes_hex`` the
            nearest-neighbour hole pitch.
        grating: ``"amplitude"`` (binary, open fraction ``duty_cycle``),
            ``"phase"`` (binary, phase ``phase_rad`` over ``duty_cycle``),
            ``"sinusoidal"`` (0.5 + 0.5 cos), ``"holes_square"`` or
            ``"holes_hex"`` (circular holes of ``hole_diameter_nm``, default
            ``period_nm / 2``).
        duty_cycle: Open / phase-shifted fraction of each period, in [0, 1].
        phase_rad: Phase step of a phase grating (default pi).
        hole_diameter_nm: Hole diameter for the hole-array gratings.
        max_order: Highest retained diffraction order |n| (hole arrays:
            circular cutoff ``|f| <= max_order / p``).
        propagation: ``"exact"`` (angular spectrum) or ``"paraxial"``.
        gap_nm: Mask-to-wafer gap for the coherent/ATL images. Default:
            ``2 z_A`` if ``bandwidth_nm > 0`` else ``z_T``.
        n_periods: Number of periods spanned by the lateral images.
        nx, ny: Lateral samples (``ny`` defaults to 1 for 1D gratings, ``nx``
            for 2D ones).
        carpet_z_max_nm: Depth of the Talbot carpet (default ``2 z_T``).
        carpet_nz: Carpet rows.
        scan_length_nm: DTL gap-scan length (default ``z_T``).
        bandwidth_nm: Source bandwidth for ATL (Gaussian FWHM or flat-top full
            width); 0 disables the ATL image.
        spectrum: ``"gaussian"`` or ``"flat"``.
        spectral_bins: Spectral bins for the exact-propagation ATL average.
        resist_thickness_nm: If > 0, also compute the intensity volume in a
            resist of this thickness at the gap.
        resist_index: Real refractive index of the resist (may be < 1 at EUV).
        absorption_per_nm: Resist intensity absorption coefficient (1/nm).
        nz: Depth slices of the resist volume.
        exposure: Volume exposure mode: ``"coherent"``, ``"dtl"``,
            ``"stationary"`` or ``"atl"``.
        dose_scale, dill_c: If ``dill_c > 0``, also map the volume to PAC
            ``m = exp(-C * dose * I)``.

    Returns:
        TalbotResult with the carpet, ``coherent_image``,
        ``stationary_image``, ``dtl_image``, ``atl_image``, the Talbot and
        achromatic lengths, diffraction efficiencies, and optional
        ``intensity_volume`` / ``pac_volume``.
    """
    from highuvlith._validation import _validate_positive, _validate_range

    _validate_positive("wavelength_nm", wavelength_nm)
    _validate_positive("period_nm", period_nm)
    if grating not in _TALBOT_GRATINGS:
        raise ValueError(f"grating must be one of {_TALBOT_GRATINGS}, got {grating!r}")
    _validate_range("duty_cycle", duty_cycle, 0.0, 1.0)
    if propagation not in ("exact", "paraxial"):
        raise ValueError(f"propagation must be 'exact' or 'paraxial', got {propagation!r}")
    if gap_nm is not None and gap_nm < 0:
        raise ValueError(f"gap_nm must be >= 0, got {gap_nm}")
    _validate_positive("nx", nx)
    _validate_positive("carpet_nz", carpet_nz)
    if bandwidth_nm < 0:
        raise ValueError(f"bandwidth_nm must be >= 0, got {bandwidth_nm}")
    if exposure not in ("coherent", "dtl", "stationary", "atl"):
        raise ValueError(
            f"exposure must be coherent / dtl / stationary / atl, got {exposure!r}"
        )
    if exposure == "atl" and bandwidth_nm <= 0:
        raise ValueError("exposure='atl' needs bandwidth_nm > 0")

    return _native_simulate_talbot(
        wavelength_nm=wavelength_nm,
        period_nm=period_nm,
        grating=grating,
        duty_cycle=duty_cycle,
        phase_rad=phase_rad,
        hole_diameter_nm=hole_diameter_nm,
        max_order=max_order,
        propagation=propagation,
        gap_nm=gap_nm,
        n_periods=n_periods,
        nx=nx,
        ny=ny,
        carpet_z_max_nm=carpet_z_max_nm,
        carpet_nz=carpet_nz,
        scan_length_nm=scan_length_nm,
        bandwidth_nm=bandwidth_nm,
        spectrum=spectrum,
        spectral_bins=spectral_bins,
        resist_thickness_nm=resist_thickness_nm,
        resist_index=resist_index,
        absorption_per_nm=absorption_per_nm,
        nz=nz,
        exposure=exposure,
        dose_scale=dose_scale,
        dill_c=dill_c,
    )


def simulate_euv_il(
    grating_period_nm: float = 100.0,
    wavelength_nm: float = 13.5,
    *,
    order: int = 1,
    grating: str = "amplitude",
    duty_cycle: float = 0.5,
    phase_rad: float | None = None,
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
    """Two-grating EUV interference lithography.

    The +m order of one transmission grating and the -m order of a second
    grating of the same period ``p`` overlap at the wafer. With
    ``sin(theta) = m * lambda / p`` the fringe period is
    ``lambda / (2 sin(theta)) = p / (2 m)`` -- set by the grating period alone,
    independent of the wavelength and of the resist index.

    Args:
        grating_period_nm: Grating period ``p`` (nm).
        wavelength_nm: Vacuum wavelength (nm); ``m * lambda / p`` must be < 1.
        order: Diffraction order ``m`` (fringe period ``p / (2m)``).
        grating, duty_cycle, phase_rad: Grating design (as in
            :func:`simulate_talbot`, 1D gratings only) setting each beam's
            intensity to the m-th-order efficiency ``|c_m|^2``.
        intensity_ratio: Multiplier on beam 2 (imbalance; visibility
            ``2 sqrt(I1 I2) / (I1 + I2)``).
        n_medium: Resist refractive index (may be < 1 at EUV).
        absorption_per_nm: Resist intensity absorption coefficient (1/nm).
        n_fringes: Fringe periods spanned along x.
        nx, ny, nz: Voxel grid dimensions.
        z_span_nm: Resist depth (nm).
        dose_scale, dill_c: Dill map to PAC ``m = exp(-C * dose * I)``.

    Returns:
        EuvIlResult with ``fringe_period_nm``, ``diffraction_angle_deg``,
        ``visibility``, ``beam_intensities``, and the ``intensity`` / ``pac``
        volumes.
    """
    from highuvlith._validation import _validate_positive

    _validate_positive("grating_period_nm", grating_period_nm)
    _validate_positive("wavelength_nm", wavelength_nm)
    if order < 1:
        raise ValueError(f"order must be >= 1, got {order}")
    if order * wavelength_nm >= grating_period_nm:
        raise ValueError(
            f"order {order} is evanescent: m * lambda ({order * wavelength_nm}) must be "
            f"< grating_period_nm ({grating_period_nm})"
        )
    if grating not in ("amplitude", "phase", "sinusoidal"):
        raise ValueError(f"grating must be amplitude / phase / sinusoidal, got {grating!r}")
    if intensity_ratio < 0:
        raise ValueError(f"intensity_ratio must be >= 0, got {intensity_ratio}")
    _validate_positive("n_medium", n_medium)
    _validate_positive("z_span_nm", z_span_nm)

    return _native_simulate_euv_il(
        grating_period_nm=grating_period_nm,
        wavelength_nm=wavelength_nm,
        order=order,
        grating=grating,
        duty_cycle=duty_cycle,
        phase_rad=phase_rad,
        intensity_ratio=intensity_ratio,
        n_medium=n_medium,
        absorption_per_nm=absorption_per_nm,
        n_fringes=n_fringes,
        nx=nx,
        ny=ny,
        nz=nz,
        z_span_nm=z_span_nm,
        dose_scale=dose_scale,
        dill_c=dill_c,
    )


# ---------------------------------------------------------------------------
# Volumetric post-exposure bake and development
# ---------------------------------------------------------------------------

_LATERAL_BOUNDARIES = ("periodic", "reflecting")
_DEPLETION_MODELS = ("none", "exponential", "loading", "local_loading")


def _check_choice(name: str, value: str, choices: tuple[str, ...]) -> None:
    if value not in choices:
        raise ValueError(f"{name} must be one of {choices}, got {value!r}")


def develop_level_set(
    volume: VolumetricResult,
    resist: huv.ResistConfig,
    dev_time_s: float,
    *,
    surface_rate_ratio: float = 1.0,
    inhibition_depth_nm: float = 0.0,
    lateral: str = "periodic",
    depletion: str = "none",
    depletion_time_constant_s: float | None = None,
    loading_capacity_nm: float | None = None,
    loading_length_nm: float | None = None,
    cfl: float = 0.5,
    reinit_interval: int = 4,
    band_cells: float = 6.0,
    max_steps: int = 2_000_000,
) -> LevelSetResult:
    """Level-set (moving-boundary) development of a latent-image volume.

    The dissolution rate is re-evaluated at the moving front every step,
    ``R = R_bulk(m) * f_inh(depth) * g(t)``: ``R_bulk`` from the resist's
    Mack/threshold model, the surface-inhibition factor
    ``f_inh = 1 - (1 - surface_rate_ratio) * exp(-depth / inhibition_depth_nm)``
    (depth below the original top surface), and the developer factor ``g``.

    Args:
        volume: Latent image (PAC, or the CAR protected fraction), (nz, ny, nx).
        resist: Resist whose development model sets ``R_bulk``.
        dev_time_s: Development time (s).
        surface_rate_ratio: Relative rate at the top surface (1 = no inhibition).
        inhibition_depth_nm: Inhibition depth (nm; 0 = none).
        lateral: ``"periodic"`` (matches the FFT aerial image) or ``"reflecting"``.
        depletion: ``"none"``, ``"exponential"`` (``g = exp(-t/tau)``, needs
            ``depletion_time_constant_s``), ``"loading"`` (``g = 1 - h/h_cap``
            with the field-mean dissolved thickness ``h``, needs
            ``loading_capacity_nm``), or ``"local_loading"`` (``h`` smoothed
            over ``loading_length_nm``).
        cfl, reinit_interval, band_cells, max_steps: Solver controls.

    Returns:
        LevelSetResult with ``arrival_times`` (comparable to fast marching),
        the final signed distance ``phi``, ``height_map()``, ``developed()``
        and diagnostics. For static rates (``depletion="none"``) fast marching
        (:func:`highuvlith.develop_fast_marching`) gives the same fronts
        within grid error, faster.
    """
    _check_choice("lateral", lateral, _LATERAL_BOUNDARIES)
    _check_choice("depletion", depletion, _DEPLETION_MODELS)
    if not dev_time_s >= 0.0:
        raise ValueError(f"dev_time_s must be >= 0, got {dev_time_s}")
    return _native_develop_level_set(
        volume,
        resist,
        dev_time_s,
        surface_rate_ratio,
        inhibition_depth_nm,
        lateral,
        depletion,
        depletion_time_constant_s,
        loading_capacity_nm,
        loading_length_nm,
        cfl,
        reinit_interval,
        band_cells,
        max_steps,
    )


def peb_gaussian(
    volume: VolumetricResult,
    lateral_nm: float,
    *,
    vertical_nm: float | None = None,
    vertical_scale: np.ndarray | None = None,
    vertical_surface_ratio: float | None = None,
    vertical_decay_nm: float | None = None,
    lateral: str = "periodic",
) -> VolumetricResult:
    """Anisotropic post-exposure bake with exact Gaussian kernels.

    Lateral and vertical diffusion lengths ``sigma = sqrt(2 D t)`` are
    independent (``vertical_nm`` defaults to ``lateral_nm``); the top and
    bottom are zero-flux. A depth-dependent vertical diffusivity is given
    either as ``vertical_scale`` (length-nz array of ``D_z(z)/D_ref``) or
    parametrically as ``1 + (vertical_surface_ratio - 1) exp(-z /
    vertical_decay_nm)``. Returns a new volume.
    """
    _check_choice("lateral", lateral, _LATERAL_BOUNDARIES)
    if vertical_scale is not None:
        vertical_scale = np.ascontiguousarray(vertical_scale, dtype=np.float64)
    return _native_peb_gaussian(
        volume,
        lateral_nm,
        vertical_nm,
        vertical_scale,
        vertical_surface_ratio,
        vertical_decay_nm,
        lateral,
    )


def peb_car(
    volume: VolumetricResult,
    *,
    peb_time_s: float = 60.0,
    k_amp: float = 0.1,
    k_quench: float = 10.0,
    quencher: float = 0.15,
    acid_diffusivity_nm2_s: float = 2.0,
    quencher_diffusivity_nm2_s: float = 0.5,
    vertical_ratio: float = 1.0,
    lateral: str = "periodic",
    max_time_step_s: float | None = None,
) -> CarPebResult:
    """Chemically amplified resist (CAR) post-exposure bake.

    Standard Mack/PROLITH-class acid/quencher reaction-diffusion model with
    concentrations normalized to the photo-acid generator (PAG):
    ``dm/dt = -k_amp m h``, ``dh/dt = D_h lap(h) - k_q h q``,
    ``dq/dt = D_q lap(q) - k_q h q``. The input volume is the Dill latent
    image read as the remaining PAG fraction, so the photo-acid is
    ``h0 = 1 - volume``; ``m`` starts fully protected. Diffusivities are
    lateral, ``vertical_ratio = D_z / D_xy``. The default rates are
    illustrative, not fitted to a specific resist.

    Returns:
        CarPebResult whose ``protected`` volume feeds
        :func:`develop_level_set` / :func:`highuvlith.develop_fast_marching`.
    """
    _check_choice("lateral", lateral, _LATERAL_BOUNDARIES)
    return _native_peb_car(
        volume,
        peb_time_s,
        k_amp,
        k_quench,
        quencher,
        acid_diffusivity_nm2_s,
        quencher_diffusivity_nm2_s,
        vertical_ratio,
        lateral,
        max_time_step_s,
    )


def _volume_extents(
    vol: VolumetricResult,
) -> tuple[tuple[float, float], tuple[float, float], tuple[float, float]]:
    """(min, max) extents of a volume from its cell-centre coordinates."""

    def extent(c: np.ndarray, fallback: float) -> tuple[float, float]:
        step = float(c[1] - c[0]) if len(c) > 1 else fallback
        return (float(c[0]) - step / 2.0, float(c[-1]) + step / 2.0)

    x, y, z = vol.x_nm, vol.y_nm, vol.z_nm
    dx = float(x[1] - x[0]) if len(x) > 1 else 1.0
    return extent(x, dx), extent(y, dx), extent(z, 1.0)


@dataclass
class VolumetricProcessResult:
    """Exposure, post-exposure bake, and development through the resist."""

    latent: VolumetricResult
    """Dill latent image (PAC / remaining PAG) after exposure."""
    baked: VolumetricResult
    """Field the development rate consumes: the latent image after the
    Gaussian bake, or the CAR protected-site fraction."""
    arrival_times: VolumetricResult
    """Development-front arrival time (s) per voxel."""
    height_map: HeightMapResult
    """Remaining resist height (nm) at ``dev_time_s``."""
    developed_cd_nm: list[float | None]
    """Per-slice developed CD (centre row), top to bottom."""
    dev_time_s: float
    car: CarPebResult | None = None
    level_set: LevelSetResult | None = None
    notes: list[str] = field(default_factory=list)
    """Development diagnostics (same checks as the CLI ``deep`` volumetric
    mode): over-development when less than a quarter of the resist volume
    remains, under-development when nothing clears to the substrate."""


def _development_notes(
    height: np.ndarray, thickness_nm: float, dz_nm: float, dose_mj_cm2: float, dev_time_s: float
) -> list[str]:
    mean_h = float(np.mean(height))
    if mean_h < 0.25 * thickness_nm:
        return [
            f"over-developed? only {100.0 * mean_h / thickness_nm:.0f} % of the resist volume "
            f"remains (mean {mean_h:.1f} nm of {thickness_nm:.0f} nm) at {dose_mj_cm2:.1f} mJ/cm² "
            f"after {dev_time_s:.0f} s; unless the pattern is mostly clear, lower dose_mj_cm2 "
            "or dev_time_s"
        ]
    if float(np.min(height)) > dz_nm:
        return [
            f"under-developed: nothing clears to the substrate (thinnest remaining "
            f"{float(np.min(height)):.1f} nm) at {dose_mj_cm2:.1f} mJ/cm² after "
            f"{dev_time_s:.0f} s; raise dose_mj_cm2 or dev_time_s"
        ]
    return []


def simulate_volumetric(
    source: huv.SourceConfig,
    optics: huv.OpticsConfig,
    mask: huv.MaskConfig,
    film_stack: huv.FilmStackConfig,
    resist: huv.ResistConfig,
    grid: huv.GridConfig,
    *,
    dose_mj_cm2: float = 30.0,
    nz: int = 32,
    n_defocus_planes: int = 4,
    dose_steps: int = 1,
    base_defocus_nm: float = 0.0,
    max_kernels: int = 20,
    peb: str = "gaussian",
    peb_lateral_nm: float | None = None,
    peb_vertical_nm: float | None = None,
    car: dict[str, Any] | None = None,
    develop: str = "fmm",
    dev_time_s: float = 60.0,
    surface_rate_ratio: float = 1.0,
    inhibition_depth_nm: float = 0.0,
    lateral: str = "periodic",
    depletion: str = "none",
    depletion_time_constant_s: float | None = None,
    loading_capacity_nm: float | None = None,
    loading_length_nm: float | None = None,
) -> VolumetricProcessResult:
    """One call from mask to developed 3D resist profile.

    Runs :func:`expose_volumetric`, then the post-exposure bake ``peb``
    (``"none"``; ``"gaussian"`` with ``peb_lateral_nm`` / ``peb_vertical_nm``,
    lateral defaulting to the resist's ``peb_diffusion_nm``; or ``"car"`` with
    keyword arguments for :func:`peb_car` in ``car``), then development
    ``develop`` = ``"fmm"`` (static rate, fast) or ``"level_set"`` (moving
    boundary; required for ``depletion`` other than ``"none"``).

    Returns:
        VolumetricProcessResult with the latent, baked, and arrival-time
        volumes, the height map, and the per-slice developed CD.
    """
    _check_choice("peb", peb, ("none", "gaussian", "car"))
    _check_choice("develop", develop, ("fmm", "level_set"))
    _check_choice("lateral", lateral, _LATERAL_BOUNDARIES)
    if develop == "fmm" and depletion != "none":
        raise ValueError(
            "developer depletion needs develop='level_set' (fast marching assumes "
            "a static rate field)"
        )

    latent = expose_volumetric(
        source,
        optics,
        mask,
        film_stack,
        resist,
        grid,
        dose_mj_cm2=dose_mj_cm2,
        nz=nz,
        n_defocus_planes=n_defocus_planes,
        dose_steps=dose_steps,
        base_defocus_nm=base_defocus_nm,
        max_kernels=max_kernels,
    )

    car_state: CarPebResult | None = None
    if peb == "gaussian":
        lateral_nm = resist.peb_diffusion_nm if peb_lateral_nm is None else peb_lateral_nm
        baked = peb_gaussian(latent, lateral_nm, vertical_nm=peb_vertical_nm, lateral=lateral)
    elif peb == "car":
        car_state = peb_car(latent, lateral=lateral, **(car or {}))
        baked = car_state.protected
    else:
        baked = latent

    level_set: LevelSetResult | None = None
    if develop == "level_set":
        level_set = develop_level_set(
            baked,
            resist,
            dev_time_s,
            surface_rate_ratio=surface_rate_ratio,
            inhibition_depth_nm=inhibition_depth_nm,
            lateral=lateral,
            depletion=depletion,
            depletion_time_constant_s=depletion_time_constant_s,
            loading_capacity_nm=loading_capacity_nm,
            loading_length_nm=loading_length_nm,
        )
        times = level_set.arrival_times
    else:
        x, z = baked.x_nm, baked.z_nm
        pixel_xy = float(x[1] - x[0]) if len(x) > 1 else grid.pixel_nm
        pixel_z = float(z[1] - z[0]) if len(z) > 1 else float(2.0 * z[0])
        times = _native_develop_fast_marching(
            baked,
            resist,
            pixel_xy,
            pixel_z,
            surface_rate_ratio,
            inhibition_depth_nm,
            lateral,
        )

    developed = (np.asarray(times.values) <= dev_time_s).astype(np.float64)
    indicator = VolumetricResult.from_array(developed, *_volume_extents(times))
    height_map = huv.height_map_from_times(times, dev_time_s)
    z = np.asarray(times.z_nm)
    dz = float(z[1] - z[0]) if len(z) > 1 else float(2.0 * z[0])
    return VolumetricProcessResult(
        latent=latent,
        baked=baked,
        arrival_times=times,
        height_map=height_map,
        developed_cd_nm=indicator.cd_at_z(0.5),
        dev_time_s=dev_time_s,
        car=car_state,
        level_set=level_set,
        notes=_development_notes(
            np.asarray(height_map.values), len(z) * dz, dz, dose_mj_cm2, dev_time_s
        ),
    )


# --- Illumination comparison, process window, source landscape, throughput --


@dataclass
class IlluminationComparison:
    """One pupil fill of :func:`compare_illumination`."""

    name: str
    illumination: tuple[Any, ...]
    #: Image contrast (I_max − I_min)/(I_max + I_min) at ``focus_nm``.
    contrast: float
    #: Printed dark-line width at the 0.3 intensity contour (None if none prints).
    cd_nm: float | None
    #: NILS of the dark line at the 0.3 contour (None if none prints).
    nils: float | None
    #: Focus planes (nm) and the contrast at each.
    focuses_nm: np.ndarray
    contrast_through_focus: np.ndarray
    status: str = "✅"


def compare_illumination(
    cd_nm: float,
    pitch_nm: float,
    illuminations: dict[str, tuple[Any, ...]] | None = None,
    *,
    source: huv.SourceConfig | None = None,
    wavelength_nm: float = 157.63,
    na: float = 0.75,
    immersion: bool | float = False,
    imaging: str = "scalar",
    polarization: str = "unpolarized",
    focus_nm: float = 0.0,
    focuses_nm: list[float] | None = None,
    grid_size: int = 128,
    pixel_nm: float = 2.0,
    max_kernels: int = 20,
) -> list[IlluminationComparison]:
    """Image one line/space pattern under several pupil fills.

    ``illuminations`` maps a name to a ``SourceConfig.with_illumination``
    spec; the default compares conventional σ 0.7, annular 0.5–0.8, an
    x-dipole (poles at σ 0.7 on ±x, radius 0.2) and a quadrupole (poles at
    σ 0.7 on ±x and ±y, radius 0.2). Every fill shares the source spectrum (``source``,
    or a generic one at ``wavelength_nm``), optics, mask and commensurate
    grid. Each entry reports contrast, printed CD and NILS (0.3 threshold,
    no resist) at ``focus_nm`` and the contrast through ``focuses_nm``
    (default −200…200 nm in 50 nm steps). Scalar Hopkins imaging is ✅;
    ``imaging="vector"`` adds polarization (also ✅).
    """
    from highuvlith._validation import _validate_positive, _validate_power_of_two

    _validate_positive("cd_nm", cd_nm)
    _validate_positive("pitch_nm", pitch_nm)
    if cd_nm >= pitch_nm:
        raise ValueError(f"cd_nm ({cd_nm}) must be less than pitch_nm ({pitch_nm})")
    _validate_power_of_two("grid_size", grid_size)
    _validate_positive("pixel_nm", pixel_nm)
    if illuminations is None:
        illuminations = {
            "conventional σ0.7": ("conventional", 0.7),
            "annular 0.5–0.8": ("annular", 0.5, 0.8),
            "dipole-x 0.7/0.2": ("dipole", 0.7, 0.2, 0.0),
            "quadrupole x/y 0.7/0.2": ("quadrupole", 0.7, 0.2, 45.0),
        }
    if not illuminations:
        raise ValueError("illuminations must name at least one pupil fill")
    if focuses_nm is None:
        focuses_nm = [-200.0, -150.0, -100.0, -50.0, 0.0, 50.0, 100.0, 150.0, 200.0]
    optics, _index = _imaging_optics(na, immersion)
    vector = _vector_settings(imaging, polarization, 0.0)
    base = source if source is not None else huv.SourceConfig(wavelength_nm=wavelength_nm)
    mask = huv.MaskConfig.line_space(cd_nm=cd_nm, pitch_nm=pitch_nm)
    grid = mask.commensurate_grid(size=grid_size, target_pixel_nm=pixel_nm)
    rows = []
    for name, spec in illuminations.items():
        src = base.with_illumination(tuple(spec))
        engine = huv.SimulationEngine(src, optics, mask, grid=grid, max_kernels=max_kernels, vector=vector)
        aerial = engine.compute_aerial_image(focus_nm=focus_nm)
        through = engine.compute_through_focus(list(focuses_nm))
        rows.append(
            IlluminationComparison(
                name=name,
                illumination=src.illumination,
                contrast=aerial.image_contrast(),
                cd_nm=aerial.cd(threshold=0.3, tone="dark"),
                nils=aerial.nils_periodic(threshold=0.3, tone="dark"),
                focuses_nm=np.asarray(focuses_nm, dtype=float),
                contrast_through_focus=np.array([img.image_contrast() for img in through]),
            )
        )
    return rows


def process_window(
    cd_nm: float,
    pitch_nm: float,
    *,
    source: huv.SourceConfig | None = None,
    wavelength_nm: float = 157.63,
    na: float = 0.75,
    sigma: float = 0.7,
    illumination: tuple[Any, ...] | None = None,
    immersion: bool | float = False,
    dose_to_clear_mj_cm2: float | None = None,
    doses_mj_cm2: list[float] | None = None,
    focuses_nm: list[float] | None = None,
    cd_tolerance_pct: float = 10.0,
    grid_size: int = 128,
    pixel_nm: float = 2.0,
    max_kernels: int = 20,
) -> huv.ProcessWindowResult:
    """Focus–exposure (ED) process window of a line/space pattern (🔶).

    A constant-threshold resist clears where ``dose · I ≥ E_th``
    (``dose_to_clear_mj_cm2``). Without ``E_th``, it is chosen so the line
    prints at ``cd_nm`` in focus at 30 mJ/cm² (``E_th`` = 30 mJ/cm² × the
    in-focus dose-to-size intensity threshold, ``dose_to_size_threshold``). ``doses_mj_cm2`` defaults to 21 doses over 30 mJ/cm² ±20 %
    and ``focuses_nm`` to 17 planes over ±200 nm. The result carries the
    Bossung curves, EL-vs-DOF, DOF at x % EL, best focus, dose-to-size and
    ``.status`` / ``.notes`` (no blur, diffusion or development; rectangular
    windows; CD on the y = 0 cut).
    """
    from highuvlith._validation import _validate_positive, _validate_power_of_two, _validate_range

    _validate_positive("cd_nm", cd_nm)
    _validate_positive("pitch_nm", pitch_nm)
    if cd_nm >= pitch_nm:
        raise ValueError(f"cd_nm ({cd_nm}) must be less than pitch_nm ({pitch_nm})")
    _validate_range("sigma", sigma, 0.01, 1.0)
    _validate_power_of_two("grid_size", grid_size)
    _validate_positive("pixel_nm", pixel_nm)
    optics, _index = _imaging_optics(na, immersion)
    if source is None:
        source = huv.SourceConfig(wavelength_nm=wavelength_nm, sigma_outer=sigma)
    if illumination is not None:
        source = source.with_illumination(tuple(illumination))
    mask = huv.MaskConfig.line_space(cd_nm=cd_nm, pitch_nm=pitch_nm)
    grid = mask.commensurate_grid(size=grid_size, target_pixel_nm=pixel_nm)
    nominal = 30.0
    if dose_to_clear_mj_cm2 is None:
        threshold = _native_dose_to_size(
            source, optics, mask, grid, cd_nm, half_width_nm=0.5 * pitch_nm, max_kernels=max_kernels
        )
        dose_to_clear_mj_cm2 = threshold * nominal
    else:
        _validate_positive("dose_to_clear_mj_cm2", dose_to_clear_mj_cm2)
    if doses_mj_cm2 is None:
        doses_mj_cm2 = list(np.linspace(0.8 * nominal, 1.2 * nominal, 21))
    if focuses_nm is None:
        focuses_nm = list(np.linspace(-200.0, 200.0, 17))
    batch = huv.BatchSimulator(source, optics, mask, grid, max_kernels=max_kernels)
    return batch.process_window_threshold(
        doses=[float(d) for d in doses_mj_cm2],
        focuses=[float(f) for f in focuses_nm],
        dose_to_clear_mj_cm2=float(dose_to_clear_mj_cm2),
        cd_target_nm=cd_nm,
        cd_tolerance_pct=cd_tolerance_pct,
    )


@dataclass(frozen=True)
class SourcePreset:
    """One row of :func:`source_landscape`."""

    label: str
    #: ``SourceConfig`` factory call that builds it, e.g. ``"lpp_sn_13nm5()"``.
    factory: str
    #: Source family (``SourceConfig.kind``).
    family: str
    #: Trait wavelength (nm): the line centre, or hc/⟨E⟩ for broadband sources.
    wavelength_nm: float
    bandwidth_pm: float
    photon_energy_ev: float
    #: The model's ``average_power_w`` (W); None where not modeled.
    average_power_w: float | None
    #: Where/how that power is defined (plasma sources: in band at IF; lasers: output; tubes: 4π X-ray power).
    power_definition: str
    #: "demonstrated" | "projection" | "theoretical" (machine maturity, not model status).
    maturity: str
    #: Capability badge of the source MODEL (docs/capability-matrix.md).
    status: str
    #: True for broadband sources (X-ray tube, betatron: ``wavelength_nm`` is a mean).
    broadband: bool = False
    note: str = ""
    #: ``derived_quantities()`` as {name: (value, unit)}.
    derived: dict[str, tuple[float, str]] = field(default_factory=dict)


#: (label, factory name, kwargs, maturity, model status, broadband, power definition, note)
_LANDSCAPE: tuple[tuple[str, str, dict[str, Any], str, str, bool, str, str], ...] = (
    ("Hg g-line", "hg_g_line", {}, "demonstrated", "✅/🔶", False, "CW lamp (power not modeled)", "heritage"),
    ("Hg h-line", "hg_h_line", {}, "demonstrated", "✅/🔶", False, "CW lamp (power not modeled)", "heritage"),
    ("Hg i-line", "hg_i_line", {}, "demonstrated", "✅/🔶", False, "CW lamp (power not modeled)", "heritage"),
    ("KrF", "krf_laser", {}, "demonstrated", "✅/🔶", False, "laser output", "representative of the 30–60 W vendor range"),
    ("ArF", "arf_laser", {}, "demonstrated", "✅/🔶", False, "laser output", "representative of the 60–120 W vendor range"),
    ("F2", "f2_laser", {}, "demonstrated", "✅", False, "laser output", "157 nm program dropped 2003"),
    ("Ar2", "ar2_laser", {}, "theoretical", "🧪", False, "laser output", "hypothetical preset"),
    ("LPA-FEL 25 nm", "lpa_fel_bella_25nm", {}, "projection", "🔶", False, "FEL output", "design projection; demonstrated LPA-FEL: 420 nm at 1 Hz"),
    ("Sn LPP 250 W", "lpp_sn_13nm5", {}, "demonstrated", "✅/🔶", False, "in band (2 %) at IF", "NXE:3400B chain"),
    ("Sn LPP 500 W", "lpp_sn_13nm5_500w", {}, "demonstrated", "✅/🔶", False, "in band (2 %) at IF", "NXE:3800E power class; drive chain assumed"),
    ("Tb LPP 6.5 nm", "lpp_tb_6nm5", {}, "projection", "🧪", False, "in band at IF", "lab CE, optimistic collection"),
    ("Gd LPP 6.7 nm", "lpp_gd_6nm7", {}, "projection", "🧪", False, "in band at IF", "lab CE, optimistic collection"),
    ("Sn DPP", "dpp_sn_13nm5", {}, "demonstrated", "🔶", False, "in band at IF (360 W into 2π at source)", "TRINITI 2010 level"),
    ("Xe DPP", "dpp_xe_13nm5", {}, "demonstrated", "🔶", False, "in band at IF (10 W into 2π at source)", "EQ-10 class"),
    ("Compact undulator", "synchrotron_compact_euv", {}, "projection", "✅/🔶", False, "central-cone line power", "illustrative compact ring, 200 mA"),
    ("HHG Ne 13.5 nm", "hhg_ne_13nm5", {}, "demonstrated", "✅/🔶", False, "harmonic power", "assumed 20 W driver × conversion efficiency"),
    ("HHG Ar 30 nm", "hhg_ar_30nm", {}, "demonstrated", "✅/🔶", False, "harmonic power", "assumed 3 W driver × conversion efficiency"),
    ("FLASH-like XFEL", "xfel_flash_13nm5", {}, "demonstrated", "✅", False, "FEL output", "FLASH measured 0.02 W at 13.7 nm"),
    ("FERMI-like seeded", "xfel_fermi_seeded", {}, "demonstrated", "✅", False, "FEL output", ""),
    ("CW-SC XFEL", "xfel_cw_sc_13nm5", {}, "projection", "🧪", False, "FEL output", "design projection"),
    ("ERL FEL", "xfel_erl_13nm5", {}, "projection", "🧪", False, "FEL output", "KEK design class"),
    ("ICS", "ics_compact_euv_13nm5", {}, "projection", "🧪", False, "in band", "aggressive design point"),
    ("SSMB", "ssmb_euv_13nm5", {}, "projection", "🧪", False, "coherent radiation", "requires bunching factor ≈ 0.146"),
    ("SXRL Ar 46.9 nm", "sxrl_ar_46nm9", {}, "demonstrated", "✅/🔶", False, "laser output", "13 µJ × 12 Hz (Heinbuch 2005)"),
    ("SXRL Ag 13.9 nm", "sxrl_ag_13nm9", {}, "demonstrated", "✅/🔶", False, "laser output", "reported ~0.1 mW order"),
    ("W X-ray tube", "xray_tube", {"anode": "W"}, "demonstrated", "🔶/✅", True, "filtered X-ray power, 4π-equivalent", "LIGA / proximity source"),
    ("Cu X-ray tube", "xray_tube", {"anode": "Cu"}, "demonstrated", "🔶/✅", True, "filtered X-ray power, 4π-equivalent", "LIGA / proximity source"),
    ("Betatron", "betatron", {}, "demonstrated", "🧪", True, "all photon energies, 10 Hz", "LWFA betatron X-rays"),
    ("Smith–Purcell EUV", "smith_purcell", {}, "theoretical", "🧪", False, "estimate, coupling ε = 1e-3", "emission demonstrated only to ~230 nm"),
    ("Entangled NOON", "entangled_noon", {}, "theoretical", "🧪", False, "not modeled as power", "see quantum_flux_budget"),
)


def source_landscape() -> list[SourcePreset]:
    """Every source preset's wavelength, power, maturity and model status.

    Values are computed live from the ``SourceConfig`` factories
    (``average_power_w``, ``derived_quantities()``); ``maturity`` (machine)
    and ``status`` (model) are curated labels from the capability matrix.
    Powers follow each family's own ``average_power_w`` definition
    (``power_definition``), so they are not all at the same plane. Feed the
    list to :func:`highuvlith.viz.plot_source_landscape`.
    """
    rows = []
    for label, factory, kwargs, maturity, status, broadband, power_def, note in _LANDSCAPE:
        src = getattr(huv.SourceConfig, factory)(**kwargs)
        args = ", ".join(f"{k}={v!r}" for k, v in kwargs.items())
        rows.append(
            SourcePreset(
                label=label,
                factory=f"{factory}({args})",
                family=src.kind,
                wavelength_nm=src.wavelength_nm,
                bandwidth_pm=src.bandwidth_pm,
                photon_energy_ev=src.photon_energy_ev,
                average_power_w=src.average_power_w,
                power_definition=power_def,
                maturity=maturity,
                status=status,
                broadband=broadband,
                note=note,
                derived={name: (value, unit) for name, value, unit, _n in src.derived_quantities()},
            )
        )
    return rows


# Wavelength bands (nm) in which each throughput preset's optics train is a
# real projection-lithography train: refractive CaF2/MgF2/LiF optics transmit
# down to ~105 nm; Mo/Si multilayers reflect from the Si L-edge (12.4 nm) to
# ~15 nm; La/B multilayers work just above the B K-edge (6.6 nm).
_THROUGHPUT_PRESET_BANDS_NM: dict[str, tuple[float, float]] = {
    "refractive": (100.0, float("inf")),
    "euv": (12.4, 15.0),
    "beuv": (6.0, 7.5),
}
_THROUGHPUT_PRESET_ALIASES = {"euv_hvm": "euv", "beuv_la_b": "beuv"}


def _throughput_preset_for(wavelength_nm: float) -> str | None:
    for name, (lo, hi) in _THROUGHPUT_PRESET_BANDS_NM.items():
        if lo <= wavelength_nm <= hi:
            return name
    return None


def wafer_throughput(
    source: huv.SourceConfig,
    *,
    dose_mj_cm2: float = 30.0,
    preset: str = "auto",
    **overrides: Any,
) -> dict[str, Any] | None:
    """Dose-limited wafers per hour for ``source`` (🔶 simplified scanner model).

    ``preset="auto"`` picks the optics-train assumptions from the
    wavelength: "refractive" above 100 nm (optics 0.30, mask 0.90, 8 mm
    slit), "euv" for 12.4–15 nm (10 Mo/Si mirrors at R 0.70, mask 0.65),
    "beuv" for 6–7.5 nm (La/B mirrors). Any other wavelength has no
    projection-lithography optics preset (X-ray tubes and the betatron at
    0.15–0.5 nm, the 25–47 nm LPA-FEL / Ar HHG / Ar soft-X-ray-laser lines),
    so ``"auto"`` raises ``ValueError`` instead of quoting an EUV-optics
    figure, unless the optics train is given (``optics_transmission`` or
    ``n_mirrors`` + ``mirror_reflectivity``; the nearest preset then supplies
    the mask and field parameters, and a note says so). An explicit
    ``preset=`` is honoured at any wavelength (a hypothetical train); outside
    the preset's band a note says so. The CLI ``throughput`` subcommand uses
    the same bands.
    ``overrides`` are the keyword arguments of ``SourceConfig.wafer_throughput``
    (``optics_transmission``, ``n_mirrors``, ``field_width_mm``,
    ``max_scan_speed_mm_s``, ...). Returns the native dict plus ``status``
    and ``notes``, or None when the source models no average power (Hg lamps,
    the entangled-photon source).
    """
    wl = source.wavelength_nm
    notes = [
        "single optics-train transmission; lumped field and wafer overheads; optional stage limit",
        "source power per the family's average_power_w definition (plasma sources at IF)",
    ]
    if preset == "auto":
        if source.average_power_w is None:
            return None
        chosen = _throughput_preset_for(wl)
        optics_given = any(
            k in overrides for k in ("optics_transmission", "n_mirrors", "mirror_reflectivity")
        )
        if chosen is None and optics_given:
            # Explicit optics train: the nearest preset only supplies the
            # mask and field parameters (as the CLI does).
            chosen = "refractive" if wl >= 100.0 else ("beuv" if wl < 10.0 else "euv")
            notes.append(
                f"no optics preset covers {wl:.4g} nm: the given optics train is used with "
                f"the '{chosen}' preset's mask and field parameters"
            )
        elif chosen is None:
            raise ValueError(
                f"no projection-lithography optics preset covers {wl:.4g} nm "
                f"({source.kind} source): refractive > 100 nm, euv 12.4–15 nm, "
                "beuv 6–7.5 nm. Pass preset='euv'|'beuv'|'refractive' (and e.g. "
                "optics_transmission=) explicitly for a hypothetical optics train."
            )
        preset = chosen
    else:
        band = _THROUGHPUT_PRESET_BANDS_NM.get(_THROUGHPUT_PRESET_ALIASES.get(preset, preset))
        if band is not None and not (band[0] <= wl <= band[1]):
            notes.append(
                f"hypothetical: {wl:.4g} nm is outside the '{preset}' optics band "
                f"({band[0]:g}–{band[1]:g} nm); the train's transmission is not physical here"
            )
    result = source.wafer_throughput(dose_mj_cm2=dose_mj_cm2, preset=preset, **overrides)
    if result is None:
        return None
    result["status"] = "🔶"
    result["notes"] = notes
    return result
