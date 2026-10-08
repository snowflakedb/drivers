"""Unit tests for scripts/nodejs/annotate_vitest_junit_retries.py.

Run with:
    python3 -m unittest scripts/test_annotate_vitest_junit_retries.py -v
"""

from __future__ import annotations

import importlib.util
import tempfile
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
MODULE_PATH = ROOT / "scripts" / "nodejs" / "annotate_vitest_junit_retries.py"


def _load():
    spec = importlib.util.spec_from_file_location("annotate_vitest_junit_retries", MODULE_PATH)
    mod = importlib.util.module_from_spec(spec)
    assert spec.loader is not None
    spec.loader.exec_module(mod)
    return mod


class AnnotateVitestJunitRetriesTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls) -> None:
        cls.mod = _load()

    def test_rescued_siblings_become_one_case_with_flaky_failure(self) -> None:
        xml = """
        <testsuites>
          <testsuite name="foo.test.ts">
            <testcase classname="foo.test.ts" name="bar" time="0.1">
              <failure message="boom-1" type="AssertionError">stack-1</failure>
            </testcase>
            <testcase classname="foo.test.ts" name="bar" time="0.1"/>
          </testsuite>
        </testsuites>
        """
        out = self.mod.annotate(xml)
        self.assertEqual(out.count("<testcase"), 1)
        self.assertEqual(out.count("<flakyFailure"), 1)
        self.assertIn("boom-1", out)
        self.assertNotIn("<failure", out.replace("<flakyFailure", ""))

    def test_duplicate_passing_siblings_collapse_without_retry_children(self) -> None:
        xml = """
        <testsuite>
          <testcase classname="t.ts" name="TIME" time="0.01"/>
          <testcase classname="t.ts" name="TIME" time="0.02"/>
        </testsuite>
        """
        out = self.mod.annotate(xml)
        self.assertEqual(out.count("<testcase"), 1)
        self.assertNotIn("flakyFailure", out)
        self.assertNotIn("rerunFailure", out)

    def test_exhausted_retries_keep_failure_and_rerun_failure(self) -> None:
        xml = """
        <testsuite>
          <testcase classname="t.ts" name="x" time="0.1">
            <failure message="first"/>
          </testcase>
          <testcase classname="t.ts" name="x" time="0.1">
            <failure message="last"/>
          </testcase>
        </testsuite>
        """
        out = self.mod.annotate(xml)
        self.assertEqual(out.count("<testcase"), 1)
        self.assertIn("rerunFailure", out)
        self.assertIn("first", out)
        self.assertIn("<failure", out)
        self.assertIn("last", out)
        self.assertNotIn("flakyFailure", out)

    def test_cli_rewrites_file(self) -> None:
        xml = '<testsuite><testcase classname="a" name="b" time="0.1"><failure message="x"/></testcase><testcase classname="a" name="b" time="0.1"/></testsuite>'
        with tempfile.TemporaryDirectory() as tmp:
            jpath = Path(tmp) / "nodejs-unit.xml"
            jpath.write_text(xml, encoding="utf-8")
            rc = self.mod.main([str(jpath)])
            self.assertEqual(rc, 0)
            body = jpath.read_text(encoding="utf-8")
            self.assertIn("flakyFailure", body)
            self.assertEqual(body.count("<testcase"), 1)


if __name__ == "__main__":
    unittest.main()
