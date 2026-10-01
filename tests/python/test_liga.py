"""LIGA deep-X-ray lithography, X-ray optical constants and EUV/BEUV
multilayer mirrors (WP-E1): Fresnel proximity diffraction, absolute exposure
time, the straight-edge profile tool, and the materials bindings.

Reference numbers come from independent calculations (numpy
re-implementations, the CXRO on-line calculators); see the Rust fixture tests
in ``deep_xray.rs`` / ``materials/*.rs`` for provenance.
"""

import math
import warnings

import numpy as np
import pytest

import highuvlith as huv
from highuvlith import api

SMALL = dict(grid_size=16, pixel_nm=500.0, nz=4, cd_nm=2000.0, pitch_nm=4000.0)


def _quiet_liga(**kwargs):
    with warnings.catch_warnings():
        warnings.simplefilter("ignore", UserWarning)
        return api.simulate_liga(**kwargs)


# ---------------------------------------------------------------------------
# simulate_liga: models, options, sampling check
# ---------------------------------------------------------------------------
def test_fresnel_is_default_and_reports_sampling():
    result = _quiet_liga(resist_thickness_um=200.0, energy_bins=24, **SMALL)
    assert result.diffraction == "fresnel"
    top, bottom = result.fresnel_scale_nm
    assert 50.0 < top < bottom
    # 500 nm pixels cannot resolve a ~0.2 um Fresnel scale: flagged.
    assert result.sampling_resolved is False
    assert any("under-resolves" in w for w in result.warnings)
    assert result.volume.shape == (4, 16, 16)
    assert result.exposure_time_s is None  # relative spectrum


def test_gaussian_option_same_depth_dose():
    kw = dict(resist_thickness_um=200.0, energy_bins=24, **SMALL)
    fresnel = _quiet_liga(diffraction="fresnel", **kw)
    gauss = _quiet_liga(diffraction="gaussian", **kw)
    assert gauss.diffraction == "gaussian"
    # The depth dose does not depend on the lateral model.
    np.testing.assert_allclose(gauss.depth_dose[1], fresnel.depth_dose[1], rtol=1e-12)
    with pytest.raises(ValueError):
        api.simulate_liga(diffraction="kirchhoff", **SMALL)


def test_coarse_grid_warns_and_strict_raises():
    with pytest.warns(UserWarning, match="under-resolves"):
        api.simulate_liga(resist_thickness_um=100.0, energy_bins=16, **SMALL)
    with pytest.raises(ValueError, match="Fresnel"):
        api.simulate_liga(resist_thickness_um=100.0, energy_bins=16, strict_sampling=True, **SMALL)


def test_filters_harden_beam_and_accept_formula_specs():
    kw = dict(resist_thickness_um=500.0, energy_bins=40, **SMALL)
    bare = _quiet_liga(**kw)
    al = _quiet_liga(filters=[("Al", 20.0)], **kw)
    kapton = _quiet_liga(filters=[("C22H10N2O5@1.42", 100.0)], **kw)
    assert al.dose_ratio < bare.dose_ratio
    assert kapton.dose_ratio < bare.dose_ratio
    assert al.mean_energy_top_kev > bare.mean_energy_top_kev
    with pytest.raises(ValueError):
        api.simulate_liga(filters=[("unobtainium", 1.0)], **SMALL)


def test_spectrum_selectors_are_exclusive():
    src = huv.SourceConfig.synchrotron_liga_bending_magnet()
    with pytest.raises(ValueError, match="only one spectrum selector"):
        api.simulate_liga(critical_energy_kev=6.0, source=src, **SMALL)
    with pytest.raises(ValueError, match="both source_distance_m"):
        api.simulate_liga(source=src, source_distance_m=15.0, **SMALL)


# ---------------------------------------------------------------------------
# Absolute exposure time
# ---------------------------------------------------------------------------
def test_absolute_bending_magnet_exposure_time():
    # 2.5 GeV / 1.5 T / 200 mA, 15 m, 50 mm scan, 2 um Ti, 500 um PMMA,
    # 3 kJ/cm^3 bottom dose: an independent numpy implementation gives
    # t = 605.265 s and a top/bottom ratio of 20.777.
    src = huv.SourceConfig.synchrotron_liga_bending_magnet()
    result = _quiet_liga(
        source=src,
        source_distance_m=15.0,
        vertical_scan_mm=50.0,
        horizontal_acceptance_mrad=5.0,
        **SMALL,
    )
    assert result.exposure_time_s == pytest.approx(605.265, rel=1e-4)
    assert result.dose_ratio == pytest.approx(20.777, rel=1e-4)
    assert result.exposure_charge_ma_h == pytest.approx(200.0 * 605.265 / 3600.0, rel=1e-4)
    assert result.top_dose_rate_kj_cm3_s * result.exposure_time_s == pytest.approx(
        result.top_dose_kj_cm3, rel=1e-6
    )
    # Behind 2 um Ti the default [0.1, 8] E_c window clips only the ~0.1 %
    # tail above 8 E_c.
    assert 0.995 < result.window_power_fraction <= 1.0


def test_window_clipping_is_reported():
    # No membrane: ~4.6 % of the bending-magnet power lies below 0.1 E_c and
    # reaches the resist - the result says so.
    result = _quiet_liga(membrane_thickness_um=0.0, **SMALL)
    assert 0.93 < result.window_power_fraction < 0.97
    assert any("outside the sampled energy window" in w for w in result.warnings)


def test_absolute_flux_density_table_exposure_time():
    # Contract C5: an absolute photons s^-1 mm^-2 keV^-1 table (e.g. from an
    # X-ray tube). Two lines at 7.999/8.001 keV carrying 0.5e12 photons
    # s^-1 mm^-2 each, 200 um PMMA, no membrane: t = 3755.71 s (numpy, NIST).
    result = _quiet_liga(
        flux_density=[(7.999, 2.5e14), (8.001, 2.5e14)],
        resist_thickness_um=200.0,
        membrane_thickness_um=0.0,
        **SMALL,
    )
    assert result.exposure_time_s == pytest.approx(3755.71, rel=2e-4)
    assert result.resist_power_density_w_mm2 == pytest.approx(1.28174e-3, rel=1e-4)
    assert result.exposure_charge_ma_h is None


# ---------------------------------------------------------------------------
# Fine 1D straight-edge profile
# ---------------------------------------------------------------------------
def test_edge_profile_reproduces_fresnel_straight_edge():
    # Monochromatic 8 keV, opaque absorber (200 um Au), 100 um gap: the
    # surface dose is the textbook straight-edge pattern - 1/4 of the open
    # level at the geometric edge, first maximum 1.3704 at v = 1.2172 with
    # v = x sqrt(2 / (lambda g)).
    lam = 1.23984193 / 8.0
    g = 100e3
    scale = math.sqrt(lam * g / 2.0)
    p = api.liga_edge_profile(
        spectrum_table=[(8.0, 1.0)],
        resist_thickness_um=100.0,
        proximity_gap_um=100.0,
        absorber_thickness_um=200.0,
        membrane_thickness_um=0.0,
        x_min_nm=-400.0,
        x_max_nm=400.0,
        dx_nm=0.5,
        depths_um=[0.0],
    )
    x = p.x_nm
    rel = p.dose_kj_cm3[0] / p.open_dose_kj_cm3[0]
    assert rel[np.argmin(np.abs(x))] == pytest.approx(0.25, abs=1e-9)
    i = np.argmax(rel)
    assert rel[i] == pytest.approx(1.37044, rel=1e-4)
    assert x[i] / scale == pytest.approx(1.2172, abs=5e-3)


def test_edge_profile_sidewall_metrics():
    p = api.liga_edge_profile(
        resist_thickness_um=200.0,
        energy_bins=24,
        x_min_nm=-2000.0,
        x_max_nm=2000.0,
        dx_nm=10.0,
        depths_um=[0.0, 100.0, 200.0],
    )
    assert p.dose_kj_cm3.shape == (3, len(p.x_nm))
    assert p.open_dose_kj_cm3[-1] == pytest.approx(3.0, rel=1e-9)
    edges = p.edge_positions_nm(2.7)
    assert all(e is not None for e in edges)
    angle = p.sidewall_angle_deg(2.7)
    assert 0.0 < angle < 5.0


# ---------------------------------------------------------------------------
# Optical constants and multilayer mirrors
# ---------------------------------------------------------------------------
def test_xray_optical_constants_match_cxro():
    # CXRO calculator (henke.lbl.gov): Mo at 10.22 g/cm^3, 13.5 nm:
    # delta = 0.0762065, beta = 0.00643542.
    delta, beta = api.xray_optical_constants("Mo", [13.5])
    assert delta[0] == pytest.approx(0.0762065, rel=5e-4)
    assert beta[0] == pytest.approx(0.00643542, rel=5e-4)
    d2, _ = api.xray_optical_constants("B4C", [6.7], density_g_cm3=2.52)
    assert d2[0] == pytest.approx(0.00103261, abs=3e-6)
    with pytest.raises(ValueError):
        api.xray_optical_constants("Mo", [100.0])


def test_mo_si_multilayer_matches_cxro():
    # CXRO multilayer calculator: Si/Mo 40 x 6.9 nm, Gamma 0.4, on SiO2:
    # R(13.5 nm) = 0.7293, peak 0.7300 at 13.48 nm.
    m = api.MultilayerMirror.mo_si()
    assert m.reflectance(13.5) == pytest.approx(0.7293, abs=3e-3)
    lam, r = m.peak(13.0, 14.0)
    assert lam == pytest.approx(13.48, abs=0.01)
    assert r == pytest.approx(0.7300, abs=3e-3)
    curve = m.reflectance_curve(np.linspace(13.0, 14.0, 11))
    assert curve.shape == (11,) and curve.max() < 0.75
    resp = m.pupil_response(13.5, 0.0)
    assert resp["reflectance_unpolarized"] == pytest.approx(m.reflectance(13.5), rel=1e-12)
    assert abs(resp["rs"] - resp["rp"]) < 1e-12
    assert 0.45 < m.bandwidth_fwhm_nm(12.8, 14.2) < 0.7
    capped = m.with_capping("Ru", 2.0).with_roughness(0.3)
    assert capped.peak(13.0, 14.0)[1] < 0.76


def test_la_b_mirror_near_boron_edge():
    # Ideal La/B tuned to 6.65 nm (just longward of the 188 eV boron K edge):
    # ~80 % (measured records ~64 %), dark at 6.5 nm where boron absorbs.
    d = api.tune_multilayer_period("la_b", 6.65, 200)
    assert d == pytest.approx(3.329, abs=0.003)
    m = api.MultilayerMirror.la_b(200, d, 0.4)
    lam, r = m.peak(6.55, 6.75, polarization="te")
    assert lam == pytest.approx(6.65, abs=1e-3)
    assert 0.75 < r < 0.85
    assert m.reflectance(6.5, polarization="te") < 0.05


def test_la_b4c_period_tuning():
    d = api.tune_multilayer_period("la_b4c", 6.7, 200)
    assert 3.33 < d < 3.42
    m = api.MultilayerMirror.la_b4c(200, d, 0.4)
    lam, r = m.peak(6.6, 6.8, polarization="te")
    assert lam == pytest.approx(6.7, abs=1e-3)
    assert 0.6 < r < 0.72
    with pytest.raises(ValueError):
        api.tune_multilayer_period("w_si", 6.7, 200)
