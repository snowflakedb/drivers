"""Unit tests for scripts/validate_behavior_differences.py.

Run with:
    python3 -m unittest scripts/test_validate_behavior_differences.py -v
"""

from __future__ import annotations

import importlib.util
import os
import subprocess
import sys
import unittest
import unittest.mock
from pathlib import Path

import yaml

ROOT = Path(__file__).resolve().parents[1]
MODULE_PATH = ROOT / "scripts" / "validate_behavior_differences.py"
PYTHON_BD = "python/BehaviorDifferences.yaml"
GATED_DIR = "python/behavior_differences"
LEGACY_PATH = f"{GATED_DIR}/auth/private-key-password-renamed.yaml"
ODBC_BD = "odbc_tests/BehaviorDifferences.yaml"
ODBC_DIR = "odbc_tests/behavior_differences"
ODBC_LEGACY_PATH = f"{ODBC_DIR}/put-get/put-expands-leading-tilde.yaml"
JDBC_BD = "jdbc/BehaviorDifferences.yaml"
JDBC_DIR = "jdbc/behavior_differences"
JDBC_LEGACY_PATH = f"{JDBC_DIR}/session/query-context-cache-same-priority.yaml"


def _load():
    spec = importlib.util.spec_from_file_location("validate_behavior_differences", MODULE_PATH)
    mod = importlib.util.module_from_spec(spec)
    assert spec.loader is not None
    sys.modules[spec.name] = mod
    spec.loader.exec_module(mod)
    return mod


def _entry_body(**extra: object) -> str:
    lines = [
        'name: "Example"',
        "status: unknown",
        "type: unknown",
        "reviewed: false",
        "impact: low",
        "is_breaking_change: false",
    ]
    for key, value in extra.items():
        lines.append(f"{key}: {value}")
    return "\n".join(lines) + "\n"


def _monolith(*ids: int) -> str:
    blocks = ["behavior_differences:"]
    for bd_id in ids:
        blocks.append(f"  {bd_id}:")
        for line in _entry_body().splitlines():
            blocks.append(f"    {line}")
    return "\n".join(blocks) + "\n"


class ValidateBehaviorDifferencesTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls) -> None:
        cls.mod = _load()

    def test_new_python_integer_key_is_rejected(self) -> None:
        result = self.mod.check_monolith_text(
            PYTHON_BD,
            _monolith(1, 99),
            {1},
            gated_file_dir=GATED_DIR,
        )
        self.assertTrue(any("BD-99" in err and "new entries must be a file" in err for err in result.errors))
        self.assertEqual(result.removed, set())

    def test_unreadable_baseline_skips_new_key_and_removal_rules(self) -> None:
        result = self.mod.check_monolith_text(
            PYTHON_BD,
            _monolith(99),
            None,
            gated_file_dir=GATED_DIR,
        )
        self.assertEqual(result.removed, set())
        self.assertFalse(any("new entries" in err for err in result.errors))

    def test_python_removal_is_reported_for_legacy_id_pairing(self) -> None:
        result = self.mod.check_monolith_text(
            PYTHON_BD,
            _monolith(1),
            {1, 5},
            gated_file_dir=GATED_DIR,
        )
        self.assertEqual(result.removed, {5})
        self.assertFalse(any("removed" in err for err in result.errors))

    def test_new_odbc_integer_key_is_rejected(self) -> None:
        result = self.mod.check_monolith_text(
            ODBC_BD,
            _monolith(1, 99),
            {1},
            gated_file_dir=ODBC_DIR,
        )
        self.assertTrue(
            any("BD-99" in err and "new entries must be a file" in err for err in result.errors)
        )
        self.assertEqual(result.removed, set())

    def test_odbc_removal_is_reported_for_legacy_id_pairing(self) -> None:
        result = self.mod.check_monolith_text(
            ODBC_BD,
            _monolith(1),
            {1, 82},
            gated_file_dir=ODBC_DIR,
        )
        self.assertEqual(result.removed, {82})
        self.assertFalse(any("removed" in err for err in result.errors))

    def test_odbc_legacy_id_pairs_with_a_removal(self) -> None:
        errors = self.mod.check_migration(ODBC_BD, {1}, {82}, {82: ODBC_LEGACY_PATH})
        self.assertEqual(errors, [])

    def test_odbc_reviewed_file_requires_status_rationale(self) -> None:
        reviewed = "\n".join(
            [
                'name: "Example"',
                "status: allowed",
                "type: bugfix",
                "reviewed: true",
                "impact: low",
                "is_breaking_change: false",
                "",
            ]
        )
        missing = self.mod.check_bd_files(
            ODBC_DIR,
            [("put-get/put-expands-leading-tilde.yaml", reviewed)],
            require_status_rationale=True,
        )
        self.assertTrue(any("status_rationale" in err for err in missing.errors))
        present = self.mod.check_bd_files(
            ODBC_DIR,
            [
                (
                    "put-get/put-expands-leading-tilde.yaml",
                    reviewed + 'status_rationale: "Accepted."\n',
                )
            ],
            require_status_rationale=True,
        )
        self.assertEqual(present.errors, [])

    def test_new_jdbc_integer_key_is_rejected(self) -> None:
        result = self.mod.check_monolith_text(
            JDBC_BD,
            _monolith(1, 99),
            {1},
            gated_file_dir=JDBC_DIR,
        )
        self.assertTrue(
            any("BD-99" in err and "new entries must be a file" in err for err in result.errors)
        )
        self.assertEqual(result.removed, set())

    def test_jdbc_removal_is_reported_for_legacy_id_pairing(self) -> None:
        result = self.mod.check_monolith_text(
            JDBC_BD,
            _monolith(1),
            {1, 59},
            gated_file_dir=JDBC_DIR,
        )
        self.assertEqual(result.removed, {59})
        self.assertFalse(any("removed" in err for err in result.errors))

    def test_jdbc_legacy_id_pairs_with_a_removal(self) -> None:
        errors = self.mod.check_migration(JDBC_BD, {1}, {59}, {59: JDBC_LEGACY_PATH})
        self.assertEqual(errors, [])

    def test_jdbc_reviewed_monolith_entry_requires_status_rationale(self) -> None:
        text = _monolith(1).replace("reviewed: false", "reviewed: true")
        result = self.mod.check_monolith_text(JDBC_BD, text, {1}, gated_file_dir=JDBC_DIR)
        self.assertTrue(any("status_rationale" in err for err in result.errors))

    def test_nodejs_removal_is_an_error(self) -> None:
        result = self.mod.check_monolith_text(
            "nodejs/BehaviorDifferences.yaml",
            _monolith(1),
            {1, 5},
        )
        self.assertTrue(
            any("BD-5" in err and "removals are not allowed" in err for err in result.errors)
        )

    def test_legacy_id_pairs_with_a_removal_in_this_diff(self) -> None:
        errors = self.mod.check_migration(PYTHON_BD, {1}, {5}, {5: LEGACY_PATH})
        self.assertEqual(errors, [])

    def test_legacy_id_stays_valid_after_the_deletion_is_committed(self) -> None:
        errors = self.mod.check_migration(PYTHON_BD, {1}, set(), {5: LEGACY_PATH})
        self.assertEqual(errors, [])

    def test_legacy_id_collides_with_a_key_still_in_the_monolith(self) -> None:
        errors = self.mod.check_migration(PYTHON_BD, {1, 5}, set(), {5: LEGACY_PATH})
        self.assertEqual(
            errors,
            [f"{LEGACY_PATH}: legacy_id 5 is still a key in {PYTHON_BD}"],
        )

    def test_removal_without_legacy_id_is_an_error(self) -> None:
        errors = self.mod.check_migration(PYTHON_BD, {1}, {5}, {})
        self.assertEqual(len(errors), 1)
        self.assertIn("BD-5", errors[0])
        self.assertIn("legacy_id: 5", errors[0])

    def test_per_file_entry_accepts_a_slug_and_records_legacy_id(self) -> None:
        body = _entry_body(legacy_id=5)
        result = self.mod.check_bd_files(
            GATED_DIR,
            [("auth/private-key-password-renamed.yaml", body)],
        )
        self.assertEqual(result.errors, [])
        self.assertEqual(result.legacy_ids, {5: LEGACY_PATH})

    def test_slug_first_segment_must_be_at_least_two_characters(self) -> None:
        short = self.mod.check_bd_files(GATED_DIR, [("auth/a-b.yaml", _entry_body())])
        self.assertTrue(any("a-b" in err for err in short.errors))
        ok = self.mod.check_bd_files(GATED_DIR, [("auth/ab-c.yaml", _entry_body())])
        self.assertEqual(ok.errors, [])

    def test_duplicate_slug_bad_area_wrapper_and_bool_legacy_id(self) -> None:
        duplicate = self.mod.check_bd_files(
            GATED_DIR,
            [
                ("auth/example-name.yaml", _entry_body()),
                ("query/example-name.yaml", _entry_body()),
            ],
        )
        self.assertTrue(any("duplicate slug" in err for err in duplicate.errors))

        area = self.mod.check_bd_files(GATED_DIR, [("nope/example-name.yaml", _entry_body())])
        self.assertTrue(any("not allowed" in err for err in area.errors))

        wrapped = self.mod.check_bd_files(
            GATED_DIR,
            [("auth/example-name.yaml", "behavior_differences:\n  1:\n    name: Example\n")],
        )
        self.assertTrue(any("do not wrap" in err for err in wrapped.errors))

        boolean = self.mod.check_bd_files(
            GATED_DIR,
            [("auth/example-name.yaml", _entry_body(legacy_id="true"))],
        )
        self.assertTrue(any("positive integer" in err for err in boolean.errors))
        self.assertEqual(boolean.legacy_ids, {})

    def test_baseline_rev_uses_bd_base_sha_when_it_exists(self) -> None:
        sha = subprocess.check_output(["git", "rev-parse", "HEAD"], text=True).strip()
        with unittest.mock.patch.dict(os.environ, {"BD_BASE_SHA": sha}):
            self.assertEqual(self.mod._baseline_rev(), sha)
        with unittest.mock.patch.dict(os.environ, {"BD_BASE_SHA": "not-a-rev"}):
            self.assertEqual(self.mod._baseline_rev(), "HEAD")
        env = os.environ.copy()
        env.pop("BD_BASE_SHA", None)
        with unittest.mock.patch.dict(os.environ, env, clear=True):
            self.assertEqual(self.mod._baseline_rev(), "HEAD")

    def test_committed_ids_reads_the_given_revision(self) -> None:
        for relpath in (PYTHON_BD, ODBC_BD, JDBC_BD):
            ids = self.mod._committed_ids(relpath, "HEAD")
            self.assertIsNotNone(ids)
            assert ids is not None
            raw = subprocess.check_output(["git", "show", f"HEAD:{relpath}"])
            data = yaml.safe_load(raw)
            entries = data.get("behavior_differences", {}) if isinstance(data, dict) else {}
            if not isinstance(entries, dict):
                entries = {}
            expected = {int(key) for key, entry in entries.items() if isinstance(entry, dict)}
            self.assertEqual(ids, expected)
            self.assertTrue(all(bd_id > 0 for bd_id in ids))
        self.assertIsNone(self.mod._committed_ids(PYTHON_BD, "not-a-rev"))


if __name__ == "__main__":
    unittest.main()
