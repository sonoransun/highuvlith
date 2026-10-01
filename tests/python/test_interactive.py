"""Smoke test for highuvlith.interactive with stand-in ipywidgets.

ipywidgets is an optional extra (``highuvlith[notebook]``) and is not in the
test environment, so minimal stand-ins record the widgets and the initial
``update()`` call runs the real engine path; this catches API drift.
"""

import sys
import types

import matplotlib
import pytest

matplotlib.use("Agg")
import matplotlib.pyplot as plt  # noqa: E402


class _Widget:
    def __init__(self, *children, **kwargs):
        self.children = children
        self.__dict__.update(kwargs)

    def observe(self, fn, names=None):
        pass

    def __enter__(self):
        return self

    def __exit__(self, *exc):
        return False

    def clear_output(self, wait=False):
        pass


def _install_stubs(monkeypatch):
    widgets = types.ModuleType("ipywidgets")
    for name in ("FloatSlider", "Output", "VBox", "HBox", "HTML"):
        setattr(widgets, name, _Widget)
    displayed = []
    ipython = types.ModuleType("IPython")
    display_mod = types.ModuleType("IPython.display")
    display_mod.display = displayed.append
    ipython.display = display_mod
    ipython.get_ipython = lambda: None  # matplotlib probes these two
    ipython.version_info = (9, 0, 0)
    monkeypatch.setitem(sys.modules, "ipywidgets", widgets)
    monkeypatch.setitem(sys.modules, "IPython", ipython)
    monkeypatch.setitem(sys.modules, "IPython.display", display_mod)
    monkeypatch.setattr(plt, "show", lambda *a, **k: None)
    return displayed


@pytest.mark.filterwarnings("error")
def test_interactive_helpers_run_against_current_api(monkeypatch):
    displayed = _install_stubs(monkeypatch)
    from highuvlith import interactive

    interactive.interactive_aerial(grid_size=32)
    interactive.interactive_focus_sweep(grid_size=32)
    assert len(displayed) == 2
    plt.close("all")
