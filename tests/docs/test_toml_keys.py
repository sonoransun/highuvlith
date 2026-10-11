"""TOML key drift: the keys the CLI reads per source family equal the keys the docs list (DESIGN §7.9).

The source of truth is the ``*_KEYS`` arrays in ``crates/highuvlith-cli/src/config.rs``
(one per family; the synchrotron has two, one per ``beamline``). The documented keys of a
family are the union of the backtick-quoted keys in ``docs/configuration.md``'s
``## [source]`` section (the shared-keys table, the family's row of the per-family table
and its ``#### \\`<type>\\`` details) and the family page's ``## TOML keys`` section (which
P6a/P6b add while the configuration tables stay until INT).

Union mode (the default): a family whose keys appear nowhere is skipped with a reason;
``HUV_DOCS_STRICT_TOML=1`` (exported by INT in CI) turns that skip into a failure.
"""

from __future__ import annotations

import os
import re
from pathlib import Path

import pytest

ROOT = Path(__file__).resolve().parents[2]
CONFIG_RS = ROOT / "crates" / "highuvlith-cli" / "src" / "config.rs"
CONFIG_MD = ROOT / "docs" / "configuration.md"
STRICT = os.environ.get("HUV_DOCS_STRICT_TOML") == "1"

#: family type -> (config.rs array names, family pages)
FAMILIES = {
    "vuv": (("VUV_KEYS",), ("sources/vuv-excimer.md", "sources/duv-heritage.md")),
    "lpa_fel": (("LPA_FEL_KEYS",), ("sources/lpa-fel.md",)),
    "lpp": (("LPP_KEYS",), ("sources/lpp.md",)),
    "synchrotron": (("SYNCHROTRON_UNDULATOR_KEYS", "SYNCHROTRON_BM_KEYS"), ("sources/synchrotron.md",)),
    "hhg": (("HHG_KEYS",), ("sources/hhg.md",)),
    "xfel": (("XFEL_KEYS", "XFEL_MACHINE_KEYS"), ("sources/xfel.md",)),
    "ics": (("ICS_KEYS", "ICS_COLLISION_KEYS"), ("sources/inverse-compton.md",)),
    "ssmb": (("SSMB_KEYS",), ("sources/ssmb.md",)),
    "entangled": (("ENTANGLED_KEYS",), ("sources/entangled-photon.md",)),
    "xray_tube": (("XRAY_TUBE_KEYS",), ("sources/xray-tube.md",)),
    "dpp": (("DPP_KEYS",), ("sources/dpp.md",)),
    "sxrl": (("SXRL_KEYS",), ("sources/soft-xray-laser.md",)),
    "betatron": (("BETATRON_KEYS",), ("sources/betatron.md",)),
    "smith_purcell": (("SMITH_PURCELL_KEYS",), ("sources/smith-purcell.md",)),
}

_ARRAY = re.compile(r"const\s+(\w+_KEYS)\s*:\s*&\[&str\]\s*=\s*&\[(.*?)\];", re.S)
_TICK = re.compile(r"`([A-Za-z_][A-Za-z0-9_]*)`")


def code_keys() -> dict[str, set[str]]:
    """Array name -> key set, from config.rs."""
    text = CONFIG_RS.read_text(encoding="utf-8")
    out = {}
    for name, body in _ARRAY.findall(text):
        out[name] = set(re.findall(r'"([^"]+)"', body))
    return out


def _section(text: str, heading_re: str, level: int) -> str:
    """Text of the first heading matching ``heading_re`` up to the next heading of the same or higher level."""
    m = re.search(rf"^{'#' * level} {heading_re}.*$", text, re.M)
    if not m:
        return ""
    rest = text[m.end():]
    n = re.search(rf"^#{{1,{level}}} ", rest, re.M)
    return rest[: n.start()] if n else rest


def documented_keys(family: str) -> tuple[set[str], list[str]]:
    """(keys, sources) — every backtick key the docs list for ``family``."""
    keys: set[str] = set()
    where: list[str] = []
    if CONFIG_MD.exists():
        text = CONFIG_MD.read_text(encoding="utf-8")
        source = _section(text, re.escape("`[source]`"), 2)
        shared = _section(source, "Shared keys", 3)
        keys |= set(_TICK.findall(shared))
        per = _section(source, "Per-family keys", 3)
        for line in per.splitlines():
            if line.startswith("|") and re.match(rf"\|\s*`{re.escape(family)}`", line):
                keys |= set(_TICK.findall(line))
                where.append("configuration.md per-family table")
        details = _section(source, rf"`{re.escape(family)}`", 4)
        if details:
            keys |= set(_TICK.findall(details))
            where.append(f"configuration.md #### `{family}`")
    for rel in FAMILIES[family][1]:
        page = ROOT / "docs" / rel
        if not page.exists():
            continue
        sec = _section(page.read_text(encoding="utf-8"), "TOML keys", 2)
        if sec:
            keys |= set(_TICK.findall(sec))
            where.append(f"{rel} ## TOML keys")
    return keys, where


def test_fixture_self_check(tmp_path):
    text = 'const DEMO_KEYS: &[&str] = &[\n    "type",\n    "sigma",\n];\n'
    assert re.findall(r'"([^"]+)"', _ARRAY.search(text).group(2)) == ["type", "sigma"]
    page = "## Intro\n\ntext\n\n## TOML keys\n\n| Key | Default |\n|---|---|\n| `sigma` | 0.7 |\n| `type` | |\n\n## Next\n\n`other`\n"
    assert set(_TICK.findall(_section(page, "TOML keys", 2))) == {"sigma", "type"}


@pytest.mark.parametrize("family", sorted(FAMILIES))
def test_family_keys_are_documented(family):
    if not CONFIG_RS.exists():
        pytest.skip("crates/highuvlith-cli/src/config.rs absent")
    arrays = code_keys()
    expected: set[str] = set()
    for name in FAMILIES[family][0]:
        assert name in arrays, f"{name} not found in config.rs"
        expected |= arrays[name]
    docs, where = documented_keys(family)
    if not where:
        msg = f"{family}: keys documented nowhere yet (configuration.md row / family page ## TOML keys)"
        if STRICT:
            pytest.fail(msg)
        pytest.skip(msg)
    relevant = docs & (expected | {"type"})
    missing = expected - docs
    assert not missing, f"{family}: keys read by the CLI but documented nowhere ({', '.join(where)}): {sorted(missing)}"
    assert relevant >= expected - {"type"}
