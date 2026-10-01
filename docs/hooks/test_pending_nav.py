"""Tests for pending_nav.py. Run: python -m pytest docs/hooks -q"""

from __future__ import annotations

import logging

import pytest

import pending_nav
from pending_nav import prune_nav, strict_mode

NAV = [
    {"Home": "index.md"},
    {"History": ["history/index.md", {"Early days": "history/early.md"}]},
    {"Process Nodes": ["nodes/index.md"]},
    {
        "Simulator": [
            {"Pipeline": "pipeline.md"},
            {"Sources": ["sources/index.md", {"DPP": "sources/dpp.md"}]},
        ]
    },
    {"Contributing": "https://github.com/example/repo/blob/main/CONTRIBUTING.md"},
    "playground/index.md",
]


def test_prune_drops_missing_pages_and_empty_sections() -> None:
    present = {"index.md", "history/index.md", "pipeline.md", "sources/index.md"}
    pruned, dropped = prune_nav(NAV, present.__contains__)
    assert pruned == [
        {"Home": "index.md"},
        {"History": ["history/index.md"]},
        {"Simulator": [{"Pipeline": "pipeline.md"}, {"Sources": ["sources/index.md"]}]},
        {"Contributing": "https://github.com/example/repo/blob/main/CONTRIBUTING.md"},
    ]
    assert dropped == [
        "history/early.md",
        "nodes/index.md",
        "sources/dpp.md",
        "playground/index.md",
    ]


def test_prune_keeps_everything_when_all_pages_exist() -> None:
    pruned, dropped = prune_nav(NAV, lambda _path: True)
    assert pruned == NAV
    assert dropped == []


def test_prune_leaves_automatic_nav_alone() -> None:
    assert prune_nav(None, lambda _path: False) == (None, [])


@pytest.mark.parametrize(
    ("environ", "strict"),
    [
        ({}, False),
        ({"CI": "true"}, True),
        ({"HUV_DOCS_STRICT_NAV": "1"}, True),
        ({"HUV_DOCS_STRICT_NAV": "yes"}, True),
        ({"HUV_DOCS_STRICT_NAV": "0", "CI": "true"}, False),
        ({"HUV_DOCS_STRICT_NAV": "", "CI": "true"}, True),
        ({"CI": "false"}, False),
    ],
)
def test_strict_mode(environ, strict) -> None:
    assert strict_mode(environ) is strict


class _Files:
    def __init__(self, present: set[str]) -> None:
        self.present = present

    def get_file_from_path(self, path: str):
        return object() if path in self.present else None


def test_on_files_prunes_config_and_logs(monkeypatch, caplog) -> None:
    monkeypatch.delenv("HUV_DOCS_STRICT_NAV", raising=False)
    monkeypatch.delenv("CI", raising=False)
    config = {"nav": [{"Home": "index.md"}, {"Later": "later.md"}]}
    files = _Files({"index.md"})
    with caplog.at_level(logging.INFO, logger="mkdocs.hooks.pending_nav"):
        assert pending_nav.on_files(files, config) is files
    assert config["nav"] == [{"Home": "index.md"}]
    assert "later.md" in caplog.text


def test_on_files_is_a_no_op_in_strict_mode(monkeypatch) -> None:
    monkeypatch.setenv("HUV_DOCS_STRICT_NAV", "1")
    nav = [{"Home": "index.md"}, {"Later": "later.md"}]
    config = {"nav": list(nav)}
    pending_nav.on_files(_Files({"index.md"}), config)
    assert config["nav"] == nav
