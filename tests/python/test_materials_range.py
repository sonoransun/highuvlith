"""Materials-database range safety: lookups never extrapolate a table.

Fixture provenance: CXRO on-line "Index of Refraction" calculator
(henke.lbl.gov/cgi-bin/getdb.pl, queried 2026-09-30, default densities
Si 2.33, Cr 7.19, SiO2 2.2 g/cm^3); the Si VUV table nodes (157 nm:
0.88 + 2.10i, 160 nm: 0.90 + 2.12i) interpolated by hand.
"""

import pytest

import highuvlith as huv

# (name, delta, beta) at 13.5 nm from the CXRO calculator.
CXRO_13P5 = [
    ("Si", 0.000998703763, 0.00182646094),
    ("Cr", 0.0751224086, 0.043694526),
    ("SiO2", 0.0219678637, 0.010773153),
]


@pytest.mark.parametrize("name,delta,beta", CXRO_13P5)
def test_vuv_names_fall_back_to_henke_at_euv(name, delta, beta):
    # Before the fix "Si" at 13.5 nm returned the 126 nm table value 0.55+1.75i.
    n = huv.refractive_index(name, 13.5)
    assert abs((1.0 - n.real) - delta) < 5e-4 * delta + 2e-6
    assert abs(n.imag - beta) < 5e-4 * beta + 2e-6
    assert n == huv.refractive_index(f"henke:{name}", 13.5)


def test_si_matches_si_euv_and_vuv_interpolation():
    assert huv.refractive_index("Si", 13.5) == huv.refractive_index("Si_euv", 13.5)
    n = huv.refractive_index("Si", 157.63)  # t = 0.63 / 3 between 157 and 160 nm
    assert n.real == pytest.approx(0.8842, abs=1e-12)
    assert n.imag == pytest.approx(2.1042, abs=1e-12)


@pytest.mark.parametrize(
    "name,wavelength,needle",
    [
        ("Si", 80.0, "henke:Si"),  # gap between the Henke and VUV ranges
        ("CaF2", 13.5, "Mo_euv"),  # no Henke table for F
        ("Si_euv", 157.0, "'Si'"),  # Henke entry outside 0.0413-41.3 nm
        ("VUV_resist", 193.0, "125-158"),
        ("SiO2", 157.63, "165-2000"),  # fused silica absorbs below ~165 nm
    ],
)
def test_out_of_range_raises_value_error_naming_alternative(name, wavelength, needle):
    with pytest.raises(ValueError) as exc:
        huv.refractive_index(name, wavelength)
    msg = str(exc.value)
    assert msg.startswith(f"{name}: wavelength")
    assert "outside its data range" in msg
    assert needle in msg


def test_in_range_callers_unchanged():
    # Node values and the F2-line end hold (1 nm) are returned as before.
    assert huv.refractive_index("Cr", 157.0) == complex(1.06, 2.05)
    assert huv.refractive_index("VUV_resist", 157.63) == complex(1.65, 0.015)
    caf2 = huv.refractive_index("CaF2", 157.63)
    assert caf2.real == pytest.approx(1.55702, abs=1e-5) and caf2.imag == 0.0
