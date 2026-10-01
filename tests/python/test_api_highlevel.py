"""High-level API convenience functions: immersion / vector line-space,
illumination comparison, ED process window, source landscape, throughput,
package exports."""

import numpy as np
import pytest

import highuvlith as huv
from highuvlith.viz.sources import MATURITY_CLASSES


def test_package_exports_resolve():
    for name in huv.__all__:
        assert hasattr(huv, name), name
    assert huv.__version__ == "0.2.0"
    for name in (
        "VectorSettings",
        "pupil_polarization_map",
        "vector_two_beam_contrast",
        "PupilPolarizationMap",
        "MultilayerMirror",
        "liga_edge_profile",
        "develop_level_set",
        "peb_car",
        "simulate_talbot",
        "TwoBeamNPhoton",
        "compare_illumination",
        "source_landscape",
    ):
        assert name in huv.__all__, name


def test_immersion_resolves_a_pitch_dry_optics_cannot():
    # 90 nm pitch at 193.368 nm, annular 0.7–0.9: dry NA 0.93 cuts off at
    # λ/(NA(1 + σ)) = 109 nm > 90 nm (no first order → contrast 0); water
    # immersion NA 1.35 cuts off at 75 nm.
    common = dict(
        wavelength_nm=193.368, illumination=("annular", 0.7, 0.9), grid_size=64, pixel_nm=1.5
    )
    dry = huv.simulate_line_space(45.0, 90.0, na=0.93, **common)
    wet = huv.simulate_line_space(45.0, 90.0, na=1.35, immersion=True, **common)
    assert dry.contrast == pytest.approx(0.0, abs=1e-9)
    assert wet.contrast > 0.3
    assert wet.config["immersion_index"] == pytest.approx(1.437)
    assert wet.config["illumination"] == ("annular", 0.7, 0.9)
    assert wet.status == "✅" and wet.notes
    with pytest.raises(ValueError):
        huv.simulate_line_space(45.0, 90.0, na=1.35, **common)  # dry NA must be < 1
    with pytest.raises(ValueError):
        huv.simulate_line_space(45.0, 90.0, na=1.40, immersion=True, **common)  # > 0.95·n


def test_vector_te_beats_tm_for_dense_lines():
    # 90 nm pitch, lines along y, x-dipole at σ 0.8 (≈ two-beam): the orders
    # meet in water at sin θ = λ/(2 p n) = 0.748 (θ = 48.4°). TE (azimuthal ≈
    # y at the x poles) keeps full overlap, like scalar imaging; TM fringes
    # scale with |cos 2θ| = 0.118.
    common = dict(
        wavelength_nm=193.368,
        na=1.35,
        immersion=True,
        illumination=("dipole", 0.8, 0.1, 0.0),
        grid_size=64,
        pixel_nm=1.5,
    )
    scalar = huv.simulate_line_space(45.0, 90.0, **common)
    te = huv.simulate_line_space(45.0, 90.0, imaging="vector", polarization="te", **common)
    tm = huv.simulate_line_space(45.0, 90.0, imaging="vector", polarization="tm", **common)
    y = huv.simulate_line_space(45.0, 90.0, imaging="vector", polarization="y", **common)
    assert te.config["imaging"] == "vector" and te.config["polarization"] == "te"
    assert scalar.config["polarization"] is None
    sin_t = 193.368 / (2 * 90.0 * 1.437)
    cos2t = abs(1 - 2 * sin_t**2)
    assert cos2t == pytest.approx(0.118, abs=1e-3)
    assert te.contrast == pytest.approx(scalar.contrast, abs=0.03)
    assert y.contrast == pytest.approx(te.contrast, abs=0.02)
    assert tm.contrast < 2 * cos2t * te.contrast
    with pytest.raises(ValueError):
        huv.simulate_line_space(45.0, 90.0, imaging="rigorous", **common)


def test_simulate_line_space_with_resist_reports_split_status():
    r = huv.simulate_line_space(65.0, 180.0, grid_size=64, pixel_nm=3.0, with_resist=True)
    assert r.status == "✅/🔶"
    assert any("Dill" in n for n in r.notes)


def test_compare_illumination_defaults():
    rows = huv.compare_illumination(90.0, 180.0, grid_size=64, pixel_nm=3.0)
    by_name = {r.name.split()[0]: r for r in rows}
    assert set(by_name) == {"conventional", "annular", "dipole-x", "quadrupole"}
    # Off-axis dipole illumination wins at k1 = 90·0.75/157.63 ≈ 0.43.
    assert by_name["dipole-x"].contrast > by_name["conventional"].contrast
    for r in rows:
        assert r.illumination[0] in ("conventional", "annular", "dipole", "quadrupole")
        c = r.contrast_through_focus
        assert len(c) == len(r.focuses_nm) == 9
        # Aberration-free symmetric system: contrast(−z) = contrast(z), peak at 0.
        np.testing.assert_allclose(c, c[::-1], rtol=0, atol=1e-9)
        assert c.argmax() == 4 and c[4] == pytest.approx(r.contrast, abs=1e-12)
    with pytest.raises(ValueError):
        huv.compare_illumination(90.0, 180.0, {})


def test_process_window_anchors_dose_to_size():
    pw = huv.process_window(
        90.0,
        180.0,
        grid_size=64,
        pixel_nm=3.0,
        doses_mj_cm2=list(np.linspace(24.0, 36.0, 7)),
        focuses_nm=list(np.linspace(-150.0, 150.0, 7)),
    )
    assert pw.status == "🔶" and pw.notes
    # E_th was chosen so the line prints at 90 nm in focus at 30 mJ/cm².
    assert pw.cd_at(30.0, 0.0) == pytest.approx(90.0, abs=0.5)
    assert pw.dose_to_size() == pytest.approx(30.0, rel=0.02)
    assert pw.depth_of_focus() > 0.0 and pw.exposure_latitude() > 0.0
    explicit = huv.process_window(
        90.0,
        180.0,
        grid_size=64,
        pixel_nm=3.0,
        dose_to_clear_mj_cm2=pw.dose_to_clear_mj_cm2,
        doses_mj_cm2=list(np.linspace(24.0, 36.0, 7)),
        focuses_nm=list(np.linspace(-150.0, 150.0, 7)),
    )
    np.testing.assert_allclose(explicit.cd_matrix, pw.cd_matrix, equal_nan=True)


def test_source_landscape_rows_are_live_and_labelled():
    rows = huv.source_landscape()
    labels = [r.label for r in rows]
    assert len(labels) == len(set(labels))
    maturities = {m for m, _ in MATURITY_CLASSES}
    for r in rows:
        assert r.maturity in maturities, r.label
        assert r.status[0] in "✅🔶🧪", r.label
        src = eval("huv.SourceConfig." + r.factory)  # noqa: S307 - curated factory strings
        assert r.wavelength_nm == src.wavelength_nm
        assert r.average_power_w == src.average_power_w
        assert r.family == src.kind
    sn = next(r for r in rows if r.label == "Sn LPP 250 W")
    assert sn.average_power_w == pytest.approx(250.0)
    assert "IF" in sn.power_definition
    assert all(r.average_power_w is None for r in rows if r.label.startswith("Hg"))
    assert any(r.broadband for r in rows)


def test_wafer_throughput_auto_preset():
    lpp = huv.wafer_throughput(huv.SourceConfig.lpp_sn_13nm5())
    assert lpp["preset"] == "euv" and lpp["status"] == "🔶"
    assert lpp["wafers_per_hour"] == pytest.approx(153.87, abs=0.05)  # WP-C notes: 153.9
    f2 = huv.SourceConfig.f2_laser()
    auto = huv.wafer_throughput(f2)
    assert auto["preset"] == "refractive"
    assert auto["wafers_per_hour"] == pytest.approx(
        f2.wafer_throughput(preset="refractive")["wafers_per_hour"]
    )
    assert huv.wafer_throughput(huv.SourceConfig.lpp_gd_6nm7())["preset"] == "beuv"
    assert huv.wafer_throughput(huv.SourceConfig.hg_i_line()) is None


def test_wafer_throughput_refuses_sources_without_projection_optics():
    # X-ray tubes / betatron (0.15–0.5 nm) and the 25–47 nm lines have no
    # projection-lithography optics preset: "auto" must not quote EUV optics.
    for src in (
        huv.SourceConfig.xray_tube(anode="W"),
        huv.SourceConfig.xray_tube(anode="Cu"),
        huv.SourceConfig.betatron(),
        huv.SourceConfig.lpa_fel_bella_25nm(),
        huv.SourceConfig.hhg_ar_30nm(),
        huv.SourceConfig.sxrl_ar_46nm9(),
    ):
        with pytest.raises(ValueError, match="no projection-lithography optics preset"):
            huv.wafer_throughput(src)
    # Mo/Si band covers the 13.9 nm Ag soft-X-ray laser; La/B covers Tb 6.5 nm.
    assert huv.wafer_throughput(huv.SourceConfig.sxrl_ag_13nm9())["preset"] == "euv"
    assert huv.wafer_throughput(huv.SourceConfig.lpp_tb_6nm5())["preset"] == "beuv"
    assert huv.wafer_throughput(huv.SourceConfig.entangled_noon()) is None
    # An explicit preset is honoured but labelled hypothetical out of band.
    tube = huv.wafer_throughput(huv.SourceConfig.xray_tube(anode="W"), preset="euv")
    assert tube["wafers_per_hour"] == pytest.approx(22.59, abs=0.05)
    assert any(n.startswith("hypothetical") for n in tube["notes"])
    lpp = huv.wafer_throughput(huv.SourceConfig.lpp_sn_13nm5(), preset="euv")
    assert not any(n.startswith("hypothetical") for n in lpp["notes"])
    # A given optics train is used with the nearest preset's mask/field values.
    w = huv.wafer_throughput(huv.SourceConfig.xray_tube(anode="W"), optics_transmission=1.0)
    assert w["preset"] == "beuv" and any("given optics train" in n for n in w["notes"])
