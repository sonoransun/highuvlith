"""Masks, image metrics and dose-aware process windows (WP-B).

Engine-free tests use closed-form synthetic images; the few engine tests use
1:1 lines on a 300 nm pitch in a one-period (commensurate) field, whose first
diffraction order lies inside NA/lambda at NA 0.75, 157.63 nm.
"""

import math

import numpy as np
import pytest

import highuvlith as huv

P = 180.0  # synthetic pitch (nm)


def _engine_x(n: int, period: float = P) -> np.ndarray:
    h = period / n
    return -period / 2 + (np.arange(n) + 0.5) * h


def _synthetic_profiles(focuses, a=0.5, b0=0.45, z0=40.0, s=150.0, n=128):
    """I(x; z) = a - b(z) cos(2 pi x / p), b(z) = b0 exp(-((z - z0)/s)^2)."""
    x = _engine_x(n)
    rows = [a - b0 * math.exp(-(((z - z0) / s) ** 2)) * np.cos(2 * np.pi * x / P) for z in focuses]
    return x, np.array(rows)


def _line_engine(cd=150.0, pitch=300.0, n=64):
    source = huv.SourceConfig.f2_laser(sigma=0.5)
    optics = huv.OpticsConfig(numerical_aperture=0.75)
    mask = huv.MaskConfig.line_space(cd_nm=cd, pitch_nm=pitch)
    grid = huv.GridConfig(size=n, pixel_nm=pitch / n)
    return source, optics, mask, grid


class TestMetricsAerialResult:
    def test_cd_tones_are_complementary(self):
        source, optics, mask, grid = _line_engine()
        engine = huv.SimulationEngine(source, optics, mask, grid=grid, max_kernels=10)
        img = engine.compute_aerial_image(focus_nm=0.0)
        dark = img.cd(threshold=0.3, tone="dark")
        bright = img.cd(threshold=0.3, tone="bright")
        assert dark is not None and bright is not None
        # One line + one space per period: the widths tile the 300 nm field.
        assert dark + bright == pytest.approx(300.0, abs=1e-6)
        feats = img.features(threshold=0.3, tone="dark")
        assert len(feats) == 1
        left, right, width, centre = feats[0]
        assert width == pytest.approx(dark, abs=1e-9)
        assert right - left == pytest.approx(width, abs=1e-9)

    def test_nils_and_log_slope_consistent(self):
        source, optics, mask, grid = _line_engine()
        engine = huv.SimulationEngine(source, optics, mask, grid=grid, max_kernels=10)
        img = engine.compute_aerial_image(focus_nm=0.0)
        cd = img.cd(threshold=0.3)
        ils = img.image_log_slope(threshold=0.3)
        nils = img.nils_periodic(threshold=0.3)
        assert nils > 0 and ils > 0
        assert nils == pytest.approx(cd * ils, rel=1e-12)
        assert img.nils_periodic(threshold=0.3, width_nm=150.0) == pytest.approx(150.0 * ils, rel=1e-12)

    def test_invalid_tone_raises(self):
        source, optics, mask, grid = _line_engine()
        engine = huv.SimulationEngine(source, optics, mask, grid=grid, max_kernels=10)
        img = engine.compute_aerial_image(focus_nm=0.0)
        with pytest.raises(ValueError, match="tone"):
            img.cd(threshold=0.3, tone="grey")


class TestMetricsMeasureCd:
    def test_measure_cd_dose_aware_with_dose_to_clear(self):
        source, optics, mask, grid = _line_engine()
        engine = huv.SimulationEngine(source, optics, mask, grid=grid, max_kernels=10)
        # E_th = 9 mJ/cm^2 at 30 mJ/cm^2 is the 0.3 intensity contour.
        at_nominal = engine.measure_cd(dose_mj_cm2=30.0, dose_to_clear_mj_cm2=9.0)
        assert at_nominal == pytest.approx(engine.measure_cd(threshold=0.3), abs=1e-9)
        low = engine.measure_cd(dose_mj_cm2=25.0, dose_to_clear_mj_cm2=9.0)
        high = engine.measure_cd(dose_mj_cm2=35.0, dose_to_clear_mj_cm2=9.0)
        # A dark (opaque) line narrows as the dose rises.
        assert low > at_nominal > high
        img = engine.compute_aerial_image(focus_nm=0.0)
        assert img.cd(threshold=0.3, tone="dark") == pytest.approx(at_nominal, abs=1e-9)


class TestProcessWindowSynthetic:
    """Closed forms for I = a - b(z) cos(2 pi x / p) with E_th = 9 mJ/cm^2."""

    focuses = [40.0 - 200.0 + 10.0 * k for k in range(41)]
    doses = [15.0 + k for k in range(31)]

    def _pw(self):
        x, profiles = _synthetic_profiles(self.focuses)
        return huv.ProcessWindowResult.from_profiles(
            x.tolist(),
            self.focuses,
            profiles,
            self.doses,
            dose_to_clear_mj_cm2=9.0,
            tone="dark",
            cd_target_nm=60.0,
            cd_tolerance_pct=10.0,
        )

    def test_dose_limits_at_best_focus(self):
        pw = self._pw()
        j0 = self.focuses.index(40.0)
        d_min, d_max = pw.dose_limits[j0]
        assert d_min == pytest.approx(28.3939877261, rel=2e-4)
        assert d_max == pytest.approx(38.2171063458, rel=2e-4)
        assert pw.tone == "dark"
        assert pw.dose_to_clear_mj_cm2 == pytest.approx(9.0)

    def test_best_focus_dose_to_size_iso_focal(self):
        pw = self._pw()
        assert pw.best_focus() == pytest.approx(40.0, abs=0.5)
        assert pw.dose_to_size() == pytest.approx(32.727272727273, rel=1e-4)
        # CD = p/2 at every focus when the threshold equals the mean a = 0.5.
        assert pw.iso_focal_dose() == pytest.approx(18.0, rel=1e-4)
        assert pw.depth_of_focus() == pytest.approx(120.6557951451, rel=5e-3)
        assert pw.exposure_latitude() == pytest.approx(29.4939416821, rel=2e-3)

    def test_ed_window(self):
        pw = self._pw()
        r5 = pw.dof_at_el(5.0)
        assert r5["dof_nm"] == pytest.approx(160.7446581188, rel=3e-3)
        assert r5["exposure_latitude_pct"] >= 5.0 - 1e-9
        dof, el = pw.el_vs_dof(n_points=11)
        assert dof.shape == (11,) and el.shape == (11,)
        assert np.all(np.diff(el) <= 1e-9)
        assert pw.dof_at_el(40.0) is None
        summary = pw.summary()
        assert summary["best_focus_nm"] == pytest.approx(40.0, abs=0.5)
        assert summary["dof_at_5pct_el"]["dof_nm"] == pytest.approx(r5["dof_nm"])

    def test_cd_matrix_depends_on_dose(self):
        pw = self._pw()
        cd = np.asarray(pw.cd_matrix)
        j0 = self.focuses.index(40.0)
        exact = [(P / math.pi) * math.acos((0.5 - 9.0 / d) / 0.45) for d in self.doses]
        # NaN where the line merges (low dose) — compare the printable rows.
        finite = np.isfinite(cd[:, j0])
        assert finite.sum() > 10
        np.testing.assert_allclose(cd[finite, j0], np.array(exact)[finite], atol=2e-3)


class TestProcessWindowTabulated:
    def test_linear_fem(self):
        doses = [30.0, 40.0, 50.0]
        focuses = [-50.0, 0.0, 50.0]
        cd = np.array([[100.0 - d] * 3 for d in doses])
        pw = huv.ProcessWindowResult.from_cd_matrix(doses, focuses, cd, cd_target_nm=60.0, cd_tolerance_pct=10.0)
        for lim in pw.dose_limits:
            assert lim == pytest.approx((34.0, 46.0))
        assert pw.exposure_latitude() == pytest.approx(30.0)
        assert pw.dose_to_size() == pytest.approx(40.0)
        assert pw.tone is None and pw.dose_to_clear_mj_cm2 is None

    def test_shape_mismatch_rejected(self):
        with pytest.raises(ValueError):
            huv.ProcessWindowResult.from_cd_matrix([30.0, 40.0], [0.0], np.zeros((3, 1)))


class TestProcessWindowEngine:
    def test_legacy_signature_is_dose_aware(self):
        source, optics, mask, grid = _line_engine()
        batch = huv.BatchSimulator(source, optics, mask, grid, max_kernels=10)
        pw = batch.process_window(
            doses=[25.0, 30.0, 35.0],
            focuses=[-100.0, 0.0, 100.0],
            cd_threshold=0.3,
            cd_target_nm=150.0,
        )
        cd = np.asarray(pw.cd_matrix)
        assert cd.shape == (3, 3)
        # Dose rows used to be identical; now the line narrows with dose.
        assert cd[0, 1] > cd[1, 1] > cd[2, 1]
        assert pw.dose_to_clear_mj_cm2 == pytest.approx(9.0)
        explicit = batch.process_window_threshold(
            doses=[25.0, 30.0, 35.0],
            focuses=[-100.0, 0.0, 100.0],
            dose_to_clear_mj_cm2=9.0,
            cd_target_nm=150.0,
        )
        np.testing.assert_allclose(np.asarray(explicit.cd_matrix), cd, equal_nan=True)
        assert "DOF" in repr(pw)


# ---------------------------------------------------------------------------
# Mask geometry, exact spectrum and commensurate grids
# ---------------------------------------------------------------------------


def _orders(n: int) -> np.ndarray:
    k = np.arange(n)
    return np.where(k < n // 2, k, k - n)


class TestMaskGeometry:
    def test_line_space_lines_are_opaque_with_width_cd(self):
        mask = huv.MaskConfig.line_space(65.0, 180.0)
        grid = mask.commensurate_grid(size=256, target_pixel_nm=1.0)
        t = mask.rasterize(grid)
        assert t.dtype == np.complex128 and t.shape == (256, 256)
        # Exact area coverage: clear fraction 1 - cd/pitch.
        assert t.real.mean() == pytest.approx(1.0 - 65.0 / 180.0, abs=1e-12)
        # The line is centred at x = 0: the two central columns are opaque.
        assert abs(t[0, 127]) < 1e-12 and abs(t[0, 128]) < 1e-12

    def test_incommensurate_pitches_no_longer_alias(self):
        # The old 10-period construction imaged (90, 190) and (100, 200)
        # identically on the 256 nm field.
        grid = huv.GridConfig(size=256, pixel_nm=1.0)
        a = huv.MaskConfig.line_space(90.0, 190.0).rasterize(grid)
        b = huv.MaskConfig.line_space(100.0, 200.0).rasterize(grid)
        assert np.abs(a - b).sum() > 10.0

    def test_orientation_and_default_pitch_y(self):
        grid = huv.GridConfig.commensurate(180.0, size=64, target_pixel_nm=3.0)
        v = huv.MaskConfig.line_space(65.0, 180.0).rasterize(grid)
        h = huv.MaskConfig.line_space(65.0, 180.0, orientation="horizontal").rasterize(grid)
        np.testing.assert_array_equal(h, v.T)
        with pytest.raises(ValueError, match="orientation"):
            huv.MaskConfig.line_space(65.0, 180.0, orientation="diagonal")
        ch = huv.MaskConfig.contact_hole(50.0, 150.0)
        assert ch.periodicity() == (150.0, None)
        assert ch.dark_field

    def test_from_features_round_trip_and_validation(self):
        feats = [
            {"type": "rect", "x": 0.0, "y": 0.0, "w": 40.0, "h": 40.0},
            {"type": "gray_rect", "x": 30.0, "y": 0.0, "w": 10.0, "h": 10.0, "transmittance": 0.25},
            {"type": "polygon", "vertices": [(-60.0, -60.0), (-40.0, -60.0), (-60.0, -40.0)]},
        ]
        mask = huv.MaskConfig.from_features(feats, dark_field=True)
        back = mask.features()
        assert [f["type"] for f in back] == ["rect", "gray_rect", "polygon"]
        assert back[1]["transmittance"] == 0.25
        with pytest.raises(ValueError, match="unknown feature type"):
            huv.MaskConfig.from_features([{"type": "circle", "r": 3.0}])
        with pytest.raises(ValueError):
            huv.MaskConfig.from_features([{"type": "rect", "x": 0.0, "y": 0.0, "w": -1.0, "h": 1.0}])


class TestExactSpectrum:
    def test_clear_mask_dc_is_n_squared(self):
        mask = huv.MaskConfig.from_features([], dark_field=False)
        s = mask.spectrum(huv.GridConfig(size=32, pixel_nm=2.0))
        assert s[0, 0] == 32 * 32
        assert np.count_nonzero(s) == 1
        assert mask.spectrum_method(huv.GridConfig(size=32, pixel_nm=2.0)) == "analytic"

    def test_rect_matches_closed_form(self):
        # S[k] = N^2 c(m) exp(-i pi (mx + my)(N-1)/N), c = separable sinc product.
        n, p = 64, 4.0
        L = n * p
        x0, y0, w, h = 3.3, -5.2, 41.7, 60.1
        mask = huv.MaskConfig.from_features(
            [{"type": "rect", "x": x0, "y": y0, "w": w, "h": h}], dark_field=True
        )
        s = mask.spectrum(huv.GridConfig(size=n, pixel_nm=p))
        m = _orders(n)
        cx = (w / L) * np.sinc(m * w / L) * np.exp(-2j * np.pi * m * x0 / L)
        cy = (h / L) * np.sinc(m * h / L) * np.exp(-2j * np.pi * m * y0 / L)
        ph = np.exp(-1j * np.pi * m * (n - 1) / n)
        expected = n * n * np.outer(cy * ph, cx * ph)
        np.testing.assert_allclose(s, expected, rtol=0, atol=1e-9 * abs(expected[0, 0]))

    def test_matches_fft_of_raster_in_fine_pixel_limit(self):
        mask = huv.MaskConfig.from_features(
            [{"type": "rect", "x": 3.3, "y": -5.2, "w": 41.7, "h": 60.1}], dark_field=True
        )

        def low_order_error(n):
            grid = huv.GridConfig(size=n, pixel_nm=256.0 / n)
            exact = mask.spectrum(grid)
            fft = np.fft.fft2(mask.rasterize(grid))
            idx = np.r_[0:4, n - 3 : n]
            return np.abs(fft[np.ix_(idx, idx)] - exact[np.ix_(idx, idx)]).max() / abs(exact[0, 0])

        coarse, fine = low_order_error(64), low_order_error(256)
        assert fine < 5e-4
        assert fine < coarse / 3.0

    def test_centred_rect_images_centred(self):
        # Gaussian-apodized inverse FFT of the exact spectrum is real and
        # mirror-symmetric about x = 0 on the pixel centres.
        n = 64
        mask = huv.MaskConfig.from_features(
            [{"type": "rect", "x": 0.0, "y": 0.0, "w": 37.3, "h": 51.9}], dark_field=True
        )
        s = mask.spectrum(huv.GridConfig(size=n, pixel_nm=4.0))
        m = _orders(n)
        field = np.fft.ifft2(s * np.exp(-(m[:, None] ** 2 + m[None, :] ** 2) / 36.0))
        assert np.abs(field.imag).max() < 1e-12
        np.testing.assert_allclose(field.real, field.real[:, ::-1], atol=1e-12)
        np.testing.assert_allclose(field.real, field.real[::-1, :], atol=1e-12)


class TestCommensurateGrid:
    def test_grid_commensurate_values(self):
        g = huv.GridConfig.commensurate(180.0, size=256, target_pixel_nm=1.0)
        assert g.pixel_nm == pytest.approx(0.703125, abs=1e-15)
        assert g.is_commensurate_with(180.0)
        g2 = huv.GridConfig.commensurate(150.0, 200.0, size=1024, target_pixel_nm=1.0)
        assert g2.field_size_nm() == pytest.approx(600.0, abs=1e-9)
        assert g2.periods_in_field(150.0) == pytest.approx(4.0)
        with pytest.raises(ValueError):
            huv.GridConfig.commensurate(100.0, 100.0 * math.sqrt(2.0))

    def test_check_commensurate_error_suggests_fix(self):
        mask = huv.MaskConfig.line_space(65.0, 180.0)
        with pytest.raises(ValueError, match=r"GridConfig::commensurate\(180, None, 256, 1\)"):
            mask.check_commensurate(huv.GridConfig(size=256, pixel_nm=1.0))
        mask.check_commensurate(mask.commensurate_grid(256, 1.0))

    def test_engine_warns_on_incommensurate_grid(self):
        source = huv.SourceConfig.f2_laser(sigma=0.5)
        optics = huv.OpticsConfig(numerical_aperture=0.75)
        mask = huv.MaskConfig.line_space(150.0, 300.0)
        with pytest.warns(UserWarning, match="incommensurate"):
            huv.SimulationEngine(source, optics, mask, grid=huv.GridConfig(size=64, pixel_nm=4.0), max_kernels=4)

    def test_api_reports_the_pixel_used(self):
        from highuvlith.api import simulate_line_space, sweep_focus

        r = simulate_line_space(100.0, 300.0, grid_size=64, pixel_nm=4.0)
        # One 300 nm period does not fit 64 px at 4 nm: the pixel coarsens to 300/64.
        assert r.config["pixel_nm"] == pytest.approx(300.0 / 64.0)
        assert r.config["pixel_nm_requested"] == 4.0
        assert r.config["periods_in_field"] == 1
        assert r.cd_nm is not None and 0.0 < r.cd_nm < 300.0
        assert r.nils is not None and r.nils > 0.0
        s = sweep_focus(100.0, 300.0, grid_size=128, pixel_nm=4.0, focus_steps=3)
        assert s["field_nm"] == pytest.approx(s["periods_in_field"] * 300.0)
        # 128 px at 4 nm hold one period: pixel 300/128, not coarser than asked.
        assert s["pixel_nm"] == pytest.approx(300.0 / 128.0)
