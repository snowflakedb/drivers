"""Unit tests for the behavior-difference and PR-link checks in scripts/check_changelog.py.

Run with:
    python3 -m unittest scripts/test_check_changelog.py -v
"""

from __future__ import annotations

import contextlib
import importlib.util
import io
import os
import subprocess
import sys
import tempfile
import unittest
import unittest.mock
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


def _bugfix(*bullets: str) -> str:
    return "---\ncategory: bugfix\n---\n\n" + "".join(f"{b}\n" for b in bullets)


class CheckBulletLinksTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls) -> None:
        cls.mod = _load()

    def _check(self, text: str, *, added: bool = True, pr_number: int | None = None, strict: bool = False):
        findings = self.mod.check_bullet_links(
            "c/changelog/unreleased/x.md", text, added=added, pr_number=pr_number, strict=strict
        )
        return findings.errors, findings.warnings

    def test_well_formed_bullet_passes_in_every_mode(self) -> None:
        text = _bugfix("- Fixed X. (snowflakedb/drivers#7)")
        for pr_number, strict in ((None, False), (None, True), (7, False)):
            with self.subTest(pr_number=pr_number, strict=strict):
                self.assertEqual(self._check(text, pr_number=pr_number, strict=strict), ([], []))

    def test_missing_link_warns_locally(self) -> None:
        self.assertEqual(
            self._check(_bugfix("- Fixed X.")),
            ([], ["c/changelog/unreleased/x.md:5: bullet has no (snowflakedb/drivers#N) link at the end; CI rejects it"]),
        )

    def test_tbd_warns_locally_and_fails_when_strict(self) -> None:
        text = _bugfix("- Fixed X. (snowflakedb/drivers#TBD)")
        message = "c/changelog/unreleased/x.md:5: bullet cites #TBD instead of a pull request number"

        self.assertEqual(self._check(text), ([], [f"{message}; CI rejects it"]))
        self.assertEqual(self._check(text, strict=True), ([message], []))
        self.assertEqual(self._check(text, pr_number=3), ([message], []))

    def test_public_repository_link_is_always_an_error(self) -> None:
        self.assertEqual(
            self._check(_bugfix("- Fixed X. (snowflakedb/drivers#7)")),
            (
                [
                    "c/changelog/unreleased/x.md:5: cites snowflakedb/drivers#7; cite "
                    "snowflakedb/drivers#N, the pull request number in this repository"
                ],
                [],
            ),
        )

    def test_text_after_the_link_is_an_error(self) -> None:
        self.assertEqual(
            self._check(_bugfix("- Fixed X (snowflakedb/drivers#7).")),
            (["c/changelog/unreleased/x.md:5: bullet must end with its (snowflakedb/drivers#N) link"], []),
        )

    def test_added_fragment_must_cite_this_pull_request(self) -> None:
        text = _bugfix("- Fixed X. (snowflakedb/drivers#7)")

        self.assertEqual(
            self._check(text, pr_number=8),
            (["c/changelog/unreleased/x.md:5: cites snowflakedb/drivers#7, but this pull request is #8"], []),
        )
        self.assertEqual(self._check(text, added=False, pr_number=8), ([], []))

    def test_link_on_a_wrapped_line_counts_but_not_on_a_sub_bullet(self) -> None:
        wrapped = _bugfix("- Fixed X when the session", "  expires. (snowflakedb/drivers#7)")
        sub_bullet = _bugfix("- Fixed X:", "  - in Y. (snowflakedb/drivers#7)")

        self.assertEqual(self._check(wrapped, strict=True), ([], []))
        self.assertEqual(
            self._check(sub_bullet, strict=True),
            (["c/changelog/unreleased/x.md:5: bullet has no (snowflakedb/drivers#N) link at the end"], []),
        )

    def test_bullet_without_a_past_tense_verb_only_warns(self) -> None:
        self.assertEqual(
            self._check(_bugfix("- `Execute` now retries. (snowflakedb/drivers#7)"), strict=True),
            (
                [],
                [
                    "c/changelog/unreleased/x.md:5: bullet starts with '`Execute`'; changelog "
                    "bullets start with a past-tense verb such as Added or Fixed"
                ],
            ),
        )
        for verb in ("Bumped", "Kept", "Built", "Rewrote"):
            with self.subTest(verb=verb):
                self.assertEqual(
                    self._check(_bugfix(f"- {verb} X. (snowflakedb/drivers#7)")), ([], [])
                )


class ValidatePrLinksTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls) -> None:
        cls.mod = _load()

    def setUp(self) -> None:
        tmp = tempfile.TemporaryDirectory()
        self.addCleanup(tmp.cleanup)
        self.root = Path(tmp.name)
        env = {key: value for key, value in os.environ.items() if not key.startswith("GIT_")}
        env["GIT_CONFIG_GLOBAL"] = os.devnull
        env_patch = unittest.mock.patch.dict(os.environ, env, clear=True)
        env_patch.start()
        self.addCleanup(env_patch.stop)
        self._git("init", "-q", "-b", "main")
        self._write("python/changelog/unreleased/old.md", _bugfix("- Fixed old. (snowflakedb/drivers#1)"))
        self._write("python/changelog/unreleased/edited.md", _bugfix("- Fixed edited. (snowflakedb/drivers#2)"))
        self._git("add", "-A")
        self._git("-c", "user.name=t", "-c", "user.email=t@example.com", "commit", "-q", "-m", "base")

    def _git(self, *args: str) -> None:
        subprocess.run(["git", *args], cwd=self.root, check=True, capture_output=True)

    def _write(self, rel: str, text: str) -> None:
        path = self.root / rel
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text(text, encoding="utf-8")

    def test_checks_only_added_and_modified_fragments(self) -> None:
        self._write("python/changelog/unreleased/new.md", _bugfix("- Fixed new. (snowflakedb/drivers#5)"))
        self._write(
            "python/changelog/unreleased/edited.md",
            _bugfix("- Fixed edited wording. (snowflakedb/drivers#2)", "- Fixed more."),
        )

        findings = self.mod.validate_pr_links(self.root, "main", pr_number=6, strict=False)

        self.assertEqual(
            sorted(findings.errors),
            [
                "python/changelog/unreleased/edited.md:6: bullet has no (snowflakedb/drivers#N) link at the end",
                "python/changelog/unreleased/new.md:5: cites snowflakedb/drivers#5, but this pull request is #6",
            ],
        )
        self.assertEqual(findings.warnings, [])

    def test_rejects_a_non_numeric_pr_number(self) -> None:
        stderr = io.StringIO()
        with contextlib.redirect_stderr(stderr):
            code = self.mod.main(["--root", str(self.root), "--base", "main", "--pr-number", "abc"])

        self.assertEqual((code, stderr.getvalue()), (2, "--pr-number must be a number, got 'abc'\n"))


if __name__ == "__main__":
    unittest.main()
