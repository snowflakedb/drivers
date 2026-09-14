"""Regression test for the JUnit parser embedded in
``.github/workflows/notify-slack-failure.yml``.

The workflow's "Build and post Slack notification" step parses every JUnit
artifact under ``test-artifacts/`` and lists the failing testcases in the Slack
alert body. The flaky lanes (ODBC "Run flaky ODBC tests" steps, Python
``python_flaky_tests``, Rust "Run flaky tests (non-blocking)") run
continue-on-error, so they never fail a run, but their result artifacts still
upload; the step drops any JUnit path containing ``flaky`` so those failures
stay out of the alert and out of CI-triage.

This test extracts that embedded Python verbatim and runs it against a synthetic
artifact tree, so the filter cannot silently regress if the parser is edited.

Run with:
    python -m unittest scripts/test_notify_slack_failure.py -v
"""

from __future__ import annotations

import json
import os
import subprocess
import sys
import tempfile
import textwrap
import unittest
from pathlib import Path

WORKFLOW = (
    Path(__file__).resolve().parents[1]
    / ".github"
    / "workflows"
    / "notify-slack-failure.yml"
)

_HEREDOC_START = "python3 <<'PYEOF'"
_HEREDOC_END = "PYEOF"


def _extract_embedded_python() -> str:
    """Returns the parser step's Python, dedented, exactly as the workflow runs
    it. The YAML ``run: |`` block strips its common indentation before bash sees
    the heredoc, so ``textwrap.dedent`` reproduces the column-0 source."""
    lines = WORKFLOW.read_text().splitlines()
    start = next(i for i, line in enumerate(lines) if _HEREDOC_START in line)
    end = next(
        i for i, line in enumerate(lines) if i > start and line.strip() == _HEREDOC_END
    )
    return textwrap.dedent("\n".join(lines[start + 1 : end]))


def _run_parser(artifacts_root: Path) -> dict:
    """Runs the extracted parser with ``artifacts_root`` as its working directory
    (the JUnit glob is relative to the cwd) and returns the Slack payload it
    writes to disk."""
    parser = artifacts_root / "_parser.py"
    parser.write_text(_extract_embedded_python())
    payload = artifacts_root / "payload.json"
    env = {
        **os.environ,
        "WORKFLOW_NAME": "Driver CI",
        "RUN_URL": "https://example.snowflake.com/run/1",
        "HEAD_SHA": "abcdef1234567890",
        "HEAD_BRANCH": "main",
        "TRIGGER_EVENT": "push",
        "FAILED_JOBS_JSON": "[]",
        "AUTHOR_MENTION": "tester",
        "SLACK_CHANNEL": "universal-driver-ci-alerts",
        "SLACK_PAYLOAD_FILE": str(payload),
    }
    subprocess.run(
        [sys.executable, parser.name], cwd=artifacts_root, env=env, check=True
    )
    return json.loads(payload.read_text())


def _failed_tests_text(payload: dict) -> str:
    """Returns the text of the "Failed tests:" section, or an empty string when
    the payload has no such section."""
    for block in payload["blocks"]:
        text = block.get("text", {}).get("text", "")
        if block.get("type") == "section" and text.startswith("*Failed tests:*"):
            return text
    return ""


def _write_junit(path: Path, classname: str, name: str, tag: str = "failure") -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(
        f'<testsuite><testcase classname="{classname}" name="{name}">'
        f'<{tag} message="boom"/></testcase></testsuite>'
    )


class FlakyArtifactFilterTests(unittest.TestCase):
    def _tree(self, root: Path) -> None:
        # One real (blocking) failure alongside a flaky lane for ODBC, Python,
        # and Rust — the three wrappers that upload flaky JUnit artifacts.
        _write_junit(
            root / "test-artifacts/odbc-junit/odbc.xml",
            "odbc.ConnectIT",
            "testRealConnect",
        )
        _write_junit(
            root / "test-artifacts/odbc-flaky-tests/odbc-flaky-junit.xml",
            "odbc.FlakyIT",
            "testFlakyOdbc",
        )
        _write_junit(
            root / "test-artifacts/results-flaky-linux/results.xml",
            "tests.flaky",
            "test_flaky_py",
        )
        _write_junit(
            root / "test-artifacts/rust-core-flaky-tests-mac/junit.xml",
            "sf_core::flaky",
            "flaky_rust_case",
            tag="error",
        )

    def test_real_failure_is_reported(self) -> None:
        with tempfile.TemporaryDirectory() as d:
            root = Path(d)
            self._tree(root)
            block = _failed_tests_text(_run_parser(root))
        self.assertIn("odbc.ConnectIT.testRealConnect", block)

    def test_flaky_lanes_are_excluded(self) -> None:
        with tempfile.TemporaryDirectory() as d:
            root = Path(d)
            self._tree(root)
            block = _failed_tests_text(_run_parser(root))
        for flaky in ("testFlakyOdbc", "test_flaky_py", "flaky_rust_case"):
            self.assertNotIn(
                flaky, block, f"flaky test {flaky!r} leaked into the alert body"
            )

    def test_only_flaky_yields_no_failed_tests_block(self) -> None:
        with tempfile.TemporaryDirectory() as d:
            root = Path(d)
            _write_junit(
                root / "test-artifacts/odbc-flaky-tests/odbc-flaky-junit.xml",
                "odbc.FlakyIT",
                "testFlakyOdbc",
            )
            payload = _run_parser(root)
        self.assertEqual(_failed_tests_text(payload), "")


if __name__ == "__main__":
    unittest.main()
