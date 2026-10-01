"""Tests for highuvlith.viz: every plot helper in both themes, the palette,
light/dark export, and that themes do not leak into matplotlib's rcParams."""

from __future__ import annotations

from dataclasses import dataclass, field
from types import SimpleNamespace

import matplotlib

matplotlib.use("Agg")

import matplotlib.pyplot as plt  # noqa: E402
import numpy as np  # noqa: E402
import pytest  # noqa: E402

import highuvlith as huv  # noqa: E402
from highuvlith import api, viz  # noqa: E402
from highuvlith.viz import style  # noqa: E402
from matplotlib.axes import Axes  # noqa: E402
from matplotlib.figure import Figure  # noqa: E402

THEMES = ("light", "dark")


@pytest.fixture(autouse=True)
def _close_figures():
    yield
    plt.close("all")


@pytest.fixture(scope="module")
def engine():
    source = huv.SourceConfig.f2_laser(0.5)
    optics = huv.OpticsConfig(numerical_aperture=0.75)
    mask = huv.MaskConfig.line_space(100.0, 300.0)
    grid = mask.commensurate_grid(32, 10.0)
    return huv.SimulationEngine(source, optics, mask, grid=grid, max_kernels=4)


@pytest.fixture(scope="module")
def aerial(engine):
    return engine.compute_aerial_image()


@pytest.fixture(scope="module")
def process_window():
    doses = [26.0, 28.0, 30.0, 32.0, 34.0]
    focuses = [-150.0, -75.0, 0.0, 75.0, 150.0]
    cd = np.array(
        [[100.0 - 0.0004 * f**2 + 3.0 * (d - 30.0) for f in focuses] for d in doses]
    )
    cd[0, 0] = np.nan  # a non-printing corner
    return huv.ProcessWindowResult.from_cd_matrix(doses, focuses, cd, 100.0, 10.0)


@pytest.fixture(scope="module")
def volume():
    values = np.random.default_rng(0).uniform(0.2, 1.0, (8, 4, 8))
    return huv.VolumetricResult.from_array(values, (0.0, 80.0), (0.0, 40.0), (0.0, 80.0))


def _check(obj, kind=Axes):
    assert isinstance(obj, kind)
    fig = obj if isinstance(obj, Figure) else obj.figure
    assert fig.axes, "no axes drawn"
    fig.canvas.draw()  # layout and every artist actually render


def test_categorical_palette_is_the_validated_order():
    assert style.THEMES["light"]["categorical"] == (
        "#2a78d6", "#eb6834", "#1baf7a", "#eda100",
        "#e87ba4", "#008300", "#4a3aa7", "#e34948",
    )
    assert style.THEMES["dark"]["categorical"] == (
        "#3987e5", "#d95926", "#199e70", "#c98500",
        "#d55181", "#008300", "#9085e9", "#e66767",
    )
    assert viz.categorical(0, "light") == "#2a78d6"
    assert viz.categorical(2, "dark") == "#199e70"
    with pytest.raises(IndexError):
        viz.categorical(8)


def test_ordinal_and_sequential_ramps_flip_with_the_theme():
    light = viz.ordinal_colors(9, "light")
    dark = viz.ordinal_colors(9, "dark")
    assert light[0] == style.BLUE_RAMP[250] and light[-1] == style.BLUE_RAMP[650]
    assert dark[0] == style.BLUE_RAMP[600] and dark[-1] == style.BLUE_RAMP[200]
    lo_light = matplotlib.colors.to_hex(viz.sequential_cmap("light")(0.0))
    lo_dark = matplotlib.colors.to_hex(viz.sequential_cmap("dark")(0.0))
    assert lo_light == style.BLUE_RAMP[100]
    assert lo_dark == style.BLUE_RAMP[700]
    mid = viz.diverging_cmap("light")(0.5)[:3]  # 256-entry LUT: nearest entry
    expected = matplotlib.colors.to_rgb(style.THEMES["light"]["diverging_mid"])
    np.testing.assert_allclose(mid, expected, atol=2.5 / 255)


def test_use_theme_does_not_leak_rcparams():
    keys = ("figure.facecolor", "axes.facecolor", "text.color", "axes.prop_cycle", "image.cmap")
    before = {k: matplotlib.rcParams[k] for k in keys}
    assert viz.current_theme() == "light"
    with viz.use_theme("dark"):
        assert viz.current_theme() == "dark"
        assert matplotlib.rcParams["axes.facecolor"] == "#1a1a19"
    assert viz.current_theme() == "light"
    assert {k: matplotlib.rcParams[k] for k in keys} == before
    with pytest.raises(ValueError):
        with viz.use_theme("sepia"):
            pass


@pytest.mark.parametrize("theme", THEMES)
def test_imaging_helpers(theme, aerial):
    ax = viz.plot_aerial(aerial, theme=theme)
    _check(ax)
    assert ax.get_facecolor()[:3] == matplotlib.colors.to_rgb(style.THEMES[theme]["surface"])
    _check(viz.plot_cross_section(aerial, threshold=0.3, theme=theme))
    _check(viz.plot_polarization_contrast(np.linspace(0.3, 0.9, 5), theme=theme))
    _check(viz.plot_polarization_contrast([0.4, 0.6], polarizations=("x",), film_n=1.7, theme=theme))


@pytest.mark.parametrize("theme", THEMES)
def test_resist_and_process_window_helpers(theme, engine, process_window):
    _check(viz.plot_resist_profile(engine.compute_resist_profile(), theme=theme))
    ax = viz.plot_bossung(process_window, theme=theme)
    _check(ax)
    assert len(ax.get_lines()) == 5
    _check(viz.plot_ed_window(process_window, theme=theme))
    _check(viz.plot_ed_window(process_window, cd_target_nm=100.0, cd_tolerance_pct=5.0, theme=theme))
    _check(viz.plot_el_vs_dof(process_window, theme=theme))


def test_ed_window_from_a_threshold_model_draws_the_dose_band(engine):
    batch = huv.BatchSimulator(
        engine.source, engine.optics, engine.mask, engine.grid, max_kernels=4
    )
    pw = batch.process_window_threshold(
        [25.0, 30.0, 35.0], [-100.0, 0.0, 100.0], 9.0, cd_target_nm=100.0, cd_tolerance_pct=20.0
    )
    ax = viz.plot_ed_window(pw)
    _check(ax)
    assert ax.collections, "dose band missing"


@pytest.mark.parametrize("theme", THEMES)
def test_source_helpers(theme):
    fig = viz.plot_source(huv.SourceConfig.f2_laser(0.7), n=33, theme=theme)
    _check(fig, Figure)
    assert len(fig.axes) == 3  # spectrum, pupil fill, colorbar
    _check(viz.plot_source(huv.SourceConfig.hhg_ne_13nm5(), n=33, theme=theme), Figure)


@dataclass(frozen=True)
class _Preset:
    label: str
    wavelength_nm: float
    average_power_w: float | None
    maturity: str
    broadband: bool = False
    factory: str = ""
    family: str = ""
    bandwidth_pm: float = 0.0
    photon_energy_ev: float = 0.0
    power_definition: str = ""
    status: str = "✅"
    note: str = ""
    derived: dict = field(default_factory=dict)


@pytest.mark.parametrize("theme", THEMES)
def test_source_landscape(theme):
    presets = [
        _Preset("Sn LPP", 13.5, 250.0, "demonstrated"),
        _Preset("ERL FEL", 13.5, 1.05e4, "projection"),
        _Preset("ICS", 13.5, 7e-5, "theoretical"),
        _Preset("W tube", 0.15, 8.9, "demonstrated", broadband=True),
        _Preset("Hg i-line", 365.0, None, "demonstrated"),
    ]
    ax = viz.plot_source_landscape(presets, theme=theme)
    _check(ax)
    assert ax.get_xscale() == "log" and ax.get_yscale() == "log"
    assert any("Not shown" in t.get_text() and "Hg i-line" in t.get_text() for t in ax.texts)
    _check(viz.plot_source_landscape(presets, annotate=False, theme=theme))


def test_source_landscape_from_the_api_when_available():
    if not hasattr(api, "source_landscape"):
        pytest.skip("api.source_landscape not available")
    _check(viz.plot_source_landscape())


@pytest.mark.parametrize("theme", THEMES)
def test_deep_litho_helpers(theme, volume):
    profile = api.liga_edge_profile(dx_nm=50, energy_bins=10, x_min_nm=-1000, x_max_nm=1000)
    _check(viz.plot_liga_edge_profile(profile, threshold_kj_cm3=2.5, theme=theme))
    liga = api.simulate_liga(grid_size=16, nz=8, energy_bins=10, diffraction="gaussian", warn=False)
    _check(viz.plot_depth_dose(liga, theme=theme))
    z, dose = liga.depth_dose
    _check(viz.plot_depth_dose(z, dose, log=True, theme=theme))
    talbot = api.simulate_talbot(nx=32, carpet_nz=16)
    _check(viz.plot_talbot_carpet(talbot, theme=theme))
    _check(viz.plot_xz_slice(volume, colorbar_label="PAC", theme=theme))
    _check(viz.plot_height_map(volume.depth_map(0.5), theme=theme))


@pytest.mark.parametrize("theme", THEMES)
def test_development_front(theme, volume):
    resist = huv.ResistConfig()
    fmm = huv.develop_fast_marching(volume, resist, 10.0, 10.0, lateral="periodic")
    level_set = api.develop_level_set(volume, resist, 5.0)
    ax = viz.plot_development_front(
        {"level set": level_set.arrival_times, "fast marching": fmm}, 5.0, theme=theme
    )
    _check(ax)
    labels = [t.get_text() for t in ax.get_legend().get_texts()]
    assert labels == ["level set", "fast marching"]
    assert ax.get_ylim()[0] > ax.get_ylim()[1]  # depth increases downward


@pytest.mark.parametrize("theme", THEMES)
def test_optimization_helpers(theme):
    grid = huv.GridConfig(32, 8.0)
    target = api.contact_target(100.0, 200.0, 1, grid)
    result = api.optimize_ilt(
        huv.SourceConfig.f2_laser(0.6),
        huv.OpticsConfig(numerical_aperture=0.75),
        grid,
        target,
        max_iterations=3,
        snapshot_every=1,
        max_kernels=4,
    )
    fig = viz.plot_ilt_evolution(result, target=target, threshold=0.3, theme=theme)
    _check(fig, Figure)
    assert len(fig.axes) == len(result.snapshots) + 2
    dof = SimpleNamespace(
        focus_nm=np.array([-100.0, 0.0, 100.0]),
        cd_without_nm=np.array([np.nan, 80.0, 70.0]),
        cd_with_nm=np.array([78.0, 80.0, 79.0]),
        dof_without_nm=120.0,
        dof_with_nm=200.0,
        gain=200.0 / 120.0,
    )
    _check(viz.plot_sraf_dof(dof, target_cd_nm=80.0, theme=theme))
    _check(viz.plot_sraf_dof(SimpleNamespace(dof=dof, cd_nm=80.0), theme=theme))
    with pytest.raises(ValueError):
        viz.plot_sraf_dof(dof)


def test_export_light_dark_writes_both_themes(tmp_path, aerial):
    paths = viz.export_light_dark(viz.plot_cross_section, tmp_path / "cut", aerial, threshold=0.3)
    assert [p.name for p in paths] == ["cut-light.png", "cut-dark.png"]
    assert all(p.stat().st_size > 1000 for p in paths)
    assert not plt.get_fignums(), "export must close its figures"
