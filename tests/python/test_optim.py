"""Optimization / patterning bindings: ILT, fragment OPC, SRAF, SADP/SAQP."""

import numpy as np
import pytest

import highuvlith as huv
from highuvlith import api


@pytest.fixture
def f2():
    return huv.SourceConfig.f2_laser(0.6)


@pytest.fixture
def na075():
    return huv.OpticsConfig(numerical_aperture=0.75)


@pytest.fixture
def grid512():
    # 64 x 8 nm = 512 nm field (several lambda/NA at 157.63 nm, NA 0.75).
    return huv.GridConfig(size=64, pixel_nm=8.0)


@pytest.fixture
def grid768():
    return huv.GridConfig(size=64, pixel_nm=12.0)


def test_ilt_contact_array_adjoint_beats_proxy(f2, na075, grid512):
    target = api.contact_target(90.0, 180.0, 2, grid512)
    assert target.shape == (64, 64)
    # Area-averaged round holes: total area = 4 * pi * 45^2 within 1 %.
    assert abs(target.sum() * 64.0 - 4 * np.pi * 45.0**2) < 0.01 * 4 * np.pi * 45.0**2
    common = dict(cost="resist", threshold=0.25, steepness=50.0, max_iterations=20, max_kernels=8)
    adj = api.optimize_ilt(f2, na075, grid512, target, **common)
    prox = api.optimize_ilt(f2, na075, grid512, target, gradient="proxy", **common)
    hist = adj.cost_history
    assert len(hist) == adj.iterations + 1
    assert np.all(np.diff(hist) <= 0.0), "cost must decrease monotonically"
    assert adj.final_cost < 0.5 * prox.final_cost
    assert adj.pattern_error < prox.pattern_error
    assert adj.mask.shape == (64, 64)
    assert 0.0 < adj.mask.min() and adj.mask.max() < 1.0
    assert adj.termination in {"converged", "max_iterations", "line_search_failed"}


def test_ilt_snapshots_and_validation(f2, na075, grid512):
    target = api.contact_target(100.0, 200.0, 2, grid512)
    r = api.optimize_ilt(
        f2, na075, grid512, target, max_iterations=4, snapshot_every=2, max_kernels=8
    )
    assert [s[0] for s in r.snapshots] == [0, 2, 4]
    with pytest.raises(ValueError):
        api.optimize_ilt(f2, na075, grid512, np.zeros((8, 8)))
    with pytest.raises(ValueError):
        api.optimize_ilt(f2, na075, grid512, target, cost="nonsense")


def test_mask_from_features_is_a_deprecated_alias():
    canonical = huv.MaskConfig.from_features([(0.0, 0.0, 110.0, 500.0), [(-50.0, -50.0), (50.0, -50.0), (0.0, 40.0)]])
    with pytest.warns(DeprecationWarning, match="MaskConfig.from_features"):
        legacy = api.mask_from_features(
            rects=[(0.0, 0.0, 110.0, 500.0)], polygons=[[(-50.0, -50.0), (50.0, -50.0), (0.0, 40.0)]]
        )
    grid = huv.GridConfig(size=32, pixel_nm=20.0)
    np.testing.assert_array_equal(np.asarray(legacy.spectrum(grid)), np.asarray(canonical.spectrum(grid)))


def test_fragment_opc_line_end_converges(f2, na075, grid768):
    mask = huv.MaskConfig.from_features([(0.0, 0.0, 110.0, 500.0)])
    r = api.fragment_opc(f2, na075, mask, grid768, 0.35, max_kernels=12)
    assert r.converged
    assert r.epe_rms_history[0] > 5.0 * r.final_epe_rms_nm
    kinds = r.fragment_kinds
    assert kinds.count("line_end") == 2
    biases = r.fragment_biases
    # Line ends are extended to fight pull-back. Converged value (121 x 121
    # source grid, 64 kernels): 10.1 nm; the default adaptive source sampling
    # gives 9.9 nm (10.2 nm with the pre-WP-A3 grid), so the bound sits below
    # the source-discretization scatter.
    assert all(b > 9.0 for b, k in zip(biases, kinds) if k == "line_end")
    assert len(r.polygons) == 1 and len(r.polygons[0]) >= 4


def test_sraf_insertion_and_dof(grid768):
    study = api.isolated_line_sraf_study(80.0)
    assert len(study.sraf.assists) >= 2
    assert study.sraf.worst_margin >= 0.05
    # Bars are narrow sub-resolution features beside the line.
    for x, _, w, _ in study.sraf.assists:
        assert w < 40.0 and abs(x) > 100.0
    assert study.dof.dof_with_nm > study.dof.dof_without_nm
    assert study.dof.focus_nm.shape == study.dof.cd_with_nm.shape
    assert study.status == "🔶" and "heuristic" in study.notes[0]


def test_sadp_and_saqp_geometry():
    r = api.sadp(128.0, 32.0, 32.0)
    assert r.nominal_pitch_nm == 64.0
    np.testing.assert_allclose(r.pitches_nm, [64.0, 64.0])
    assert abs(r.pitch_walk_nm) < 1e-12
    walked = api.sadp(128.0, 34.0, 32.0)
    assert abs(walked.pitch_walk_nm - 4.0) < 1e-12  # 2W + 2t - P
    q = api.saqp(128.0, 48.0, 16.0, 16.0)
    np.testing.assert_allclose(q.line_cds_nm, [16.0] * 4)
    np.testing.assert_allclose(q.spaces_nm, [16.0] * 4)
    sid = api.sadp(128.0, 36.0, 30.0, tone="spacer_is_dielectric")
    assert sorted(sid.line_cds_nm) == pytest.approx([32.0, 36.0])
    with pytest.raises(ValueError):
        api.sadp(128.0, 68.0, 32.0)  # spacers merge
    with pytest.raises(ValueError):
        api.sadp(128.0, 32.0, 32.0, tone="bogus")
