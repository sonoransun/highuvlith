"""Integration tests for the volumetric / deep-layer Python bindings:
volumetric exposure + 3D development, LIGA deep-X-ray, grayscale round trip,
multi-beam interference, and the quantum N-photon aerial image."""

import numpy as np
import pytest

import highuvlith as huv


# ---------------------------------------------------------------------------
# Shared small volumetric exposure (built once; engine TCC is the cost).
# ---------------------------------------------------------------------------
@pytest.fixture(scope="module")
def volumetric_setup():
    source = huv.SourceConfig.f2_laser(sigma=0.7)
    optics = huv.OpticsConfig(numerical_aperture=0.75)
    mask = huv.MaskConfig.line_space(cd_nm=65.0, pitch_nm=180.0)
    grid = huv.GridConfig(size=32, pixel_nm=8.0)

    # Index-matched substrate -> no bottom reflection -> clean Beer-Lambert
    # decay through the (deliberately absorbing) resist, so deeper slices are
    # unambiguously less exposed.
    film = huv.FilmStackConfig()
    film.set_substrate(1.65, 0.02)
    resist = huv.ResistConfig(thickness_nm=150.0, dill_a=0.5, dill_b=3.0, dill_c=0.03)
    nz = 8
    thickness_nm = 150.0

    volume = huv.expose_volumetric(
        source,
        optics,
        mask,
        film,
        resist,
        grid,
        dose_mj_cm2=30.0,
        nz=nz,
        n_defocus_planes=2,
        max_kernels=8,
    )
    return {
        "volume": volume,
        "resist": resist,
        "nz": nz,
        "nx": 32,
        "ny": 32,
        "pixel_nm": 8.0,
        "thickness_nm": thickness_nm,
    }


# ---------------------------------------------------------------------------
# Construction / shape / dtype.
# ---------------------------------------------------------------------------
def test_volumetric_result_shape_and_coords(volumetric_setup):
    vol = volumetric_setup["volume"]
    nz, ny, nx = volumetric_setup["nz"], volumetric_setup["ny"], volumetric_setup["nx"]

    values = vol.values
    assert values.shape == (nz, ny, nx)
    assert values.dtype == np.float64
    assert vol.shape == (nz, ny, nx)

    assert vol.x_nm.shape == (nx,)
    assert vol.y_nm.shape == (ny,)
    assert vol.z_nm.shape == (nz,)
    # z runs from the resist top downward, strictly increasing.
    assert np.all(np.diff(vol.z_nm) > 0)


def test_height_map_from_depth_map(volumetric_setup):
    vol = volumetric_setup["volume"]
    hm = vol.depth_map(threshold=0.5)
    assert isinstance(hm, huv.HeightMapResult)
    assert hm.values.shape == (volumetric_setup["ny"], volumetric_setup["nx"])
    assert hm.x_nm.shape == (volumetric_setup["nx"],)
    assert hm.y_nm.shape == (volumetric_setup["ny"],)
    # Development depth cannot exceed the resist thickness.
    assert np.all(hm.values >= 0.0)
    assert np.all(hm.values <= volumetric_setup["thickness_nm"] + 1e-9)


# ---------------------------------------------------------------------------
# Volumetric exposure physics.
# ---------------------------------------------------------------------------
def test_expose_volumetric_bounds_and_depth_trend(volumetric_setup):
    values = volumetric_setup["volume"].values
    # PAC concentration stays in [0, 1].
    assert np.all(values >= 0.0)
    assert np.all(values <= 1.0)

    # Deeper slices are less exposed (higher PAC) for an absorbing stack:
    # the per-slice mean PAC increases monotonically with depth.
    slice_means = values.mean(axis=(1, 2))
    assert slice_means[0] < slice_means[-1]
    assert np.all(np.diff(slice_means) >= -1e-9)


def test_cd_at_z_returns_per_slice(volumetric_setup):
    vol = volumetric_setup["volume"]
    cds = vol.cd_at_z(threshold=0.5)
    assert isinstance(cds, list)
    assert len(cds) == volumetric_setup["nz"]
    # Entries are either None or a positive width.
    for c in cds:
        assert c is None or c > 0.0


# ---------------------------------------------------------------------------
# Fast-marching development + height map.
# ---------------------------------------------------------------------------
def test_fast_marching_and_height_map(volumetric_setup):
    vol = volumetric_setup["volume"]
    resist = volumetric_setup["resist"]
    nz, ny, nx = volumetric_setup["nz"], volumetric_setup["ny"], volumetric_setup["nx"]
    thickness = volumetric_setup["thickness_nm"]

    times = huv.develop_fast_marching(
        vol,
        resist,
        pixel_xy_nm=volumetric_setup["pixel_nm"],
        pixel_z_nm=thickness / nz,
    )
    assert times.shape == (nz, ny, nx)
    t = times.values
    # Every voxel is reached (connected grid) with a finite, non-negative time.
    assert np.all(np.isfinite(t))
    assert np.all(t >= 0.0)

    hm = huv.height_map_from_times(times, dev_time_s=2.0)
    assert isinstance(hm, huv.HeightMapResult)
    assert hm.values.shape == (ny, nx)
    assert np.all(hm.values >= 0.0)
    assert np.all(hm.values <= thickness + 1e-9)


# ---------------------------------------------------------------------------
# LIGA deep-X-ray shadow printing.
# ---------------------------------------------------------------------------
def test_simulate_liga_smoke():
    result = huv.simulate_liga(
        critical_energy_kev=6.23,
        resist_thickness_um=200.0,
        cd_nm=800.0,
        pitch_nm=1600.0,
        grid_size=32,
        pixel_nm=100.0,
        nz=16,
    )
    # Hard X-rays are attenuated with depth, so the top is over-dosed relative
    # to the just-cleared bottom: the contrast ratio exceeds 1.
    assert result.dose_ratio > 1.0
    assert result.top_dose_kj_cm3 > result.bottom_dose_kj_cm3

    z_um, dose = result.depth_dose
    assert z_um.shape == dose.shape
    assert z_um[0] == pytest.approx(0.0, abs=1e-9)
    # Absorbed dose decreases monotonically into the depth.
    assert dose[0] > dose[-1]
    assert np.all(np.diff(dose) <= 1e-9)

    # Volumetric dose field and developed-depth map are wired through.
    assert result.volume.shape == (16, 32, 32)
    assert result.developed_depth.values.shape == (32, 32)


# ---------------------------------------------------------------------------
# Grayscale round trip: blazed grating -> transmittance -> printed height.
# ---------------------------------------------------------------------------
def test_grayscale_blazed_round_trip():
    n = 64
    period_px = 32
    thickness = 200.0
    target = huv.blazed_grating(n, period_px, depth_nm=120.0, thickness_nm=thickness)
    assert target.shape == (n, n)

    result = huv.simulate_grayscale(
        target,
        thickness_nm=thickness,
        d_th=10.0,
        d_clear=100.0,
        exposure_dose_mj_cm2=100.0,
        pixel_nm=8.0,
    )
    printed = result.height_map.values
    assert printed.shape == (n, n)

    # Printed relief tracks the target (both low where much resist is removed).
    corr = np.corrcoef(target.ravel(), printed.ravel())[0, 1]
    assert corr > 0.5

    # Within a period the sawtooth ramps down: the per-period-averaged center
    # profile has a clear negative slope along x.
    center = printed[n // 2, :]
    profile = center.reshape(n // period_px, period_px).mean(axis=0)
    slope_corr = np.corrcoef(np.arange(period_px), profile)[0, 1]
    assert slope_corr < -0.7


def test_grayscale_transmittance_helpers():
    n = 32
    target = huv.microlens_array(n, pitch_px=16, sag_nm=80.0, thickness_nm=200.0)
    assert target.shape == (n, n)

    t = huv.grayscale_transmittance_for_target(
        target, thickness_nm=200.0, d_th=10.0, d_clear=100.0, exposure_dose=200.0
    )
    assert t.shape == (n, n)
    # Intensity transmittance stays physical.
    assert np.all(t >= 0.0)
    assert np.all(t <= 1.0 + 1e-9)


# ---------------------------------------------------------------------------
# Multi-beam interference: two-beam fringe period.
# ---------------------------------------------------------------------------
def test_interference_two_beam_period():
    wavelength = 200.0
    half_angle_deg = 30.0
    x_span = 1000.0
    nx = 128
    vol = huv.simulate_interference(
        "two_beam",
        wavelength_nm=wavelength,
        n_medium=1.0,
        half_angle_deg=half_angle_deg,
        nx=nx,
        ny=4,
        nz=4,
        x_span_nm=x_span,
        y_span_nm=32.0,
        z_span_nm=64.0,
        dose_scale=1.0,
        dill_c=0.01,
    )
    assert vol.shape == (4, 4, nx)

    # Period from the dominant FFT mode of a single row. Expected fringe
    # period is lambda / (2 sin theta_air) = 200 / (2 * 0.5) = 200 nm.
    row = np.asarray(vol.values[0, 0, :], dtype=float)
    row = row - row.mean()
    spectrum = np.abs(np.fft.rfft(row))
    k = int(np.argmax(spectrum[1:])) + 1
    measured_period = x_span / k
    assert measured_period == pytest.approx(200.0, abs=10.0)


def test_interference_two_photon_less_consumed():
    kwargs = dict(
        wavelength_nm=200.0,
        n_medium=1.5,
        half_angle_deg=20.0,
        nx=16,
        ny=16,
        nz=4,
        x_span_nm=400.0,
        y_span_nm=400.0,
        z_span_nm=50.0,
        dose_scale=2.0,
        dill_c=0.5,
    )
    one = huv.simulate_interference("three_beam_hex", two_photon=False, **kwargs)
    two = huv.simulate_interference("three_beam_hex", two_photon=True, **kwargs)
    # Two-photon (I^2) kinetics sharpen the lattice: bright antinodes (I > 1)
    # are consumed more completely, so the minimum PAC is lower than one-photon.
    assert two.values.min() < one.values.min()


# ---------------------------------------------------------------------------
# Quantum N-photon aerial image compose.
# ---------------------------------------------------------------------------
def test_quantum_line_space_sharpens():
    result = huv.simulate_quantum_line_space(
        cd_nm=65.0,
        pitch_nm=180.0,
        n=2,
        fidelity=1.0,
        grid_size=128,
        pixel_nm=2.0,
        max_kernels=12,
    )
    assert result.classical.shape == result.quantum.shape
    # N-photon absorption raises image contrast over the classical image.
    assert result.quantum_contrast > result.classical_contrast
