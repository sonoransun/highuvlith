"""Integration tests for the lab / compact source families through the Python
bindings: hard X-ray tube, discharge-produced plasma (DPP/LDP), plasma
soft-X-ray lasers, laser-wakefield betatron X-rays, and Smith-Purcell
free-electron grating radiation. Fixture numbers were computed independently
of the Rust implementation (NumPy re-derivations)."""

import math

import pytest

import highuvlith as huv


def _dq(src):
    """Derived quantities as a {name: value} dict."""
    return {name: value for name, value, _unit, _note in src.derived_quantities()}


class TestXrayTube:
    def test_w_preset_physics(self):
        src = huv.SourceConfig.xray_tube()
        assert src.kind == "xray_tube"
        # Photon-weighted mean wavelength of the 250 um Be-filtered spectrum
        # (independent NumPy fixture with the NIST Be table).
        assert src.wavelength_nm == pytest.approx(0.1525608, rel=1e-6)
        # Continuum + W L lines after the window (4pi-equivalent).
        assert src.average_power_w == pytest.approx(8.8599205, rel=1e-6)
        dq = _dq(src)
        # eta = 1.1e-9 * 74 * 60 kV
        assert dq["bremsstrahlung_efficiency"] == pytest.approx(4.884e-3, rel=1e-12)
        assert dq["duane_hunt_min_wavelength_nm"] == pytest.approx(1.23984193 / 60.0)
        # Broadband, incoherent, CW.
        assert src.transverse_coherence_fraction is None
        assert src.rep_rate_hz == 0.0

    def test_cu_anode_ka_dominates(self):
        src = huv.SourceConfig.xray_tube(anode="Cu")
        spectrum = src.xray_spectrum(n_bins=128)
        assert spectrum is not None
        energies = [e for e, _ in spectrum]
        weights = [w for _, w in spectrum]
        assert sum(weights) == pytest.approx(1.0, abs=1e-12)
        assert energies == sorted(energies)
        assert max(energies) < 40.0  # Duane-Hunt limit at 40 kV (preset)
        peak_e = energies[weights.index(max(weights))]
        assert abs(peak_e - 8.05) < 0.4  # Cu K-alpha

    def test_flux_density_scaling(self):
        src = huv.SourceConfig.xray_tube(anode="W", kvp=60.0, current_ma=30.0)
        near = src.spectral_flux_density(500.0, n_bins=59)
        far = src.spectral_flux_density(1000.0, n_bins=59)
        # 1 keV bins; bin centred at 20.5 keV (independent fixture).
        assert near[19][0] == pytest.approx(20.5)
        assert near[19][1] == pytest.approx(1.85110236e7, rel=1e-7)
        assert near[19][1] / far[19][1] == pytest.approx(4.0)
        hot = huv.SourceConfig.xray_tube(anode="W", kvp=60.0, current_ma=60.0)
        assert hot.spectral_flux_density(500.0, n_bins=59)[19][1] == pytest.approx(
            2.0 * near[19][1]
        )

    def test_voltage_hardens_spectrum(self):
        soft = huv.SourceConfig.xray_tube(anode="W", kvp=30.0)
        hard = huv.SourceConfig.xray_tube(anode="W", kvp=90.0)
        assert hard.wavelength_nm < soft.wavelength_nm

    def test_bad_inputs_rejected(self):
        with pytest.raises(ValueError):
            huv.SourceConfig.xray_tube(anode="Xx")
        with pytest.raises(ValueError):
            huv.SourceConfig.xray_tube(kvp=2.0)
        with pytest.raises(ValueError):
            huv.SourceConfig.xray_tube(current_ma=0.0)
        with pytest.raises(ValueError):
            huv.SourceConfig.xray_tube(be_window_um=-1.0)
        with pytest.raises(ValueError):
            huv.SourceConfig.xray_tube().spectral_flux_density(0.0)

    def test_absolute_flux_feeds_liga_manually(self):
        # The CLI/LIGA default uses the tube's RELATIVE spectrum (no exposure
        # time); the absolute flux density is handed over explicitly.
        tube = huv.SourceConfig.xray_tube(anode="W", kvp=60.0, current_ma=30.0)
        flux = tube.spectral_flux_density(100.0, n_bins=59)
        kw = dict(
            resist_thickness_um=100.0, grid_size=32, pixel_nm=400.0, nz=8, warn=False
        )
        absolute = huv.simulate_liga(flux_density=flux, **kw)
        assert absolute.exposure_time_s is not None and absolute.exposure_time_s > 0.0
        relative = huv.simulate_liga(spectrum_table=tube.xray_spectrum(n_bins=59), **kw)
        assert relative.exposure_time_s is None
        # Same spectral shape -> same depth-dose contrast.
        assert absolute.dose_ratio == pytest.approx(relative.dose_ratio, rel=1e-6)

    def test_other_families_have_no_xray_spectrum(self):
        src = huv.SourceConfig.f2_laser()
        assert src.xray_spectrum() is None
        assert src.spectral_flux_density(100.0) is None


class TestDpp:
    def test_sn_power_chain(self):
        src = huv.SourceConfig.dpp_sn_13nm5()
        assert src.kind == "dpp"
        assert src.wavelength_nm == pytest.approx(13.5)
        # 18 kW x 2% = 360 W into 2 pi AT THE SOURCE (the TRINITI continuous
        # level); x (1.5 sr / 2 pi) x 0.4 = 34.3775 W at intermediate focus.
        assert src.average_power_w == pytest.approx(216.0 / (2.0 * math.pi), rel=1e-12)
        dq = _dq(src)
        assert dq["in_band_power_2pi_w"] == pytest.approx(360.0)
        assert dq["in_band_pulse_energy_2pi_mj"] == pytest.approx(180.0)
        assert dq["etendue_usable_fraction"] == pytest.approx(1.0)
        assert dq["hvm_power_gap"] == pytest.approx(
            250.0 * 2.0 * math.pi / 216.0, rel=1e-12
        )

    def test_xe_etendue_limited(self):
        src = huv.SourceConfig.dpp_xe_13nm5()
        dq = _dq(src)
        # G_src = 1 mm x 3 mm x 1.5 sr = 4.5 mm^2 sr > 3.3 mm^2 sr
        assert dq["source_etendue_mm2_sr"] == pytest.approx(4.5)
        assert dq["etendue_usable_fraction"] == pytest.approx(3.3 / 4.5)
        assert src.shot_to_shot_rms == pytest.approx(0.05)

    def test_bad_sigma_rejected(self):
        with pytest.raises(ValueError):
            huv.SourceConfig.dpp_sn_13nm5(sigma=0.0)


class TestSxrl:
    def test_fixed_lasing_lines(self):
        assert huv.SourceConfig.sxrl_ar_46nm9().wavelength_nm == pytest.approx(46.9)
        assert huv.SourceConfig.sxrl_ag_13nm9().wavelength_nm == pytest.approx(13.9)
        assert huv.SourceConfig.sxrl(scheme="cd_13nm2").wavelength_nm == pytest.approx(
            13.2
        )
        assert huv.SourceConfig.sxrl(scheme="mo_18nm9").wavelength_nm == pytest.approx(
            18.9
        )

    def test_ar_capillary_preset(self):
        src = huv.SourceConfig.sxrl_ar_46nm9()
        assert src.kind == "sxrl"
        # 13 uJ x 12 Hz = 0.156 mW (desk-top anchor, Heinbuch et al. 2005).
        assert src.average_power_w == pytest.approx(1.56e-4)
        assert src.pulse_duration_fs == pytest.approx(1.2e6)
        assert src.transverse_coherence_fraction == pytest.approx(0.3)
        # Narrow line: 1e-4 x 46.9 nm = 4.69 pm.
        assert src.bandwidth_pm == pytest.approx(4.69)
        dq = _dq(src)
        assert dq["coherence_length_um"] == pytest.approx(469.0)
        assert dq["interference_min_half_pitch_nm"] == pytest.approx(46.9 / 4.0)
        assert dq["photons_per_pulse"] == pytest.approx(3.0693010e12, rel=1e-7)
        notes = {name: note for name, _v, _u, note in src.derived_quantities()}
        assert "Heinbuch" in notes["average_power_w"]

    def test_overrides_and_errors(self):
        src = huv.SourceConfig.sxrl(
            scheme="ag_13nm9", pulse_energy_uj=5.0, rep_rate_hz=100.0
        )
        assert src.average_power_w == pytest.approx(5e-4)
        with pytest.raises(ValueError):
            huv.SourceConfig.sxrl(scheme="krypton")
        with pytest.raises(ValueError):
            huv.SourceConfig.sxrl(rel_linewidth=0.5)


class TestBetatron:
    def test_derived_physics(self):
        src = huv.SourceConfig.betatron()
        assert src.kind == "betatron"
        assert src.electron_energy_mev == pytest.approx(200.0)
        dq = _dq(src)
        assert dq["betatron_strength_k"] == pytest.approx(8.3352, rel=1e-4)
        assert dq["critical_energy_kev"] == pytest.approx(8.06911, rel=1e-5)
        assert dq["photons_per_shot"] == pytest.approx(8.72986e8, rel=1e-5)
        # Trait wavelength: hc / (0.30792 E_c).
        assert src.wavelength_nm == pytest.approx(0.499002, rel=1e-5)
        assert src.average_power_w == pytest.approx(3.47521e-6, rel=1e-5)

    def test_critical_energy_scales_with_density(self):
        low = _dq(huv.SourceConfig.betatron(plasma_density_cm3=1e19))
        high = _dq(huv.SourceConfig.betatron(plasma_density_cm3=2e19))
        assert high["critical_energy_kev"] / low[
            "critical_energy_kev"
        ] == pytest.approx(2.0)

    def test_spectrum_and_flux(self):
        src = huv.SourceConfig.betatron()
        spectrum = src.xray_spectrum(n_bins=32)
        assert sum(w for _, w in spectrum) == pytest.approx(1.0, abs=1e-12)
        flux = src.spectral_flux_density(1000.0, n_bins=16)
        assert len(flux) == 16
        assert all(v > 0.0 for _, v in flux)

    def test_undulator_regime_rejected(self):
        with pytest.raises(ValueError):
            huv.SourceConfig.betatron(betatron_amplitude_um=0.1)
        with pytest.raises(ValueError):
            huv.SourceConfig.betatron(plasma_density_cm3=1e22)


class TestSmithPurcell:
    def test_default_targets_13nm5(self):
        src = huv.SourceConfig.smith_purcell()
        assert src.kind == "smith_purcell"
        assert src.wavelength_nm == pytest.approx(13.5)
        assert src.electron_energy_mev == pytest.approx(0.03)
        dq = _dq(src)
        # a = lambda * beta(30 keV) at 90 deg; h_int = beta gamma lambda / 4 pi.
        assert dq["interaction_height_nm"] == pytest.approx(0.373484, rel=1e-5)
        # Order-of-magnitude power: alpha * 1e-3 * 100 photons/e at 10 nA.
        assert src.average_power_w == pytest.approx(6.7019e-10, rel=1e-4)
        assert dq["ideal_coupling_power_w"] == pytest.approx(6.7019e-7, rel=1e-4)
        assert dq["hvm_power_gap"] > 1e11
        # Per-nA rate vs the measured vdW-crystal X-ray anchor (~80 /s/nA).
        assert dq["photon_rate_per_na"] == pytest.approx(4.5546492e6, rel=1e-7)

    def test_dispersion_with_explicit_period(self):
        # vdW-lattice scale period: a = 0.335 nm at 30 keV, 90 deg -> 1.0202 nm
        src = huv.SourceConfig.smith_purcell(grating_period_nm=0.335)
        assert src.wavelength_nm == pytest.approx(1.02017145, rel=1e-8)
        # Faster electrons emit shorter wavelengths at a fixed period.
        fast = huv.SourceConfig.smith_purcell(
            grating_period_nm=0.335, electron_energy_kev=200.0
        )
        assert fast.wavelength_nm < src.wavelength_nm

    def test_bad_inputs_rejected(self):
        with pytest.raises(ValueError):
            huv.SourceConfig.smith_purcell(diffraction_order=0)
        with pytest.raises(ValueError):
            huv.SourceConfig.smith_purcell(observation_angle_deg=180.0)
        with pytest.raises(ValueError):
            huv.SourceConfig.smith_purcell(coupling_efficiency=2.0)


@pytest.mark.parametrize(
    "factory",
    [
        lambda: huv.SourceConfig.xray_tube(),
        lambda: huv.SourceConfig.dpp_sn_13nm5(),
        lambda: huv.SourceConfig.sxrl_ar_46nm9(),
        lambda: huv.SourceConfig.betatron(),
        lambda: huv.SourceConfig.smith_purcell(),
    ],
)
def test_derived_quantities_are_finite(factory):
    src = factory()
    dq = src.derived_quantities()
    assert len(dq) >= 7
    for name, value, unit, note in dq:
        assert math.isfinite(value), name
        assert isinstance(unit, str) and isinstance(note, str)
    weights = src.xray_spectrum()  # None for the EUV families
    if weights is not None:
        assert sum(w for _, w in weights) == pytest.approx(1.0, abs=1e-12)


@pytest.mark.parametrize(
    "factory",
    [
        lambda: huv.SourceConfig.dpp_sn_13nm5(),
        lambda: huv.SourceConfig.sxrl_ag_13nm9(),
        lambda: huv.SourceConfig.sxrl_ar_46nm9(),
        lambda: huv.SourceConfig.smith_purcell(),
    ],
)
def test_euv_families_drive_imaging(factory):
    """The narrow-band EUV families drive the projection pipeline end to end
    (the broadband X-ray families are for LIGA shadow printing instead)."""
    source = factory()
    optics = huv.OpticsConfig.schwarzschild(numerical_aperture=0.33)
    cd = max(20.0, 2.0 * source.wavelength_nm)
    mask = huv.MaskConfig.line_space(cd_nm=cd, pitch_nm=3.0 * cd)
    grid = mask.commensurate_grid(size=64, target_pixel_nm=max(1.0, cd / 16.0))
    engine = huv.SimulationEngine(source, optics, mask, grid=grid, max_kernels=8)
    contrast = engine.compute_aerial_image(focus_nm=0.0).image_contrast()
    assert 0.0 < contrast <= 1.0, f"{source.kind}: contrast {contrast}"
