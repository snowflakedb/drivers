"""Unit tests for scripts/pytest_junit_retry_plugin/annotate_pytest_junit_retries.py.

Run with:
    python3 -m unittest scripts/test_annotate_pytest_junit_retries.py -v
"""

from __future__ import annotations

import importlib.util
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
MODULE_PATH = ROOT / "scripts" / "pytest_junit_retry_plugin" / "annotate_pytest_junit_retries.py"


def _load():
    spec = importlib.util.spec_from_file_location("annotate_pytest_junit_retries", MODULE_PATH)
    mod = importlib.util.module_from_spec(spec)
    assert spec.loader is not None
    spec.loader.exec_module(mod)
    return mod


class AnnotatePytestJunitRetriesTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls) -> None:
        cls.mod = _load()

    def test_rescued_retry_emits_flaky_failure_per_failed_attempt(self) -> None:
        nodeid = "tests/unit/test_foo.py::test_bar"
        xml = """<?xml version="1.0" encoding="utf-8"?>
<testsuite name="pytest">
  <testcase classname="tests.unit.test_foo" name="test_bar" time="0.09"/>
</testsuite>
"""
        history = {
            nodeid: [
                self.mod.Attempt("rerun", "boom-1"),
                self.mod.Attempt("rerun", "boom-2"),
                self.mod.Attempt("passed", ""),
            ]
        }
        out = self.mod.annotate(xml, history)
        self.assertEqual(out.count("<flakyFailure"), 2)
        self.assertNotIn("rerunFailure", out)
        self.assertIn("boom-1", out)
        self.assertNotIn("<failure", out)

    def test_exhausted_retry_keeps_failure_and_rerun_failure(self) -> None:
        xml = """
<testsuite>
  <testcase classname="tests.unit.test_foo" name="test_bar" time="0.10">
    <failure message="still red"/>
  </testcase>
</testsuite>
"""
        history = {
            "tests/unit/test_foo.py::test_bar": [
                self.mod.Attempt("rerun", "first"),
                self.mod.Attempt("failed", "last"),
            ]
        }
        out = self.mod.annotate(xml, history)
        self.assertIn("rerunFailure", out)
        self.assertNotIn("flakyFailure", out)
        self.assertIn("<failure", out)
        self.assertIn("first", out)

    def test_first_pass_is_unchanged(self) -> None:
        xml = '<testsuite><testcase classname="tests.unit.test_foo" name="ok" time="0.01"/></testsuite>'
        history = {"tests/unit/test_foo.py::ok": [self.mod.Attempt("passed", "")]}
        out = self.mod.annotate(xml, history)
        self.assertNotIn("flakyFailure", out)
        self.assertNotIn("rerunFailure", out)

    def test_class_nodeid_maps_to_junit_classname(self) -> None:
        keys = self.mod.keys_for_nodeid("tests/unit/test_foo.py::TestCls::test_bar")
        self.assertIn("tests.unit.test_foo.TestCls::test_bar", keys)


if __name__ == "__main__":
    unittest.main()
