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
    mask = huv.MaskConfig.line_space(cd_nm=64.0, pitch_nm=128.0)
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


# ---------------------------------------------------------------------------
# Talbot / EUV-IL
# ---------------------------------------------------------------------------
from highuvlith.api import simulate_euv_il, simulate_talbot  # noqa: E402


def _dominant_period(row, pixel_nm):
    row = np.asarray(row, dtype=float)
    spectrum = np.abs(np.fft.rfft(row - row.mean()))
    k = int(np.argmax(spectrum[1:])) + 1
    return len(row) * pixel_nm / k


def test_talbot_lengths_and_paraxial_revival():
    p, lam = 100.0, 13.5
    res = simulate_talbot(
        lam, p, propagation="paraxial", max_order=9, nx=64, n_periods=1, carpet_nz=64
    )
    assert res.talbot_length_nm == pytest.approx(2 * p * p / lam, rel=1e-12)
    exact = lam / (1 - np.sqrt(1 - (lam / p) ** 2))
    assert res.talbot_length_exact_nm == pytest.approx(exact, rel=1e-12)
    carpet = res.carpet
    assert carpet.shape == (64, 64)
    # Default carpet depth is 2 z_T over 64 rows: rows 32 apart are one Talbot
    # length apart and identical (paraxial self-imaging).
    z = res.carpet_z_nm
    assert z[37] - z[5] == pytest.approx(res.talbot_length_nm, rel=1e-12)
    np.testing.assert_allclose(carpet[5], carpet[37], atol=1e-9)
    # ... and half a Talbot length shifts the pattern by half a period.
    np.testing.assert_allclose(carpet[5 + 16], np.roll(carpet[5], 32), atol=1e-9)


def test_talbot_dtl_prints_half_period():
    p = 100.0
    res = simulate_talbot(13.5, p, duty_cycle=0.3, max_order=7, nx=64, carpet_nz=8)
    pixel = res.x_nm[1] - res.x_nm[0]
    stationary = res.stationary_image[0]
    assert _dominant_period(stationary, pixel) == pytest.approx(p / 2, rel=1e-9)
    # One-z_T scan with exact propagation is close to the stationary limit.
    assert np.max(np.abs(res.dtl_image[0] - stationary)) < 0.05
    eff = {(n, m): e for n, m, e in res.efficiencies}
    assert eff[(0, 0)] == pytest.approx(0.09, rel=1e-12)  # c0 = open fraction


def test_talbot_atl_converges_beyond_achromatic_distance():
    p, lam, bw = 100.0, 13.5, 0.5
    kwargs = dict(bandwidth_nm=bw, max_order=7, nx=32, n_periods=1, carpet_nz=4)
    res = simulate_talbot(lam, p, **kwargs)
    assert res.achromatic_distance_nm == pytest.approx(2 * p * p / bw, rel=1e-12)
    assert res.gap_nm == pytest.approx(2 * res.achromatic_distance_nm, rel=1e-12)
    np.testing.assert_allclose(res.atl_image[0], res.stationary_image[0], atol=2e-3)
    near = simulate_talbot(lam, p, gap_nm=0.05 * res.achromatic_distance_nm, **kwargs)
    assert np.max(np.abs(near.atl_image[0] - near.stationary_image[0])) > 0.05


def test_talbot_resist_volume_feeds_development():
    res = simulate_talbot(
        13.5,
        100.0,
        max_order=5,
        nx=32,
        carpet_nz=4,
        resist_thickness_nm=60.0,
        resist_index=0.97,
        absorption_per_nm=0.004,
        nz=6,
        exposure="stationary",
        dose_scale=20.0,
        dill_c=0.1,
    )
    assert res.intensity_volume.shape == (6, 1, 32)
    pac = res.pac_volume.values
    assert np.all((pac >= 0.0) & (pac <= 1.0))
    depth = res.pac_volume.depth_map(0.5).values
    assert depth.max() > depth.min()


def test_talbot_hex_hole_array_is_2d():
    res = simulate_talbot(
        13.5,
        100.0,
        grating="holes_hex",
        hole_diameter_nm=50.0,
        max_order=3,
        nx=16,
        n_periods=1,
        carpet_nz=4,
    )
    assert res.stationary_image.shape == (16, 16)
    eff = {(n, m): e for n, m, e in res.efficiencies}
    # c00 = 2 pi r^2 / (sqrt(3) a^2) for r = 25, a = 100.
    assert eff[(0, 0)] == pytest.approx((2 * np.pi * 625 / (np.sqrt(3) * 1e4)) ** 2, rel=1e-12)


def test_euv_il_fringe_period_independent_of_wavelength():
    for lam in (13.5, 6.7):
        res = simulate_euv_il(100.0, lam, nx=128, ny=2, nz=2)
        assert res.fringe_period_nm == pytest.approx(50.0, rel=1e-12)
        x = res.intensity.x_nm
        row = res.intensity.values[0, 0]
        assert _dominant_period(row, x[1] - x[0]) == pytest.approx(50.0, rel=1e-9)
    second = simulate_euv_il(100.0, 13.5, order=2, nx=128, ny=2, nz=2)
    assert second.fringe_period_nm == pytest.approx(25.0, rel=1e-12)
    assert second.diffraction_angle_deg == pytest.approx(np.degrees(np.arcsin(0.27)), rel=1e-12)
    with pytest.raises(ValueError):
        simulate_euv_il(100.0, 60.0, order=2)


def test_euv_il_visibility_and_beam_intensities():
    p = 100.0
    res = simulate_euv_il(p, 13.5, intensity_ratio=0.25, nx=64, ny=2, nz=2)
    assert res.visibility == pytest.approx(0.8, rel=1e-12)
    i1, i2 = res.beam_intensities
    assert i1 == pytest.approx(1 / np.pi**2, rel=1e-12)  # |c1|^2 of a 50 % slit grating
    assert i2 == pytest.approx(0.25 / np.pi**2, rel=1e-12)
    x = res.intensity.x_nm
    expected = i1 + i2 + 2 * np.sqrt(i1 * i2) * np.cos(4 * np.pi * x / p)
    np.testing.assert_allclose(res.intensity.values[0, 0], expected, atol=1e-12)


# ---------------------------------------------------------------------------
# Level-set development, surface inhibition, and post-exposure bake (PEB).
# ---------------------------------------------------------------------------
from highuvlith.api import (  # noqa: E402
    develop_level_set,
    development_rate,
    peb_car,
    peb_gaussian,
    simulate_volumetric,
)


def _uniform_volume(value, nx=4, ny=4, nz=20, dx=5.0, dz=5.0):
    values = np.full((nz, ny, nx), value, dtype=np.float64)
    return huv.VolumetricResult.from_array(
        values, (0.0, nx * dx), (0.0, ny * dx), (0.0, nz * dz)
    )


def test_volumetric_result_from_array_round_trip():
    values = np.arange(2 * 3 * 4, dtype=np.float64).reshape(2, 3, 4)
    vol = huv.VolumetricResult.from_array(values, (0.0, 8.0), (-3.0, 3.0), (0.0, 20.0))
    assert vol.shape == (2, 3, 4)
    np.testing.assert_array_equal(vol.values, values)
    np.testing.assert_allclose(vol.x_nm, [1.0, 3.0, 5.0, 7.0])
    np.testing.assert_allclose(vol.z_nm, [5.0, 15.0])
    with pytest.raises(ValueError):
        huv.VolumetricResult.from_array(values, (1.0, 0.0), (0.0, 1.0), (0.0, 1.0))


def test_level_set_planar_front_is_exact():
    # Fully exposed resist, threshold model (1000 nm/s): the front is planar
    # and arrives at depth z at t = z / R exactly.
    vol = _uniform_volume(0.0)
    resist = huv.ResistConfig(model="threshold")
    result = develop_level_set(vol, resist, dev_time_s=0.08)
    t = result.arrival_times.values[:, 1, 1]
    expected = vol.z_nm / 1000.0
    reached = expected < 0.08
    np.testing.assert_allclose(t[reached], expected[reached], rtol=1e-9)
    assert np.all(np.isinf(t[~reached]))
    assert result.dissolved_thickness_nm == pytest.approx(80.0, rel=1e-9)
    assert result.height_map().values.shape == (4, 4)
    assert result.developed().values.sum() == reached.sum() * 16


def test_level_set_agrees_with_fast_marching(volumetric_setup):
    vol = volumetric_setup["volume"]
    resist = volumetric_setup["resist"]
    dz = volumetric_setup["thickness_nm"] / volumetric_setup["nz"]
    t_end = 30.0
    fmm = huv.develop_fast_marching(
        vol, resist, pixel_xy_nm=volumetric_setup["pixel_nm"], pixel_z_nm=dz, lateral="periodic"
    )
    ls = develop_level_set(vol, resist, dev_time_s=t_end, lateral="periodic")
    a = fmm.values
    b = ls.arrival_times.values
    both = (a <= 0.9 * t_end) & np.isfinite(b)
    assert both.sum() > 100
    rel = np.abs(a[both] - b[both]) / a[both]
    assert np.median(rel) < 0.05
    # The default (mirror-edged) FMM call is unchanged by the new keywords.
    legacy = huv.develop_fast_marching(vol, resist, volumetric_setup["pixel_nm"], dz)
    explicit = huv.develop_fast_marching(
        vol, resist, volumetric_setup["pixel_nm"], dz,
        surface_rate_ratio=1.0, inhibition_depth_nm=0.0, lateral="reflecting",
    )
    np.testing.assert_array_equal(legacy.values, explicit.values)


def test_surface_inhibition_rate_and_top_arrival():
    vol = _uniform_volume(0.0, nz=40, dz=2.0)
    resist = huv.ResistConfig()  # Mack: R(m = 0) = rmax + rmin = 100.1 nm/s
    rate = development_rate(vol, resist, surface_rate_ratio=0.1, inhibition_depth_nm=10.0)
    expected = 100.1 * (1.0 - 0.9 * np.exp(-vol.z_nm / 10.0))
    np.testing.assert_allclose(rate.values[:, 0, 0], expected, rtol=1e-12)

    fresh = huv.develop_fast_marching(vol, resist, 5.0, 2.0)
    inhibited = huv.develop_fast_marching(
        vol, resist, 5.0, 2.0, surface_rate_ratio=0.1, inhibition_depth_nm=10.0
    )
    # Top voxel: seeded at (dz/2) / R, so the ratio is 1 / f_inh(1 nm).
    f_top = 1.0 - 0.9 * np.exp(-0.1)
    ratio = inhibited.values[0, 1, 1] / fresh.values[0, 1, 1]
    assert ratio == pytest.approx(1.0 / f_top, rel=1e-9)
    assert np.all(inhibited.values >= fresh.values)
    with pytest.raises(ValueError):
        huv.develop_fast_marching(vol, resist, 5.0, 2.0, lateral="sideways")


def test_level_set_exponential_developer_ageing():
    vol = _uniform_volume(0.0)
    resist = huv.ResistConfig(model="threshold")
    tau, t_end = 0.02, 0.05
    result = develop_level_set(
        vol, resist, t_end, depletion="exponential", depletion_time_constant_s=tau
    )
    assert result.final_rate_factor == pytest.approx(np.exp(-t_end / tau), rel=1e-12)
    # Planar front: depth z reached at t = -tau ln(1 - z / (R tau)).
    t = result.arrival_times.values[:, 1, 1]
    pseudo = vol.z_nm / 1000.0
    reached = pseudo < tau * (1.0 - np.exp(-t_end / tau))
    np.testing.assert_allclose(
        t[reached], -tau * np.log(1.0 - pseudo[reached] / tau), rtol=1e-9
    )
    assert np.all(np.isinf(t[~reached]))
    with pytest.raises(ValueError):
        develop_level_set(vol, resist, 1.0, depletion="exponential")


def test_peb_gaussian_anisotropic_moments():
    n, nz, dx, dz = 32, 32, 2.0, 1.0
    values = np.zeros((nz, n, n))
    values[nz // 2, n // 2, n // 2] = 1.0
    vol = huv.VolumetricResult.from_array(values, (0.0, n * dx), (0.0, n * dx), (0.0, nz * dz))
    baked = peb_gaussian(vol, 6.0, vertical_nm=3.0).values
    assert baked.sum() == pytest.approx(1.0, rel=1e-12)
    z = (np.arange(nz) - nz // 2) * dz
    x = (np.arange(n) - n // 2) * dx
    assert (baked.sum(axis=(1, 2)) * z**2).sum() == pytest.approx(9.0, rel=1e-6)
    assert (baked.sum(axis=(0, 1)) * x**2).sum() == pytest.approx(36.0, rel=1e-6)

    # Vertical-only diffusion leaves an x-only pattern untouched.
    stripes = np.broadcast_to((np.arange(n) % 8 < 4).astype(float), (4, n, n)).copy()
    svol = huv.VolumetricResult.from_array(stripes, (0.0, n * dx), (0.0, n * dx), (0.0, 20.0))
    np.testing.assert_allclose(peb_gaussian(svol, 0.0, vertical_nm=5.0).values, stripes, atol=1e-12)
    # A depth profile needs both parameters.
    with pytest.raises(ValueError):
        peb_gaussian(svol, 2.0, vertical_surface_ratio=2.0)
    graded = peb_gaussian(svol, 2.0, vertical_surface_ratio=3.0, vertical_decay_nm=5.0)
    assert graded.values.sum() == pytest.approx(stripes.sum(), rel=1e-12)


def test_peb_car_exact_limit_and_mass_balance():
    rng = np.random.default_rng(7)
    m_exp = rng.uniform(0.2, 1.0, size=(4, 8, 8))
    vol = huv.VolumetricResult.from_array(m_exp, (0.0, 16.0), (0.0, 16.0), (0.0, 20.0))

    # No quencher, no diffusion: m = exp(-k_amp (1 - m_exp) t) exactly.
    exact = peb_car(
        vol, peb_time_s=30.0, k_amp=0.2, quencher=0.0,
        acid_diffusivity_nm2_s=0.0, quencher_diffusivity_nm2_s=0.0,
    )
    np.testing.assert_allclose(
        exact.protected.values, np.exp(-0.2 * (1.0 - m_exp) * 30.0), rtol=1e-12
    )

    # Coupled bake: acid + neutralized = initial acid, and (acid - quencher)
    # is conserved by the 1:1 neutralization.
    coupled = peb_car(vol, quencher=0.2)
    acid0 = (1.0 - m_exp).sum()
    assert coupled.acid.values.sum() + coupled.neutralized_total == pytest.approx(acid0, rel=1e-10)
    assert coupled.acid.values.sum() - coupled.quencher.values.sum() == pytest.approx(
        acid0 - 0.2 * m_exp.size, rel=1e-10
    )
    protected = coupled.protected.values
    assert np.all((protected >= 0.0) & (protected <= 1.0))
    more_base = peb_car(vol, quencher=0.4)
    assert more_base.protected.values.mean() > protected.mean()


def test_simulate_volumetric_end_to_end():
    kwargs = dict(
        source=huv.SourceConfig.f2_laser(sigma=0.7),
        optics=huv.OpticsConfig(numerical_aperture=0.75),
        mask=huv.MaskConfig.line_space(cd_nm=64.0, pitch_nm=128.0),
        film_stack=huv.FilmStackConfig(),
        resist=huv.ResistConfig(thickness_nm=150.0),
        grid=huv.GridConfig(size=32, pixel_nm=8.0),
        dose_mj_cm2=60.0,
        nz=8,
        n_defocus_planes=2,
        max_kernels=8,
        dev_time_s=20.0,
    )
    car = simulate_volumetric(
        **kwargs, peb="car", develop="level_set",
        surface_rate_ratio=0.3, inhibition_depth_nm=10.0,
    )
    assert car.car is not None and car.level_set is not None
    assert car.arrival_times.shape == (8, 32, 32)
    assert len(car.developed_cd_nm) == 8
    heights = car.height_map.values
    assert np.all((heights >= 0.0) & (heights <= 150.0 + 1e-9))

    gauss = simulate_volumetric(**kwargs, peb="gaussian", develop="fmm")
    assert gauss.car is None and gauss.level_set is None
    assert np.all(np.isfinite(gauss.arrival_times.values))
    with pytest.raises(ValueError):
        simulate_volumetric(**kwargs, develop="fmm", depletion="loading", loading_capacity_nm=50.0)


def test_simulate_volumetric_development_notes():
    """Same over-/under-development checks as the CLI deep volumetric mode."""
    kwargs = dict(
        source=huv.SourceConfig.f2_laser(sigma=0.7),
        optics=huv.OpticsConfig(numerical_aperture=0.75),
        mask=huv.MaskConfig.line_space(cd_nm=150.0, pitch_nm=300.0),
        film_stack=huv.FilmStackConfig(),
        resist=huv.ResistConfig(thickness_nm=150.0),
        grid=huv.GridConfig(size=32, pixel_nm=18.75),
        nz=8,
        n_defocus_planes=2,
        max_kernels=8,
    )
    notes = {d: simulate_volumetric(**kwargs, dose_mj_cm2=d).notes for d in (1.0, 12.0, 30.0)}
    assert notes[1.0] and notes[1.0][0].startswith("under-developed")
    assert notes[12.0] == []  # near dose-to-size for this L/S (CLI: bottom CD 150 nm)
    # The 30 mJ/cm² default over-develops the default stack (CLI: same 16.4 nm).
    assert notes[30.0] and "mean 16.4 nm of 150 nm" in notes[30.0][0]
