"""Unit tests for the behavior-difference checks in scripts/check_changelog.py.

Run with:
    python3 -m unittest scripts/test_check_changelog.py -v
"""

from __future__ import annotations

import importlib.util
import sys
import tempfile
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
MODULE_PATH = ROOT / "scripts" / "check_changelog.py"
UPCOMING = "# Changelog\n\n## Upcoming Release\n\nIn-progress notes are files under `changelog/unreleased/`.\n"


def _load():
    spec = importlib.util.spec_from_file_location("check_changelog", MODULE_PATH)
    mod = importlib.util.module_from_spec(spec)
    assert spec.loader is not None
    sys.modules[spec.name] = mod
    spec.loader.exec_module(mod)
    return mod


def _fragment(*, bd_line: str | None = None, category: str = "breaking") -> str:
    header = [f"category: {category}"]
    if bd_line is not None:
        header.append(bd_line)
    return "---\n" + "\n".join(header) + "\n---\n\n- Changed X. (snowflakedb/drivers#1)\n"


def _entry(status: str) -> str:
    return (
        'name: "Example"\n'
        f"status: {status}\n"
        "type: unknown\n"
        "reviewed: true\n"
        "impact: low\n"
        "is_breaking_change: false\n"
    )


class ParseFragmentTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls) -> None:
        cls.mod = _load()

    def test_lists_behavior_differences(self) -> None:
        parsed = self.mod.parse_fragment(_fragment(bd_line="behavior_differences: [BD#a-b, BD#c]"))

        self.assertEqual(parsed.errors, [])
        self.assertEqual(parsed.category, "breaking")
        self.assertEqual(parsed.behavior_differences, ("BD#a-b", "BD#c"))

    def test_category_only_fragment_has_no_behavior_differences(self) -> None:
        parsed = self.mod.parse_fragment(_fragment())

        self.assertEqual((parsed.errors, parsed.behavior_differences), ([], ()))

    def test_rejects_malformed_lists(self) -> None:
        cases = {
            "behavior_differences: BD#a": ["behavior_differences must be a list such as [BD#some-slug]"],
            "behavior_differences: []": ["behavior_differences is empty"],
            "behavior_differences: [BD#Upper]": [
                "behavior_differences item 'BD#Upper' is not BD#<kebab-case-slug>"
            ],
            "behavior_differences: [5]": ["behavior_differences item '5' is not BD#<kebab-case-slug>"],
            "behavior_differences: [BD#a, BD#a]": ["behavior_differences lists BD#a more than once"],
        }
        for line, expected in cases.items():
            with self.subTest(line=line):
                self.assertEqual(self.mod.parse_fragment(_fragment(bd_line=line)).errors, expected)

    def test_rejects_the_field_twice(self) -> None:
        text = _fragment(bd_line="behavior_differences: [BD#a]\nbehavior_differences: [BD#b]")

        self.assertEqual(
            self.mod.parse_fragment(text).errors, ["behavior_differences is set more than once"]
        )

    def test_rejects_other_fields(self) -> None:
        self.assertEqual(
            self.mod.parse_fragment(_fragment(bd_line="owner: me")).errors,
            ["only category and behavior_differences fields are allowed in fragment metadata"],
        )


class ValidateBdCitationsTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls) -> None:
        cls.mod = _load()

    def setUp(self) -> None:
        tmp = tempfile.TemporaryDirectory()
        self.addCleanup(tmp.cleanup)
        self.root = Path(tmp.name)
        for component in ("dotnet", "python"):
            (self.root / component / "changelog" / "unreleased").mkdir(parents=True)
            (self.root / component / "CHANGELOG.md").write_text(UPCOMING, encoding="utf-8")

    def _write(self, rel: str, text: str) -> None:
        path = self.root / rel
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text(text, encoding="utf-8")

    def _errors(self) -> list[str]:
        return self.mod.validate(self.root)

    def test_allowed_entry_without_fragment_is_an_error(self) -> None:
        self._write("dotnet/behavior_differences/query/rows-affected.yaml", _entry("allowed"))

        self.assertEqual(
            self._errors(),
            [
                "dotnet/behavior_differences/query/rows-affected.yaml: status allowed needs a "
                "changelog fragment under dotnet/changelog/ whose behavior_differences lists "
                "BD#rows-affected"
            ],
        )

    def test_allowed_entry_cited_by_an_unreleased_fragment_passes(self) -> None:
        self._write("dotnet/behavior_differences/query/rows-affected.yaml", _entry("allowed"))
        self._write(
            "dotnet/changelog/unreleased/rows-affected.md",
            _fragment(bd_line="behavior_differences: [BD#rows-affected]"),
        )

        self.assertEqual(self._errors(), [])

    def test_allowed_entry_cited_by_a_released_fragment_passes(self) -> None:
        self._write("dotnet/behavior_differences/query/rows-affected.yaml", _entry('"allowed"'))
        self._write(
            "dotnet/changelog/v1.0.0/rows-affected.md",
            _fragment(bd_line="behavior_differences: [BD#rows-affected]"),
        )
        self._write(
            "dotnet/CHANGELOG.md",
            UPCOMING + "\n## v1.0.0\n\nBreaking changes:\n\n- Changed X. (snowflakedb/drivers#1)\n",
        )

        self.assertEqual(self._errors(), [])

    def test_other_statuses_need_no_fragment(self) -> None:
        for status in ("unknown", "todo", "fixed"):
            self._write(f"dotnet/behavior_differences/query/{status}-entry.yaml", _entry(status))

        self.assertEqual(self._errors(), [])

    def test_same_slug_in_two_areas_is_an_error(self) -> None:
        self._write("dotnet/behavior_differences/connection/put-get.yaml", _entry("allowed"))
        self._write("dotnet/behavior_differences/query/put-get.yaml", _entry("todo"))
        self._write(
            "dotnet/changelog/unreleased/put-get.md",
            _fragment(bd_line="behavior_differences: [BD#put-get]"),
        )

        self.assertEqual(
            self._errors(),
            [
                "dotnet/behavior_differences/query/put-get.yaml: BD#put-get is also defined by "
                "dotnet/behavior_differences/connection/put-get.yaml; slugs are unique across areas"
            ],
        )

    def test_same_slug_in_another_component_registry_is_independent(self) -> None:
        self._write("dotnet/behavior_differences/query/put-get.yaml", _entry("allowed"))
        self._write("python/behavior_differences/query/put-get.yaml", _entry("allowed"))

        self.assertEqual(
            self._errors(),
            [
                "dotnet/behavior_differences/query/put-get.yaml: status allowed needs a "
                "changelog fragment under dotnet/changelog/ whose behavior_differences lists "
                "BD#put-get"
            ],
        )

    def test_citing_a_missing_entry_is_an_error(self) -> None:
        self._write(
            "dotnet/changelog/unreleased/rows-affected.md",
            _fragment(bd_line="behavior_differences: [BD#no-such-entry]"),
        )

        self.assertEqual(
            self._errors(),
            [
                "dotnet/changelog/unreleased/rows-affected.md: BD#no-such-entry has no entry "
                "under dotnet/behavior_differences/"
            ],
        )

    def test_citation_outside_a_registry_component_is_an_error(self) -> None:
        self._write(
            "python/changelog/unreleased/rows-affected.md",
            _fragment(bd_line="behavior_differences: [BD#rows-affected]"),
        )

        self.assertEqual(
            self._errors(),
            [
                "python/changelog/unreleased/rows-affected.md: behavior_differences is only "
                "supported in dotnet fragments"
            ],
        )

    def test_rendered_changelog_omits_the_metadata(self) -> None:
        self._write("dotnet/behavior_differences/query/rows-affected.yaml", _entry("allowed"))
        self._write(
            "dotnet/changelog/v1.0.0/rows-affected.md",
            _fragment(bd_line="behavior_differences: [BD#rows-affected]"),
        )

        self.mod.render_changelogs(self.root)

        self.assertEqual(
            (self.root / "dotnet" / "CHANGELOG.md").read_text(encoding="utf-8"),
            UPCOMING + "\n## v1.0.0\n\nBreaking changes:\n\n- Changed X. (snowflakedb/drivers#1)\n",
        )


if __name__ == "__main__":
    unittest.main()
