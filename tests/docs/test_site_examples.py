"""Execute the Python examples embedded in the documentation.

Every fenced ``python`` block in ``docs/**/*.md`` and ``README.md`` must be classified by
an HTML comment on the line directly above its opening fence (same indentation):

* ``<!-- verify-example -->``: the block is run in a fresh interpreter (cwd = a temporary
  directory, ``MPLBACKEND=Agg``) and must exit cleanly within ``HUV_EXAMPLE_TIMEOUT``
  seconds (default 60);
* ``<!-- example-skip: <reason> -->``: the block is a fragment, needs optional packages or
  hardware, or is otherwise not meant to run on its own.

A verified block's printed output may be recorded right after it, in a collapsible
``??? success "Output"`` admonition (docs pages) or a bare ```text fence (README). The test
checks the recorded text against a fresh run, numbers within a small tolerance;
``--update-outputs`` rewrites the recorded outputs from a real run and adds the admonition
to every docs example that prints but has none.

Standalone use (from the repository root, with the extension built in release mode):

    python tests/docs/test_site_examples.py            # run all, print outputs
    python tests/docs/test_site_examples.py docs/nodes # only files under a path
    python tests/docs/test_site_examples.py --update-outputs
"""

from __future__ import annotations

import os
import re
import subprocess
import sys
import tempfile
import textwrap
from concurrent.futures import ThreadPoolExecutor
from dataclasses import dataclass
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
TIMEOUT_S = float(os.environ.get("HUV_EXAMPLE_TIMEOUT", "60"))

FENCE_RE = re.compile(r"^(?P<indent>[ \t]*)```python\s*$")
VERIFY_RE = re.compile(r"^[ \t]*<!--\s*verify-example\s*-->\s*$")
SKIP_RE = re.compile(r"^[ \t]*<!--\s*example-skip:\s*\S.*-->\s*$")
OUTPUT_RE = re.compile(r'^(?P<indent>[ \t]*)\?\?\?\+? success "Output"\s*$')
LEADIN_RE = re.compile(r"^[ \t]*Output\b[^`]*:\s*$")  # "Output:" / "Output (note):" line before a ```text fence


@dataclass
class Block:
    path: Path
    line: int  # 1-based line of the opening fence
    end: int  # 0-based index of the closing fence line
    indent: str
    kind: str  # "verify" | "skip" | "unclassified"
    code: str

    @property
    def id(self) -> str:
        return f"{self.path.relative_to(ROOT).as_posix()}:{self.line}"


def doc_files(paths: list[Path] | None = None) -> list[Path]:
    files = sorted((ROOT / "docs").rglob("*.md")) + [ROOT / "README.md"]
    if paths:
        roots = [p.resolve() for p in paths]
        files = [f for f in files if any(f == r or r in f.parents for r in roots)]
    return files


def extract(path: Path) -> list[Block]:
    lines = path.read_text(encoding="utf-8").splitlines()
    blocks = []
    i = 0
    while i < len(lines):
        m = FENCE_RE.match(lines[i])
        if not m:
            i += 1
            continue
        indent = m.group("indent")
        j = i + 1
        while j < len(lines) and lines[j].rstrip() != indent + "```":
            j += 1
        prev = lines[i - 1] if i > 0 else ""
        kind = "verify" if VERIFY_RE.match(prev) else "skip" if SKIP_RE.match(prev) else "unclassified"
        body = [ln[len(indent):] if ln.startswith(indent) else ln.lstrip() for ln in lines[i + 1 : j]]
        blocks.append(Block(path, i + 1, j, indent, kind, textwrap.dedent("\n".join(body)) + "\n"))
        i = j + 1
    return blocks


def all_blocks(paths: list[Path] | None = None) -> list[Block]:
    return [b for f in doc_files(paths) for b in extract(f)]


def run_block(block: Block) -> subprocess.CompletedProcess:
    env = dict(os.environ, MPLBACKEND="Agg", PYTHONHASHSEED="0")
    pkg = str(ROOT / "python")
    env["PYTHONPATH"] = pkg + os.pathsep + env["PYTHONPATH"] if env.get("PYTHONPATH") else pkg
    with tempfile.TemporaryDirectory() as tmp:
        try:
            return subprocess.run(
                [sys.executable, "-c", block.code],
                cwd=tmp, env=env, capture_output=True, text=True, timeout=TIMEOUT_S,
            )
        except subprocess.TimeoutExpired as exc:
            return subprocess.CompletedProcess(exc.cmd, -1, exc.stdout or "", f"timed out after {TIMEOUT_S:.0f} s")


NUM_RE = re.compile(r"[-+]?(?:\d+\.?\d*|\.\d+)(?:[eE][-+]?\d+)?")
ABS_TOL, REL_TOL = 0.02, 0.03  # printed numbers may drift this much before the doc must be regenerated


@dataclass
class Output:
    start: int  # 0-based index of the opening ```text fence
    stop: int  # 0-based index one past the closing fence
    indent: str  # indentation of the fence lines
    text: str


def find_output(lines: list[str], block: Block) -> Output | None:
    """The block's recorded output: a ``??? success "Output"`` admonition holding a
    ```text fence, or a ```text fence right after the example, optionally introduced by
    an ``Output:`` line (README / python-api.md style)."""
    k = block.end + 1
    while k < len(lines) and not lines[k].strip():
        k += 1
    if k >= len(lines):
        return None
    m = OUTPUT_RE.match(lines[k])
    if m and m.group("indent") == block.indent:
        k += 1
        while k < len(lines) and not lines[k].strip():
            k += 1
        indent = block.indent + "    "
    else:
        indent = block.indent
        if LEADIN_RE.match(lines[k]) and lines[k].startswith(indent + "Output"):
            k += 1
            while k < len(lines) and not lines[k].strip():
                k += 1
    if k >= len(lines) or lines[k].rstrip() != indent + "```text":
        return None
    j = k + 1
    while j < len(lines) and lines[j].rstrip() != indent + "```":
        j += 1
    body = [ln[len(indent):] for ln in lines[k + 1 : j]]
    return Output(k, j + 1, indent, "\n".join(body).rstrip())


def outputs_match(expected: str, actual: str) -> bool:
    """Same text, with every printed number within ABS_TOL or REL_TOL of the recorded one."""
    exp_lines, act_lines = expected.strip().splitlines(), actual.strip().splitlines()
    if len(exp_lines) != len(act_lines):
        return False
    for e, a in zip(exp_lines, act_lines):
        if NUM_RE.sub("#", " ".join(e.split())) != NUM_RE.sub("#", " ".join(a.split())):
            return False
        for x, y in zip(map(float, NUM_RE.findall(e)), map(float, NUM_RE.findall(a))):
            if abs(x - y) > max(ABS_TOL, REL_TOL * abs(x)):
                return False
    return True


def update_outputs(results: dict[str, tuple[Block, subprocess.CompletedProcess]]) -> int:
    """Rewrite each recorded output from a real run; under docs/, add a collapsible
    ``??? success "Output"`` block to every example that prints but has none."""
    changed = 0
    by_file: dict[Path, list[tuple[Block, str]]] = {}
    for block, proc in results.values():
        if proc.returncode == 0 and proc.stdout.strip():
            by_file.setdefault(block.path, []).append((block, proc.stdout))
    for path, items in by_file.items():
        lines = path.read_text(encoding="utf-8").splitlines()
        for block, stdout in sorted(items, key=lambda t: -t[0].line):
            out = find_output(lines, block)
            if out is not None:
                ind = out.indent
                text = [(ind + ln).rstrip() for ln in stdout.rstrip("\n").splitlines()]
                new = [ind + "```text", *text, ind + "```"]
                if lines[out.start : out.stop] != new:
                    lines[out.start : out.stop] = new
                    changed += 1
            elif path != ROOT / "README.md":
                ind = block.indent + "    "
                text = [(ind + ln).rstrip() for ln in stdout.rstrip("\n").splitlines()]
                new = ["", block.indent + '??? success "Output"', "", ind + "```text", *text, ind + "```"]
                lines[block.end + 1 : block.end + 1] = new
                changed += 1
        path.write_text("\n".join(lines) + "\n", encoding="utf-8")
    return changed


# ----------------------------------------------------------------------------- pytest

try:
    import pytest
except ImportError:  # standalone use without pytest
    pytest = None

if pytest is not None:
    _VERIFY = [b for b in all_blocks() if b.kind == "verify"]

    def test_every_python_block_is_classified():
        bad = [b.id for b in all_blocks() if b.kind == "unclassified"]
        assert not bad, (
            "python blocks without <!-- verify-example --> or <!-- example-skip: reason --> "
            "on the line above the fence: " + ", ".join(bad)
        )

    def test_examples_found():
        assert len(_VERIFY) >= 50

    @pytest.mark.parametrize("block", _VERIFY, ids=[b.id for b in _VERIFY])
    def test_example_runs(block: Block):
        proc = run_block(block)
        assert proc.returncode == 0, f"{block.id} failed:\n{block.code}\n--- stderr ---\n{proc.stderr[-4000:]}"
        out = find_output(block.path.read_text(encoding="utf-8").splitlines(), block)
        if out is not None:
            assert outputs_match(out.text, proc.stdout), (
                f"{block.id}: printed output no longer matches the page "
                f"(regenerate with tests/docs/test_site_examples.py --update-outputs)\n"
                f"--- page ---\n{out.text}\n--- run ---\n{proc.stdout}"
            )


def main(argv: list[str]) -> int:
    update = "--update-outputs" in argv
    paths = [Path(a) for a in argv if not a.startswith("--")]
    blocks = all_blocks(paths or None)
    for b in blocks:
        if b.kind == "unclassified":
            print(f"UNCLASSIFIED {b.id}")
    verify = [b for b in blocks if b.kind == "verify"]
    with ThreadPoolExecutor(max_workers=os.cpu_count() or 4) as pool:
        procs = list(pool.map(run_block, verify))
    results = {b.id: (b, p) for b, p in zip(verify, procs)}
    failed = 0
    for b, p in results.values():
        status = "ok" if p.returncode == 0 else "FAIL"
        out = find_output(b.path.read_text(encoding="utf-8").splitlines(), b) if p.returncode == 0 else None
        if out is not None and not outputs_match(out.text, p.stdout):
            status = "STALE-OUTPUT"
        failed += status != "ok"
        print(f"=== {status} {b.id}")
        if p.stdout:
            print(textwrap.indent(p.stdout.rstrip(), "    "))
        if p.returncode != 0:
            print(textwrap.indent(p.stderr.rstrip()[-3000:], "  ! "))
    print(f"\n{len(verify) - failed}/{len(verify)} examples passed, {sum(b.kind == 'skip' for b in blocks)} skipped")
    if update:
        print(f"updated {update_outputs(results)} output blocks")
    return 1 if failed else 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
