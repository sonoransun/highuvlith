"""Exact imaging-engine features exposed through SimulationEngine:
defocus model selection, per-wavelength imaging, through-focus batches and
kernel bookkeeping."""

import numpy as np
import pytest

import highuvlith as huv


def _engine(**kwargs):
    source = huv.SourceConfig.f2_laser(sigma=0.6)
    optics = huv.OpticsConfig(numerical_aperture=0.75)
    mask = huv.MaskConfig.line_space(cd_nm=80.0, pitch_nm=160.0)
    grid = huv.GridConfig(size=64, pixel_nm=5.0)
    return huv.SimulationEngine(source, optics, mask, grid=grid, max_kernels=16, **kwargs)


def test_diagnostics_report_kernel_bookkeeping():
    info = _engine().imaging_diagnostics()
    assert 0 < info["num_kernels"] <= 16
    assert 0.0 < info["captured_energy_fraction"] <= 1.0 + 1e-12
    assert info["method"] in ("dense_tcc", "dense_gram", "randomized")
    assert info["num_source_points"] > 100
    assert info["defocus_model"] == "exact"
    assert info["support_exceeds_nyquist"] is False


def test_defocus_models_agree_in_focus_only():
    exact = _engine()
    legacy = _engine(defocus_model="kernel_phase")
    a = np.asarray(exact.compute_aerial_image(0.0).intensity)
    b = np.asarray(legacy.compute_aerial_image(0.0).intensity)
    np.testing.assert_allclose(a, b, atol=1e-12)
    assert legacy.imaging_diagnostics()["defocus_model"] == "kernel_phase"
    with pytest.raises(ValueError):
        _engine(defocus_model="bogus")


def test_multiwavelength_matches_polychromatic_for_narrow_band():
    engine = _engine()
    poly = np.asarray(engine.compute_polychromatic(60.0).intensity)
    multi = np.asarray(engine.compute_multiwavelength(60.0).intensity)
    np.testing.assert_allclose(multi, poly, atol=1e-4)


def test_through_focus_matches_single_images():
    engine = _engine()
    focus = [-100.0, 0.0, 100.0]
    batch = engine.compute_through_focus(focus)
    assert len(batch) == 3
    for f, img in zip(focus, batch):
        single = np.asarray(engine.compute_aerial_image(f).intensity)
        np.testing.assert_allclose(np.asarray(img.intensity), single, atol=1e-13)


def test_coherent_source_override():
    engine = _engine(source_points_per_axis=1)
    assert engine.imaging_diagnostics()["num_source_points"] == 1


def _grating_engine(optics, wavelength_nm=None, **kwargs):
    source = huv.SourceConfig.f2_laser(sigma=0.6)
    mask = huv.MaskConfig.line_space(cd_nm=80.0, pitch_nm=160.0)
    grid = huv.GridConfig(size=64, pixel_nm=5.0)
    return huv.SimulationEngine(source, optics, mask, grid=grid, max_kernels=16, **kwargs)


def test_immersion_and_euv_optics_factories():
    wet = huv.OpticsConfig.immersion_193i()
    assert wet.kind == "refractive"
    assert wet.numerical_aperture == 1.35
    assert abs(wet.immersion_index - 1.437) < 1e-12
    custom = huv.OpticsConfig.immersion(numerical_aperture=1.2, immersion_index=1.44)
    assert custom.numerical_aperture == 1.2
    with pytest.raises(ValueError):
        huv.OpticsConfig.immersion(numerical_aperture=1.40, immersion_index=1.437)
    with pytest.raises(ValueError):
        huv.OpticsConfig(numerical_aperture=1.2)
    nxe = huv.OpticsConfig.euv_nxe()
    hna = huv.OpticsConfig.euv_high_na()
    assert nxe.kind == hna.kind == "euv_projection"
    assert (nxe.numerical_aperture, hna.numerical_aperture) == (0.33, 0.55)
    assert hna.immersion_index == 1.0
    hna.add_aberration(9, 0.01)
    with pytest.raises(ValueError):
        huv.OpticsConfig.euv_projection(numerical_aperture=0.55, central_obscuration=1.0)


def test_immersion_engine_runs_above_na_one():
    engine = _grating_engine(huv.OpticsConfig.immersion_193i())
    img = np.asarray(engine.compute_aerial_image(0.0).intensity)
    assert img.min() >= -1e-12 and engine.image_contrast(0.0) > 0.1


def test_clear_field_normalization_and_absolute_mode():
    optics = huv.OpticsConfig.schwarzschild(flare=0.0)
    rel = _grating_engine(optics)
    abs_ = _grating_engine(optics, normalization="absolute")
    c = rel.clear_field_intensity(0.0)
    assert 0.0 < c < 1.0
    info = rel.imaging_diagnostics()
    assert info["normalized"] is True and abs(info["clear_field_intensity"] - c) < 1e-15
    assert abs_.imaging_diagnostics()["normalized"] is False
    a = np.asarray(rel.compute_aerial_image(0.0).intensity)
    b = np.asarray(abs_.compute_aerial_image(0.0).intensity)
    np.testing.assert_allclose(a * c, b, atol=1e-12)
    with pytest.raises(ValueError):
        _grating_engine(optics, normalization="bogus")


def test_vector_settings_are_accepted():
    from highuvlith._native import VectorSettings  # package export is a Phase-2 item

    vs = VectorSettings(polarization="x")
    engine = _grating_engine(huv.OpticsConfig(numerical_aperture=0.75), vector=vs)
    info = engine.imaging_diagnostics()
    assert info["imaging_model"] == "vector"
    img = np.asarray(engine.compute_aerial_image(0.0).intensity)
    assert img.min() >= -1e-12


def test_multilayer_pupil_on_reflective_optics():
    plain = huv.OpticsConfig.euv_nxe()
    assert plain.has_multilayer_pupil is False
    uniform = plain.with_multilayer_pupil([(0.0, 0.0, 0.0, 0.0), (0.0, 0.0, 0.0, 0.0)])
    graded = plain.with_multilayer_pupil([(0.0, 0.0, 0.0, 15.0)] * 2)
    assert uniform.has_multilayer_pupil and graded.has_multilayer_pupil
    assert plain.has_multilayer_pupil is False  # returns a copy
    source = huv.SourceConfig.lpp_sn_13nm5(sigma=0.5)
    mask = huv.MaskConfig.line_space(cd_nm=30.0, pitch_nm=60.0)
    grid = huv.GridConfig(size=32, pixel_nm=60.0 / 16.0)
    engines = [
        huv.SimulationEngine(source, o, mask, grid=grid, max_kernels=12)
        for o in (plain, uniform, graded)
    ]
    imgs = [np.asarray(e.compute_aerial_image(0.0).intensity) for e in engines]
    # Uniform coating: relative image unchanged, absolute clear field |r|^4.
    np.testing.assert_allclose(imgs[0], imgs[1], atol=1e-12)
    assert engines[1].clear_field_intensity() < 0.6 * engines[0].clear_field_intensity()
    # Angle-dependent coating: a different (apodized) image.
    assert np.abs(imgs[2] - imgs[0]).max() > 1e-3
    with pytest.raises(ValueError):
        huv.OpticsConfig(numerical_aperture=0.5).with_multilayer_pupil([(0.0, 0.0, 0.0, 0.0)])
    with pytest.raises(ValueError):
        plain.with_multilayer_pupil([(0.0, 0.0, 0.0, 0.0)], coating="gold")
