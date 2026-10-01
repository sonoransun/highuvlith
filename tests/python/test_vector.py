"""Vector (polarized) pupil bindings: VectorSettings and pupil-physics helpers.

Closed-form checks mirror crates/highuvlith-core/tests/vector_pupil.rs; fixture
numbers come from an independent numpy reference (explicit θ/φ unit vectors
and a numerical boundary-condition solve for the Fresnel coefficients).
"""

import math

import numpy as np
import pytest

from highuvlith._native import (
    VectorSettings,
    fresnel_coefficients,
    radiometric_factor,
    vector_pupil_field,
)
from highuvlith.api import pupil_polarization_map, vector_two_beam_contrast


def test_settings_defaults_and_getters():
    s = VectorSettings()
    assert s.polarization == "unpolarized"
    assert s.angle_deg is None
    assert s.image_index == 1.0
    assert s.obliquity is True
    assert s.reduction == 4.0
    assert s.film_n is None and s.film_k is None
    assert s.columns_per_source_point() == 6

    lin = VectorSettings("linear", angle_deg=30.0, film_n=1.7, film_k=0.03)
    assert lin.polarization == "linear" and lin.angle_deg == 30.0
    assert lin.film_n == 1.7 and lin.film_k == 0.03
    assert lin.columns_per_source_point() == 3
    assert "VectorSettings(" in repr(lin)

    # Aliases and case-insensitivity; TE/TM reserve the on-axis fallback state.
    assert VectorSettings("azimuthal").polarization == "te"
    assert VectorSettings("RADIAL").polarization == "tm"
    assert VectorSettings("te").columns_per_source_point() == 6


@pytest.mark.parametrize(
    "kwargs",
    [
        {"polarization": "circular"},
        {"image_index": 0.0},
        {"reduction": -1.0},
        {"film_n": 1.7, "film_k": -0.1},
        {"film_n": 0.0},
        {"angle_deg": float("nan")},
    ],
)
def test_settings_reject_bad_parameters(kwargs):
    with pytest.raises(ValueError):
        VectorSettings(**kwargs)


def test_validate_against_na():
    VectorSettings().validate(0.9)
    VectorSettings(image_index=1.44).validate(1.35)  # water immersion
    with pytest.raises(ValueError):
        VectorSettings().validate(1.0)  # NA must stay below the image index
    with pytest.raises(ValueError):
        VectorSettings(reduction=0.5).validate(0.9)


def test_field_on_axis_is_the_jones_vector():
    e = vector_pupil_field(
        VectorSettings("linear", angle_deg=30.0, obliquity=False), 0.9, 0.0, 0.0
    )
    assert e.shape == (1, 3) and e.dtype == np.complex128
    a = math.radians(30.0)
    np.testing.assert_allclose(e[0], [math.cos(a), math.sin(a), 0.0], atol=1e-15)
    un = vector_pupil_field(VectorSettings(), 0.9, 0.0, 0.0)
    h = 2**-0.5
    np.testing.assert_allclose(un, [[h, 0.0, 0.0], [0.0, h, 0.0]], atol=1e-15)


@pytest.mark.parametrize("na", [0.3, 0.8, 0.95])
def test_two_beam_contrast_closed_forms(na):
    # Orders at (±1, 0): y-pol is TE, x-pol is TM with contrast |cos 2θ|.
    assert vector_two_beam_contrast(na, "y") == pytest.approx(1.0, abs=1e-12)
    assert vector_two_beam_contrast(na, "x") == pytest.approx(abs(1 - 2 * na**2), abs=1e-12)
    assert vector_two_beam_contrast(na, "unpolarized") == pytest.approx(1 - na**2, abs=1e-12)
    # Azimuthal/radial illumination at a source point on the x axis = y/x.
    assert vector_two_beam_contrast(na, "te") == pytest.approx(1.0, abs=1e-12)
    assert vector_two_beam_contrast(na, "tm") == pytest.approx(abs(1 - 2 * na**2), abs=1e-12)


def test_two_beam_tm_contrast_inside_film():
    # n = 1.7 film at NA 0.9: |cos 2θ₂| with sin θ₂ = 0.9/1.7.
    got = vector_two_beam_contrast(0.9, "x", film_n=1.7)
    assert got == pytest.approx(0.439446366782007, abs=1e-12)


def test_pupil_map_norm_and_symmetry():
    m = pupil_polarization_map(0.9, "x", n=33, obliquity=False)
    assert m.field.shape == (1, 3, 33, 33)
    pxx, pyy = np.meshgrid(m.px, m.py)
    inside = pxx**2 + pyy**2 <= 1.0
    # Norm preserved (|M·J| = |J|) inside the pupil, zero outside.
    np.testing.assert_allclose(m.intensity[inside], 1.0, atol=1e-12)
    assert np.all(m.intensity[~inside] == 0.0)
    # E_z = −α·J_x = −NA·p_x: odd in p_x.
    ez = m.field[0, 2]
    np.testing.assert_allclose(ez, -ez[:, ::-1], atol=1e-12)
    np.testing.assert_allclose(ez[inside], (-0.9 * pxx)[inside], atol=1e-12)
    # Largest longitudinal share at the x edge of the pupil: sin²θ = NA².
    assert m.longitudinal_fraction.max() == pytest.approx(0.81, abs=1e-12)


def test_pupil_map_obliquity_energy_statement():
    # |E|² cos θ_img = cos θ_obj at every pupil point (R = 4, air).
    m = pupil_polarization_map(0.9, "unpolarized", n=33)
    pxx, pyy = np.meshgrid(m.px, m.py)
    rho2 = pxx**2 + pyy**2
    inside = rho2 <= 1.0
    cos_img = np.sqrt(1 - 0.81 * rho2[inside])
    cos_obj = np.sqrt(1 - 0.81 * rho2[inside] / 16.0)
    np.testing.assert_allclose(m.intensity[inside] * cos_img, cos_obj, rtol=1e-12)


def test_radiometric_factor_fixture():
    assert radiometric_factor(0.9) == pytest.approx(1.4951027749973473, abs=1e-12)
    assert radiometric_factor(0.0) == 1.0
    # Object at infinity: 1/√cos θ (angular-spectrum form of Richards–Wolf).
    assert radiometric_factor(0.5, 1.0, math.inf) == pytest.approx(0.75**-0.25, abs=1e-12)


def test_fresnel_coefficients():
    f = fresnel_coefficients(1.0, 1.7, 0.0, 0.0)
    assert f["t_s"] == pytest.approx(2 / 2.7, abs=1e-15)
    assert isinstance(f["t_p"], complex)
    g = fresnel_coefficients(1.0, 1.7, 0.03, 0.6)
    assert g["t_s"] == pytest.approx(complex(0.6691571505173691, -0.008974562862638537), abs=1e-12)
    assert g["R_s"] + g["T_s"] == pytest.approx(1.0, abs=1e-12)
    assert g["R_p"] + g["T_p"] == pytest.approx(1.0, abs=1e-12)
    brewster = fresnel_coefficients(1.0, 1.7, 0.0, math.sin(math.atan(1.7)))
    assert abs(brewster["r_p"]) < 1e-12
    assert brewster["T_p"] == pytest.approx(1.0, abs=1e-12)
    with pytest.raises(ValueError):
        fresnel_coefficients(1.0, 1.7, 0.0, 1.0)
