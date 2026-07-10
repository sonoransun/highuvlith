"""Integration tests for the bleeding-edge source families through the
Python bindings: construction, live physics getters, error paths, and a
small end-to-end imaging run per family."""

import pytest

import highuvlith as huv


HC_EV_NM = 1239.84193


class TestLppSource:
    def test_sn_preset(self):
        src = huv.SourceConfig.lpp_sn_13nm5()
        assert src.kind == "lpp"
        assert src.wavelength_nm == pytest.approx(13.5)
        # Power chain is live: 25 kW x 5.5% x 5% = 68.75 W.
        assert src.average_power_w == pytest.approx(68.75)

    def test_gd_beuv_preset(self):
        src = huv.SourceConfig.lpp_gd_6nm7()
        assert src.wavelength_nm == pytest.approx(6.7)

    def test_bad_sigma_rejected(self):
        with pytest.raises(ValueError):
            huv.SourceConfig.lpp_sn_13nm5(sigma=0.0)


class TestSynchrotronSource:
    def test_wavelength_derived_from_machine(self):
        src = huv.SourceConfig.synchrotron_undulator()
        assert src.kind == "synchrotron"
        # 538 MeV, 20 mm, K=1, n=1 -> ~13.5 nm from the resonance condition.
        assert src.wavelength_nm == pytest.approx(13.5, abs=0.2)

    def test_energy_scaling(self):
        low = huv.SourceConfig.synchrotron_undulator(electron_energy_gev=0.538)
        high = huv.SourceConfig.synchrotron_undulator(electron_energy_gev=1.076)
        # Doubling gamma quarters the wavelength.
        assert low.wavelength_nm / high.wavelength_nm == pytest.approx(4.0, rel=0.02)

    def test_even_harmonic_rejected(self):
        with pytest.raises(ValueError):
            huv.SourceConfig.synchrotron_undulator(harmonic=2)

    def test_liga_bending_magnet(self):
        src = huv.SourceConfig.synchrotron_liga_bending_magnet()
        assert src.kind == "synchrotron"


class TestHhgSource:
    def test_ne_13nm5_preset(self):
        src = huv.SourceConfig.hhg_ne_13nm5()
        assert src.kind == "hhg"
        assert src.wavelength_nm == pytest.approx(800.0 / 59.0)
        assert src.transverse_coherence_fraction == pytest.approx(0.9)

    def test_cutoff_violation_rejected(self):
        # Ar at 2e14 W/cm^2 has a 53.6 eV cutoff; harmonic 59 (91.4 eV)
        # cannot be generated.
        with pytest.raises(ValueError):
            huv.SourceConfig.hhg(
                gas="argon", driver_intensity_w_cm2=2e14, harmonic=59
            )

    def test_even_harmonic_rejected(self):
        with pytest.raises(ValueError):
            huv.SourceConfig.hhg(harmonic=26)

    def test_unknown_gas_rejected(self):
        with pytest.raises(ValueError):
            huv.SourceConfig.hhg(gas="unobtainium")


class TestXfelSource:
    def test_sase_jitter_is_live(self):
        src = huv.SourceConfig.xfel_flash_13nm5()
        assert src.kind == "xfel"
        assert src.wavelength_nm == pytest.approx(13.5)
        # SASE pulse-energy statistics: rms = 1/sqrt(M) > 0.
        assert src.shot_to_shot_rms > 0.1

    def test_seeded_stable(self):
        src = huv.SourceConfig.xfel_fermi_seeded()
        assert src.shot_to_shot_rms == pytest.approx(0.02)
        # Seeded bandwidth is far narrower than SASE.
        sase = huv.SourceConfig.xfel_flash_13nm5()
        assert src.bandwidth_pm < sase.bandwidth_pm / 50


class TestIcsSource:
    def test_wavelength_from_kinematics(self):
        src = huv.SourceConfig.ics(target_wavelength_nm=13.5)
        assert src.kind == "ics"
        assert src.wavelength_nm == pytest.approx(13.5)

    def test_impossible_upshift_rejected(self):
        with pytest.raises(ValueError):
            huv.SourceConfig.ics(target_wavelength_nm=13.5, laser_wavelength_nm=10.0)


class TestSsmbSource:
    def test_design_point(self):
        src = huv.SourceConfig.ssmb_euv_13nm5()
        assert src.kind == "ssmb"
        assert src.wavelength_nm == pytest.approx(13.5)
        # Projected kW-class average power reported directly (CW).
        assert src.average_power_w == pytest.approx(1000.0)


class TestEntangledSource:
    def test_noon_reports_physical_wavelength(self):
        src = huv.SourceConfig.entangled_noon(wavelength_nm=157.63, n=2)
        assert src.kind == "entangled"
        # The trait reports the PHYSICAL wavelength; quantum sharpening
        # is applied via the quantum module, not silently.
        assert src.wavelength_nm == pytest.approx(157.63)

    def test_n1_rejected(self):
        with pytest.raises(ValueError):
            huv.SourceConfig.entangled_noon(n=1)

    def test_bad_fidelity_rejected(self):
        with pytest.raises(ValueError):
            huv.SourceConfig.entangled_noon(fidelity=1.5)


@pytest.mark.parametrize(
    "factory,na",
    [
        (lambda: huv.SourceConfig.lpp_sn_13nm5(), 0.33),
        (lambda: huv.SourceConfig.synchrotron_undulator(), 0.33),
        (lambda: huv.SourceConfig.hhg_ne_13nm5(), 0.33),
        (lambda: huv.SourceConfig.xfel_flash_13nm5(), 0.33),
        (lambda: huv.SourceConfig.ics(), 0.33),
        (lambda: huv.SourceConfig.ssmb_euv_13nm5(), 0.33),
        (lambda: huv.SourceConfig.entangled_noon(), 0.75),
    ],
)
def test_end_to_end_imaging_smoke(factory, na):
    """Every source family must drive the full imaging pipeline."""
    source = factory()
    optics = huv.OpticsConfig(numerical_aperture=na)
    # Feature scale ~2x wavelength keeps diffraction orders in the pupil.
    cd = max(20.0, 2.0 * source.wavelength_nm)
    mask = huv.MaskConfig.line_space(cd_nm=cd, pitch_nm=3.0 * cd)
    grid = huv.GridConfig(size=64, pixel_nm=max(1.0, cd / 16.0))

    engine = huv.SimulationEngine(source, optics, mask, grid=grid, max_kernels=8)
    aerial = engine.compute_aerial_image(focus_nm=0.0)
    contrast = aerial.image_contrast()
    assert 0.0 < contrast <= 1.0, f"{source.kind}: contrast {contrast}"
