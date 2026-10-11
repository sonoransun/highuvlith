"""``highuvlith.viz.style`` carries the same categorical series as ``docs/figures/tokens.py`` (DESIGN §4.1).

This test pins **exactly** that — the eight light and eight dark series hexes — and nothing
else: viz's surfaces (``#fcfcfb`` / ``#1a1a19``) are the library's own plotting chrome and
differ from ``tokens.THEMES`` by design, viz has no grade colours, and
``python/highuvlith/viz/style.py`` is edited by nobody in the docs redesign, so the test must
never be able to fail on a value nobody is allowed to change.
"""

from __future__ import annotations

import sys
from pathlib import Path

import pytest

ROOT = Path(__file__).resolve().parents[2]
if str(ROOT / "docs" / "figures") not in sys.path:
    sys.path.insert(0, str(ROOT / "docs" / "figures"))

import tokens  # noqa: E402


@pytest.mark.parametrize("theme", ["light", "dark"])
def test_viz_categorical_equals_tokens_series(theme):
    viz_style = pytest.importorskip("highuvlith.viz.style")
    assert tuple(viz_style.THEMES[theme]["categorical"]) == tuple(tokens.SERIES[theme])


def test_series_have_eight_slots():
    assert len(tokens.SERIES["light"]) == len(tokens.SERIES["dark"]) == 8
