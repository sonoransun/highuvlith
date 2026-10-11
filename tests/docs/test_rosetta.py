"""The canonical run (65 nm lines / 180 nm pitch, F₂ 157.63 nm, NA 0.75, σ 0.7 = ``examples/sim.toml``)
gives the same numbers through every front end (DESIGN §1.2 D6, §7.9).

* Python ``simulate_line_space(65.0, 180.0, na=0.75)`` against the CLI transcript recorded on
  ``docs/cli.md`` (``Contrast: 0.4545 … Printed CD … 41.20 nm … NILS: 0.654``) — the numbers are
  compared with the example runner's tolerances (abs 0.02 / rel 3 %);
* the same against a live ``highuvlith simulate --config examples/sim.toml`` run when the CLI
  binary exists (``target/release/highuvlith`` or ``$HUV_CLI``), else that part skips;
* the Rosetta table on ``docs/reference/index.md`` between ``<!-- rosetta:start -->`` /
  ``<!-- rosetta:end -->`` (written by P8a) must quote the same numbers; skips while the page
  or its markers are absent.
"""

from __future__ import annotations

import os
import re
import subprocess
import sys
from pathlib import Path

import pytest

ROOT = Path(__file__).resolve().parents[2]
ABS_TOL, REL_TOL = 0.02, 0.03
NUM = r"([-+]?\d+(?:\.\d+)?)"


def _close(a: float, b: float) -> bool:
    return abs(a - b) <= max(ABS_TOL, REL_TOL * abs(a))


@pytest.fixture(scope="module")
def python_numbers():
    huv = pytest.importorskip("highuvlith")
    result = huv.simulate_line_space(65.0, 180.0, na=0.75)
    out = {"contrast": float(result.contrast), "nils": float(result.nils)}
    for attr in ("printed_cd_nm", "cd_nm", "printed_cd"):
        if hasattr(result, attr) and getattr(result, attr) is not None:
            out["cd"] = float(getattr(result, attr))
            break
    return out


def _cli_numbers_from_text(text: str) -> dict[str, float]:
    out: dict[str, float] = {}
    m = re.search(r"Contrast:\s*" + NUM, text)
    if m:
        out["contrast"] = float(m.group(1))
    m = re.search(r"NILS:\s*" + NUM, text)
    if m:
        out["nils"] = float(m.group(1))
    m = re.search(r"Printed CD[^:]*:\s*" + NUM + r"\s*nm", text)
    if m:
        out["cd"] = float(m.group(1))
    return out


def test_fixture_self_check():
    assert _cli_numbers_from_text("Results:\n  Contrast:  0.4545\n  Printed CD (dark feature at I = 0.3000): 41.20 nm\n  NILS:      0.654\n") == {
        "contrast": 0.4545, "cd": 41.20, "nils": 0.654}
    assert _close(0.454, 0.4545) and not _close(0.40, 0.4545)


def test_python_matches_the_recorded_cli_transcript(python_numbers):
    page = ROOT / "docs" / "cli.md"
    if not page.exists():
        pytest.skip("docs/cli.md absent")
    recorded = _cli_numbers_from_text(page.read_text(encoding="utf-8"))
    if "contrast" not in recorded:
        pytest.skip("docs/cli.md carries no simulate transcript")
    assert _close(python_numbers["contrast"], recorded["contrast"]), (python_numbers, recorded)
    if "nils" in recorded:
        assert _close(python_numbers["nils"], recorded["nils"]), (python_numbers, recorded)
    if "cd" in recorded and "cd" in python_numbers:
        assert _close(python_numbers["cd"], recorded["cd"]), (python_numbers, recorded)


def _cli_binary() -> Path | None:
    env = os.environ.get("HUV_CLI")
    for c in ([Path(env)] if env else []) + [ROOT / "target" / "release" / "highuvlith"]:
        if c.is_file() and os.access(c, os.X_OK):
            return c
    return None


def test_python_matches_a_live_cli_run(python_numbers, tmp_path):
    cli = _cli_binary()
    if cli is None:
        pytest.skip("CLI binary not built (cargo build --release -p highuvlith-cli, or set HUV_CLI)")
    proc = subprocess.run([str(cli), "simulate", "--config", str(ROOT / "examples" / "sim.toml"), "--output", str(tmp_path / "a.png")],
                          capture_output=True, text=True, cwd=str(ROOT), timeout=300)
    assert proc.returncode == 0, proc.stderr[-2000:]
    live = _cli_numbers_from_text(proc.stdout + proc.stderr)
    assert "contrast" in live, (proc.stdout + proc.stderr)[-2000:]
    assert _close(python_numbers["contrast"], live["contrast"]), (python_numbers, live)
    if "nils" in live:
        assert _close(python_numbers["nils"], live["nils"])


def test_reference_rosetta_table_quotes_the_same_numbers(python_numbers):
    page = ROOT / "docs" / "reference" / "index.md"
    if not page.exists():
        pytest.skip("docs/reference/index.md absent (G0a stub lands it; P8a fills it)")
    text = page.read_text(encoding="utf-8")
    a, b = text.find("<!-- rosetta:start -->"), text.find("<!-- rosetta:end -->")
    if a < 0 or b < 0:
        pytest.skip("no <!-- rosetta:start --> … <!-- rosetta:end --> block yet (P8a)")
    block = text[a:b]
    nums = [float(x) for x in re.findall(r"\b0\.\d{2,4}\b", block)]
    assert any(_close(python_numbers["contrast"], n) for n in nums), f"contrast {python_numbers['contrast']:.4f} not in the Rosetta block"
    assert any(_close(python_numbers["nils"], n) for n in nums), f"NILS {python_numbers['nils']:.3f} not in the Rosetta block"
