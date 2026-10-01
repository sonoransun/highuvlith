"""Bindings of the quantum (N-photon) research module (🧪).

Fixture numbers are computed here independently of the Rust code (closed
forms with math.comb / numpy and the documented entangled-source constants).
"""

import math

import numpy as np
import pytest

import highuvlith as huv

HC_EV_NM = 1239.84193
E_CHARGE = 1.602176634e-19


def _classical_fringe(u, n):
    mean = math.comb(2 * n, n) / 2**n
    return (1.0 + np.cos(u)) ** n / mean


def test_two_beam_periods_and_harmonics_fixture():
    tb = huv.TwoBeamNPhoton(157.63, 30.0, num_photons=3, fidelity=0.7)
    # sin 30° = 1/2: classical period λ/(2·½) = λ, N00N period λ/3.
    assert tb.classical_period_nm() == pytest.approx(157.63, rel=1e-12)
    assert tb.noon_period_nm() == pytest.approx(157.63 / 3, rel=1e-12)
    assert tb.fringe_wavenumber_per_nm() == pytest.approx(2 * math.pi / 157.63, rel=1e-12)
    # a_j = F·[j = N] + (1 − F)·2C(2N, N−j)/C(2N, N), N = 3: 0.45, 0.18, 0.73.
    expected = [1.0] + [
        0.7 * (j == 3) + 0.3 * 2 * math.comb(6, 3 - j) / math.comb(6, 3) for j in (1, 2, 3)
    ] + [0.0]
    assert expected[1:4] == pytest.approx([0.45, 0.18, 0.73], abs=1e-12)
    for j, a in enumerate(expected):
        assert tb.harmonic_amplitude(j) == pytest.approx(a, abs=1e-12)
    assert tb.status == "🧪" and tb.notes


def test_two_beam_profile_matches_closed_form():
    lam, theta, n, f, phi = 193.0, 20.0, 2, 0.6, 0.3
    tb = huv.TwoBeamNPhoton(lam, theta, num_photons=n, fidelity=f, phase_rad=phi)
    k = 4 * math.pi * math.sin(math.radians(theta)) / lam
    x = np.linspace(-300.0, 300.0, 41)
    u = k * x + phi
    noon = 1.0 + np.cos(n * u)
    classical = _classical_fringe(u, n)
    np.testing.assert_allclose(tb.profile(x), f * noon + (1 - f) * classical, rtol=1e-12, atol=1e-12)
    assert tb.noon_exposure(x[3]) == pytest.approx(noon[3], abs=1e-12)
    assert tb.classical_exposure(x[3]) == pytest.approx(classical[3], abs=1e-12)
    assert tb.exposure(x[3]) == pytest.approx(f * noon[3] + (1 - f) * classical[3], abs=1e-12)
    # Unit mean over one classical period for any F.
    xp = np.linspace(0.0, 2 * math.pi / k, 4096, endpoint=False)
    assert tb.profile(xp).mean() == pytest.approx(1.0, abs=1e-9)


@pytest.mark.parametrize(
    "kwargs",
    [
        {"half_angle_deg": 0.0},
        {"half_angle_deg": 90.0},
        {"num_photons": 0},
        {"fidelity": 1.5},
        {"wavelength_nm": -1.0},
    ],
)
def test_two_beam_rejects_bad_parameters(kwargs):
    args = {"wavelength_nm": 157.63, "half_angle_deg": 30.0, "num_photons": 2, "fidelity": 1.0}
    args.update(kwargs)
    with pytest.raises(ValueError):
        huv.TwoBeamNPhoton(**args)


def test_n_photon_absorption_is_the_power_law_and_alias_ignores_fidelity():
    rng = np.random.default_rng(1)
    img = rng.uniform(0.0, 1.4, size=(8, 8))
    img[0, 0] = -1e-12  # roundoff below zero is clamped
    out = huv.n_photon_absorption_image(img, 3)
    expected = np.clip(img, 0.0, None) ** 3
    np.testing.assert_allclose(out, expected, rtol=1e-14, atol=0.0)
    for fidelity in (0.0, 0.5, 1.0):
        np.testing.assert_array_equal(huv.quantum_aerial_image(img, n=3, fidelity=fidelity), out)
    with pytest.raises(ValueError):
        huv.n_photon_absorption_image(img, 0)


def test_flux_budget_from_entangled_source_constants():
    budget = huv.quantum_flux_budget(157.63, 2)
    e_ev = HC_EV_NM / 157.63
    hvm_rate = 0.67 / (e_ev * E_CHARGE)  # HVM_WAFER_POWER_W = 0.67 W
    assert budget["photon_energy_ev"] == pytest.approx(e_ev, rel=1e-6)
    assert budget["hvm_photon_rate_per_s"] == pytest.approx(hvm_rate, rel=1e-6)
    assert budget["relative_flux"] == pytest.approx(1e13 / hvm_rate, rel=1e-6)
    # p_clear·N·P_HVM / (σ·Φ·D) with p = 0.1, Φ = 1e13 /s, D = 30 mJ/cm².
    for key, sigma in (
        ("exposure_time_ratio_bound", 1e-23),
        ("exposure_time_ratio_tightest_bound", 1e-25),
        ("exposure_time_ratio_claimed", 1e-17),
    ):
        assert budget[key] == pytest.approx(0.1 * 2 * 0.67 / (sigma * 1e13 * 0.030), rel=1e-12)
    classical = huv.quantum_flux_budget(157.63, 1)
    assert classical["relative_flux"] == 1.0 and classical["exposure_time_ratio_bound"] == 1.0
    assert huv.TwoBeamNPhoton(157.63, 30.0).flux_budget() == budget


def _noon_engine(pixel_nm=2.0):
    source = huv.SourceConfig(wavelength_nm=157.63, sigma_outer=0.5)
    optics = huv.OpticsConfig(numerical_aperture=0.75)
    mask = huv.MaskConfig.line_space(cd_nm=50.0, pitch_nm=100.0)
    grid = mask.commensurate_grid(size=64, target_pixel_nm=pixel_nm)
    return huv.SimulationEngine(source, optics, mask, grid=grid, max_kernels=20)


def test_noon_ideal_image_resolves_below_the_classical_cutoff():
    # 100 nm pitch at 157.63 nm, NA 0.75, σ 0.5: below the classical cutoff
    # λ/(NA(1 + σ)) = 140.1 nm → classical (and I²) contrast 0; at λ/2 the
    # cutoff is 70 nm, so the pitch images.
    assert 157.63 / (0.75 * 1.5) > 100.0 > 157.63 / 2 / (0.75 * 1.5)
    res = _noon_engine().compute_noon_ideal_image(num_photons=2, fidelity=1.0)
    assert res.classical.image_contrast() == pytest.approx(0.0, abs=1e-9)
    assert res.n_photon_absorption.image_contrast() == pytest.approx(0.0, abs=1e-9)
    assert res.noon_limit.image_contrast() > 0.6
    np.testing.assert_allclose(res.exposure.intensity, res.noon_limit.intensity, rtol=0, atol=1e-12)
    assert res.effective_wavelength_nm == pytest.approx(157.63 / 2)
    assert res.classical_resolution_nm == pytest.approx(0.61 * 157.63 / 0.75)
    assert res.noon_limit_resolution_nm == pytest.approx(0.61 * 157.63 / 1.5)
    assert res.flux == huv.quantum_flux_budget(157.63, 2)
    assert res.status == "🧪" and len(res.notes) == 3


def test_noon_ideal_image_fidelity_mixture_and_grid_check():
    engine = _noon_engine()
    res = engine.compute_noon_ideal_image(num_photons=2, fidelity=0.25)
    np.testing.assert_allclose(
        res.exposure.intensity,
        0.25 * np.asarray(res.noon_limit.intensity)
        + 0.75 * np.asarray(res.classical.intensity) ** 2,
        rtol=1e-12,
        atol=1e-12,
    )
    # N = 1 is classical imaging exactly.
    one = engine.compute_noon_ideal_image(num_photons=1, fidelity=1.0)
    np.testing.assert_array_equal(one.noon_limit.intensity, one.classical.intensity)
    # pixel must be ≤ λ/(4·N·NA) = 13.1 nm for N = 4.
    with pytest.raises(ValueError, match="pixel"):
        _noon_engine(pixel_nm=25.0).compute_noon_ideal_image(num_photons=4)


def test_quantum_line_space_models():
    absorb = huv.simulate_quantum_line_space(
        120.0, 240.0, n=2, grid_size=64, pixel_nm=4.0, max_kernels=12
    )
    assert absorb.model == "n_photon_absorption" and absorb.status == "🧪"
    np.testing.assert_allclose(absorb.quantum, np.asarray(absorb.classical) ** 2, rtol=1e-12)
    assert absorb.quantum_contrast > absorb.classical_contrast
    assert absorb.exposure_time_ratio_bound == pytest.approx(0.1 * 2 * 0.67 / (1e-23 * 1e13 * 0.03))

    noon = huv.simulate_quantum_line_space(
        50.0, 100.0, n=2, model="noon", sigma=0.5, grid_size=64, pixel_nm=2.0
    )
    assert noon.classical_contrast == pytest.approx(0.0, abs=1e-9)
    assert noon.quantum_contrast > 0.6
    assert noon.noon_limit_resolution_nm == pytest.approx(noon.classical_resolution_nm / 2)
    with pytest.raises(ValueError):
        huv.simulate_quantum_line_space(50.0, 100.0, model="boto")
