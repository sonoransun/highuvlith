"""Source physics derived from machine parameters, through the Python
bindings: every family's ``derived_quantities()``, the new machine
parameters and presets, and the dose-limited throughput model.

Fixture numbers were computed independently (scipy/mpmath) of the Rust
implementation; see the WP-C notes for the reference script.
"""

import math

import pytest

import highuvlith as huv

HC_EV_NM = 1239.84193
E_CHARGE = 1.602176634e-19


def dq(src, name):
    value = src.derived_quantity(name)
    assert value is not None, f"{src.kind} does not report {name}"
    return value


ALL_PRESETS = [
    lambda: huv.SourceConfig.f2_laser(),
    lambda: huv.SourceConfig.ar2_laser(),
    lambda: huv.SourceConfig.arf_laser(),
    lambda: huv.SourceConfig.krf_laser(),
    lambda: huv.SourceConfig.hg_i_line(),
    lambda: huv.SourceConfig.hg_h_line(),
    lambda: huv.SourceConfig.hg_g_line(),
    lambda: huv.SourceConfig.lpa_fel_bella_25nm(),
    lambda: huv.SourceConfig.lpp_sn_13nm5(),
    lambda: huv.SourceConfig.lpp_gd_6nm7(),
    lambda: huv.SourceConfig.synchrotron_compact_euv(),
    lambda: huv.SourceConfig.synchrotron_liga_bending_magnet(),
    lambda: huv.SourceConfig.hhg_ne_13nm5(),
    lambda: huv.SourceConfig.xfel_flash_13nm5(),
    lambda: huv.SourceConfig.xfel_erl_13nm5(),
    lambda: huv.SourceConfig.ics_compact_euv_13nm5(),
    lambda: huv.SourceConfig.ssmb_euv_13nm5(),
    lambda: huv.SourceConfig.entangled_noon(),
]


@pytest.mark.parametrize("factory", ALL_PRESETS)
def test_every_family_reports_derived_physics(factory):
    src = factory()
    quantities = src.derived_quantities()
    assert len(quantities) >= 3
    for name, value, unit, note in quantities:
        assert name and unit and note
        assert not math.isnan(value), f"{src.kind}.{name} is NaN"
    # photon energy (where reported) is hc / lambda of the source
    energy = src.derived_quantity("photon_energy")
    if energy is not None:
        assert energy == pytest.approx(HC_EV_NM / src.wavelength_nm, rel=1e-9)
    assert src.derived_quantity("no_such_quantity") is None


class TestVuv:
    def test_f2_photon_budget(self):
        src = huv.SourceConfig.f2_laser()
        # 10 mJ / 7.866 eV = 7.935e15 photons per pulse (scipy fixture)
        assert dq(src, "photons_per_pulse") == pytest.approx(7.935278e15, rel=1e-6)
        assert dq(src, "average_power") == pytest.approx(40.0)

    def test_ar2_flagged_hypothetical(self):
        src = huv.SourceConfig.ar2_laser()
        assert dq(src, "demonstrated") == 0.0
        note = [q for q in src.derived_quantities() if q[0] == "relative_bandwidth"][0][3]
        assert "never been demonstrated" in note


class TestHeritage:
    @pytest.mark.parametrize(
        "factory,wavelength,energy_ev",
        [
            (huv.SourceConfig.arf_laser, 193.368, 6.411826),
            (huv.SourceConfig.krf_laser, 248.3, 4.993322),
            (huv.SourceConfig.hg_i_line, 365.0153, 3.396685),
            (huv.SourceConfig.hg_h_line, 404.6563, 3.063938),
            (huv.SourceConfig.hg_g_line, 435.8328, 2.844765),
        ],
    )
    def test_wavelengths_and_photon_energy(self, factory, wavelength, energy_ev):
        src = factory()
        assert src.kind == "vuv"
        assert src.wavelength_nm == pytest.approx(wavelength, abs=1e-12)
        assert dq(src, "photon_energy") == pytest.approx(energy_ev, rel=1e-6)
        # photons per unit dose relative to 13.5 nm EUV
        assert dq(src, "photons_vs_13nm5") == pytest.approx(wavelength / 13.5)

    def test_lasers_vs_lamps(self):
        arf = huv.SourceConfig.arf_laser()
        assert arf.average_power_w == pytest.approx(90.0)
        assert arf.rep_rate_hz == pytest.approx(6000.0)
        # E95 of the 0.2 pm FWHM Gaussian line: 1.6646 x 0.2 pm
        assert dq(arf, "e95_bandwidth") == pytest.approx(0.332928, rel=1e-5)
        assert huv.SourceConfig.krf_laser().average_power_w == pytest.approx(40.0)
        lamp = huv.SourceConfig.hg_i_line()
        assert lamp.average_power_w is None  # CW lamp, power not modeled
        assert lamp.rep_rate_hz == 0.0
        assert lamp.spectral_samples == 1
        assert dq(lamp, "continuous_wave") == 1.0
        assert lamp.derived_quantity("photons_per_pulse") is None
        with pytest.raises(ValueError):
            huv.SourceConfig.hg_g_line(sigma=0.0)

    def test_lamp_images_like_a_monochromatic_source(self):
        src = huv.SourceConfig.hg_i_line(sigma=0.6)
        optics = huv.OpticsConfig(numerical_aperture=0.6)
        mask = huv.MaskConfig.line_space(cd_nm=350.0, pitch_nm=700.0)
        grid = huv.GridConfig(size=64, pixel_nm=21.875)
        engine = huv.SimulationEngine(src, optics, mask, grid=grid, max_kernels=8)
        mono = engine.compute_aerial_image(focus_nm=0.0).image_contrast()
        poly = engine.compute_polychromatic(focus_nm=0.0).image_contrast()
        assert 0.0 < mono <= 1.0
        # One spectral sample: the polychromatic path adds no spurious defocus.
        assert poly == pytest.approx(mono, rel=1e-9)


class TestLpaFel:
    def test_preset_violates_energy_spread_criterion(self):
        src = huv.SourceConfig.lpa_fel_bella_25nm()
        assert dq(src, "resonant_wavelength") == pytest.approx(25.0, rel=1e-9)
        assert dq(src, "pierce_parameter_1d") == pytest.approx(5.3239e-3, rel=1e-4)
        # 1 % LPA energy spread ~ 1.9 rho: no practical gain.
        assert dq(src, "energy_spread_over_rho") > 1.5
        assert dq(src, "undulator_over_saturation_length") < 0.1

    def test_factory_undulator_and_five_percent_rule(self):
        ok = huv.SourceConfig.lpa_fel(
            25.0, undulator_period_mm=20.0, undulator_k=1.6724, energy_spread_rel=1e-3
        )
        assert dq(ok, "energy_spread_over_rho") < 1.0
        with pytest.raises(ValueError):
            huv.SourceConfig.lpa_fel(25.0, undulator_period_mm=20.0, undulator_k=1.2)
        with pytest.raises(ValueError):
            huv.SourceConfig.lpa_fel(25.0, undulator_period_mm=20.0)
        with pytest.raises(ValueError):
            huv.SourceConfig.lpa_fel(25.0, peak_current_a=1000.0)

    def test_bandwidth_derived_from_beam_unless_overridden(self):
        # The 25 nm preset's bandwidth is its own SASE 2 rho lambda (266 pm).
        src = huv.SourceConfig.lpa_fel_bella_25nm()
        sase = 2 * dq(src, "pierce_parameter_1d") * 25.0 * 1e3
        assert sase == pytest.approx(266.197, rel=1e-4)
        assert src.bandwidth_pm == pytest.approx(sase, rel=1e-12)
        assert dq(src, "bandwidth_fwhm") == pytest.approx(sase, rel=1e-12)
        machine = dict(undulator_period_mm=20.0, undulator_k=1.6724, peak_current_a=1000.0)
        derived = huv.SourceConfig.lpa_fel(25.0, **machine)
        assert derived.bandwidth_pm == pytest.approx(dq(derived, "sase_bandwidth_from_rho"))
        pinned = huv.SourceConfig.lpa_fel(25.0, bandwidth_pm=2.5, **machine)
        assert pinned.bandwidth_pm == pytest.approx(2.5)
        # Without an undulator + beam: 0.1 % of lambda.
        assert huv.SourceConfig.lpa_fel(13.5).bandwidth_pm == pytest.approx(13.5)
        with pytest.raises(ValueError):
            huv.SourceConfig.lpa_fel(25.0, bandwidth_pm=-1.0)


class TestLpp:
    def test_power_at_intermediate_focus(self):
        src = huv.SourceConfig.lpp_sn_13nm5()
        # NXE:3400B chain: 21.5 kW x 6 % = 1290 W into 2 pi -> 250 W at IF
        assert src.average_power_w == pytest.approx(250.0)
        assert dq(src, "in_band_emission_2pi") == pytest.approx(1290.0)
        assert dq(src, "etendue_margin") == pytest.approx(21.0085, rel=1e-4)
        assert dq(src, "hvm_power_ratio") == pytest.approx(1.0)
        # Gd in-band power is flagged as a projection.
        gd = huv.SourceConfig.lpp_gd_6nm7()
        note = [q for q in gd.derived_quantities() if q[0] == "in_band_power_at_if"][0][3]
        assert "PROJECTION" in note
        # NXE:3800E class: 500 W at IF, 10 mJ per 50 kHz droplet.
        e = huv.SourceConfig.lpp_sn_13nm5_500w()
        assert e.average_power_w == pytest.approx(500.0)
        assert dq(e, "hvm_power_ratio") == pytest.approx(2.0)

    def test_drive_laser_options(self):
        co2 = huv.SourceConfig.lpp_sn_13nm5(drive_laser="co2")
        tm = huv.SourceConfig.lpp_sn_13nm5(drive_laser="thulium_2um")
        yag = huv.SourceConfig.lpp_sn_13nm5(drive_laser="solid_state_1um")
        assert co2.average_power_w > tm.average_power_w > yag.average_power_w
        # critical density ~ 1/lambda^2: CO2 couples ~100x lower density.
        assert dq(yag, "critical_density") / dq(co2, "critical_density") == pytest.approx(
            (10.6 / 1.064) ** 2, rel=1e-9
        )
        with pytest.raises(ValueError):
            huv.SourceConfig.lpp_sn_13nm5(drive_laser="ruby")

    def test_tb_preset(self):
        assert huv.SourceConfig.lpp_tb_6nm5().wavelength_nm == pytest.approx(6.5)


class TestSynchrotron:
    def test_emittance_derives_coherence(self):
        legacy = huv.SourceConfig.synchrotron_undulator()
        assert legacy.transverse_coherence_fraction == pytest.approx(0.2)
        src = huv.SourceConfig.synchrotron_undulator(
            emittance_x_nm_rad=10.0, emittance_y_nm_rad=0.1
        )
        assert src.transverse_coherence_fraction == pytest.approx(0.0887892, rel=1e-5)
        assert dq(src, "coherent_fraction") == pytest.approx(
            src.transverse_coherence_fraction, rel=1e-12
        )

    def test_absolute_flux_and_sinc2_line(self):
        src = huv.SourceConfig.synchrotron_compact_euv()
        # Kim/Attwood central-cone power at 200 mA: 0.2324 W
        assert src.average_power_w == pytest.approx(0.232372, rel=1e-5)
        # sinc^2 FWHM = 0.8859 / (nN) of the line
        assert src.bandwidth_pm / (src.wavelength_nm * 1e3) == pytest.approx(
            0.0088589, rel=1e-4
        )
        # Flux scales with ring current.
        half = huv.SourceConfig.synchrotron_undulator(ring_current_ma=100.0)
        full = huv.SourceConfig.synchrotron_undulator(ring_current_ma=200.0)
        assert full.average_power_w / half.average_power_w == pytest.approx(2.0)

    def test_bending_magnet_power_per_mrad(self):
        src = huv.SourceConfig.synchrotron_liga_bending_magnet()
        assert dq(src, "power_per_mrad") == pytest.approx(19.7974, rel=1e-5)
        assert src.average_power_w is None


class TestHhg:
    def test_phase_matching_and_efficiency(self):
        ne = huv.SourceConfig.hhg_ne_13nm5()
        assert dq(ne, "keldysh_parameter") == pytest.approx(0.671891, rel=1e-5)
        assert dq(ne, "critical_ionization") == pytest.approx(0.008613, rel=1e-3)
        # A 250 W harmonic would need a GW-class driver.
        assert dq(ne, "driver_power_for_hvm") > 1e9

    def test_driver_power_derives_harmonic_power(self):
        src = huv.SourceConfig.hhg(
            gas="argon", driver_intensity_w_cm2=2e14, harmonic=27, driver_average_power_w=50.0
        )
        assert src.average_power_w == pytest.approx(5e-4, rel=1e-9)
        # 0.5 mW at 41.8 eV is above the measured record: a projection.
        assert dq(src, "power_vs_measured_record") > 1.0

    def test_default_powers_anchored_to_measured_records(self):
        # Assumed drivers: default 5 W, Ar preset 3 W, Ne preset 20 W.
        ar = huv.SourceConfig.hhg_ar_30nm()
        assert ar.average_power_w == pytest.approx(3e-5, rel=1e-9)
        ne = huv.SourceConfig.hhg_ne_13nm5()
        assert 4.3e-7 <= ne.average_power_w <= 1e-6
        generic = huv.SourceConfig.hhg()
        assert generic.average_power_w == pytest.approx(ne.average_power_w / 4, rel=1e-9)
        for src in (ar, ne, generic):
            assert dq(src, "power_vs_measured_record") <= 1.0
        assert dq(ne, "measured_power_record") == pytest.approx(1.05e-6, rel=1e-2)

    def test_comb_passband(self):
        comb = huv.SourceConfig.hhg(
            gas="argon",
            driver_intensity_w_cm2=2e14,
            harmonic=27,
            full_comb=True,
            comb_passband_nm=(25.0, 35.0),
        )
        assert dq(comb, "comb_line_count") == 5
        as_list = huv.SourceConfig.hhg(
            gas="argon", driver_intensity_w_cm2=2e14, harmonic=27,
            full_comb=True, comb_passband_nm=[25.0, 35.0],
        )
        assert dq(as_list, "comb_line_count") == 5
        with pytest.raises(ValueError):
            huv.SourceConfig.hhg(full_comb=True, comb_passband_nm=[25.0])
        # One spectral sample per harmonic in comb mode would give one TCC per
        # line; the weights still sum to 1 over the filtered comb.
        assert comb.bandwidth_pm > 0
        assert huv.SourceConfig.hhg_ar_30nm().wavelength_nm == pytest.approx(800 / 27)


class TestXfel:
    def test_flash_rho_derived(self):
        src = huv.SourceConfig.xfel_flash_13nm5()
        rho = dq(src, "pierce_parameter_1d")
        assert rho == pytest.approx(2.31367e-3, rel=1e-5)
        assert src.bandwidth_pm == pytest.approx(2 * rho * 13.5e3, rel=1e-9)

    def test_machine_class_presets(self):
        cw = huv.SourceConfig.xfel_cw_sc_13nm5()
        erl = huv.SourceConfig.xfel_erl_13nm5()
        assert cw.average_power_w == pytest.approx(134.95, rel=1e-4)
        assert erl.average_power_w == pytest.approx(10536.6, rel=1e-4)
        assert dq(erl, "beam_power_average") == pytest.approx(7.8e6, rel=1e-9)
        assert erl.electron_energy_mev == pytest.approx(800.0)


class TestIcs:
    def test_derived_yield_and_gap(self):
        src = huv.SourceConfig.ics_compact_euv_13nm5()
        assert dq(src, "photons_per_collision") == pytest.approx(1.713257e6, rel=1e-6)
        assert dq(src, "collected_fraction") == pytest.approx(0.0282538, rel=1e-5)
        assert src.average_power_w == pytest.approx(7.12266e-5, rel=1e-5)
        assert 3e6 < dq(src, "hvm_power_gap") < 4e6

    def test_factory_collision_parameters(self):
        base = huv.SourceConfig.ics()
        assert base.average_power_w == pytest.approx(1e-5)  # stored placeholder
        derived = huv.SourceConfig.ics(
            bunch_charge_pc=100.0, laser_pulse_energy_mj=10.0, rep_rate_hz=1e8
        )
        assert derived.average_power_w == pytest.approx(7.12266e-5, rel=1e-5)
        with pytest.raises(ValueError):
            huv.SourceConfig.ics(bunch_charge_pc=-1.0)


class TestSsmb:
    def test_quadratic_bunching_sensitivity(self):
        preset = huv.SourceConfig.ssmb_euv_13nm5()
        assert preset.average_power_w == pytest.approx(1000.0)
        assert dq(preset, "bunching_factor") == pytest.approx(0.145800, rel=1e-5)
        assert dq(preset, "microbunch_rms_length") == pytest.approx(4.21641, rel=1e-5)
        weak = huv.SourceConfig.ssmb(bunching_factor=0.1)
        assert weak.average_power_w == pytest.approx(470.421, rel=1e-5)
        weaker = huv.SourceConfig.ssmb(bunching_factor=0.05)
        assert weak.average_power_w / weaker.average_power_w == pytest.approx(4.0)
        # Without radiator fields the stored projection is reported.
        assert huv.SourceConfig.ssmb(average_power_w=750.0).average_power_w == 750.0


class TestEntangled:
    def test_exposure_time_range(self):
        src = huv.SourceConfig.entangled_noon(pair_rate_hz=1e6)
        # DEFAULT (independent bounds 1e-23..1e-25 cm^2): >= 1e16 s .. 1e18 s
        assert dq(src, "exposure_time_bound") == pytest.approx(1e16)
        assert dq(src, "exposure_time_bound_tight") == pytest.approx(1e18)
        # OPTIMISTIC (disputed claims 1e-17..1e-21 cm^2): 1e10 s .. 1e14 s
        assert dq(src, "exposure_time_claims_optimistic") == pytest.approx(1e10)
        assert dq(src, "exposure_time_claims_pessimistic") == pytest.approx(1e14)
        bright = huv.SourceConfig.entangled_noon(pair_rate_hz=1e9)
        assert dq(bright, "exposure_time_bound") == pytest.approx(1e13)
        # Entangled-regime ceiling vs an HVM wafer photon rate
        assert dq(src, "entangled_flux_ceiling") == pytest.approx(1e13)
        assert dq(src, "hvm_rate_over_flux_ceiling") > 1e4
        with pytest.raises(ValueError):
            huv.SourceConfig.entangled_noon(pair_rate_hz=-1.0)


class TestThroughput:
    def test_euv_hvm_like(self):
        src = huv.SourceConfig.lpp_sn_13nm5()
        r = src.wafer_throughput(dose_mj_cm2=30.0, cd_nm=16.0)
        assert r["power_at_wafer_w"] == pytest.approx(250.0 * 0.7**10 * 0.65, rel=1e-9)
        assert r["fields_per_wafer"] == 84
        assert 100.0 < r["wafers_per_hour"] < 200.0
        assert r["pulses_per_point"] > 50.0
        # 30 mJ/cm^2 into a (16 nm)^2 square at 13.5 nm: ~5220 photons.
        assert r["photons_per_cd_square"] == pytest.approx(5219.6, rel=1e-4)
        assert r["relative_shot_noise"] == pytest.approx(
            1 / math.sqrt(r["photons_per_cd_square"]), rel=1e-12
        )

    def test_power_ranking_and_saturation(self):
        low = huv.SourceConfig.ics_compact_euv_13nm5().wafer_throughput()
        lpp = huv.SourceConfig.lpp_sn_13nm5().wafer_throughput()
        erl = huv.SourceConfig.xfel_erl_13nm5().wafer_throughput()
        assert low["wafers_per_hour"] < 1e-3
        assert lpp["wafers_per_hour"] < erl["wafers_per_hour"]
        # Overhead ceiling: 3600 / (84 x 0.1 s + 10 s)
        assert erl["wafers_per_hour"] < 3600.0 / (84 * 0.1 + 10.0)

    def test_stage_limit_and_no_power(self):
        src = huv.SourceConfig.xfel_erl_13nm5()
        r = src.wafer_throughput(max_scan_speed_mm_s=500.0)
        assert r["stage_limited"] is True
        assert r["scan_speed_mm_s"] == pytest.approx(500.0)
        bm = huv.SourceConfig.synchrotron_liga_bending_magnet()
        assert bm.wafer_throughput() is None
        with pytest.raises(ValueError):
            src.wafer_throughput(dose_mj_cm2=0.0)
