"""The palette cannot fork: the CSS token block of docs/stylesheets/extra.css is exactly what
``docs/figures/tokens.py --css`` prints (DESIGN §4.1). Run: python -m pytest docs/hooks -q

``tokens.py`` lands with the visual foundation group; until it is present the comparison is
skipped and only the marker block's shape is checked.
"""

from __future__ import annotations

import importlib.util
import re
import sys
from pathlib import Path

import pytest

ROOT = Path(__file__).resolve().parents[2]
CSS = ROOT / "docs" / "stylesheets" / "extra.css"
TOKENS = ROOT / "docs" / "figures" / "tokens.py"
_BLOCK = re.compile(r"^/\* huv-tokens:start.*?^/\* huv-tokens:end \*/$", re.S | re.M)


def css_block() -> str:
    text = CSS.read_text(encoding="utf-8")
    blocks = _BLOCK.findall(text)
    assert len(blocks) == 1, "extra.css carries exactly one /* huv-tokens:start */ … /* huv-tokens:end */ block"
    return blocks[0]


def test_extra_css_has_one_token_block_with_every_token() -> None:
    block = css_block()
    for name in (
        "--huv-violet-900", "--huv-cyan-700", "--huv-spectrum", "--huv-implemented", "--huv-simplified",
        "--huv-theoretical", "--huv-planned", "--huv-hue-home", "--huv-hue-history", "--huv-hue-nodes",
        "--huv-hue-future", "--huv-hue-simulator", "--huv-hue-reference", "--huv-section", "--huv-s1",
        "--huv-s8", "--huv-fs-h1", "--huv-fs-lead", "--huv-fs-small", "--huv-fs-micro", "--huv-measure",
        "--huv-radius-pill", "--huv-hairline", "--huv-rule", "--huv-motion", "--huv-chip-tint", "--huv-chip-border",
    ):
        assert name in block, name
    assert '[data-md-color-scheme="slate"]' in block
    assert "do not edit by hand" in block


@pytest.mark.skipif(not TOKENS.exists(), reason="docs/figures/tokens.py is absent on this branch (lands with V0)")
def test_token_block_is_byte_equal_to_tokens_css() -> None:
    spec = importlib.util.spec_from_file_location("huv_tokens", TOKENS)
    module = importlib.util.module_from_spec(spec)
    assert spec.loader is not None
    sys.modules.setdefault("huv_tokens", module)
    spec.loader.exec_module(module)
    generated = module.css()
    assert generated.rstrip("\n") == css_block().rstrip("\n")
