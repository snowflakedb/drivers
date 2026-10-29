#!/usr/bin/env python3
"""Pre-commit validator for driver behavior-difference registries.

Monolith file shape (every driver):
  behavior_differences:
    <positive_int_id>:
      <fields...>

Python, ODBC, and JDBC also accept one file per entry under
``<driver>/behavior_differences/<area>/<slug>.yaml`` (``python/behavior_differences``,
``odbc_tests/behavior_differences``, or ``jdbc/behavior_differences``). The file body is
the entry itself (no behavior_differences wrapper). The slug is the id. A new Python,
ODBC, or JDBC integer key in the monolith is rejected. Deleting an existing integer key
there is allowed only when exactly one of those files sets legacy_id to that key, and
the key is no longer in the monolith. That legacy_id stays valid after the deletion is
in the baseline. The baseline is HEAD locally and BD_BASE_SHA in CI (the pull-request
base), because ``pre-commit run --all-files`` checks out a tree that already contains
the change. Node.js stays integer-only; removals there are still forbidden.

Per-entry schema:

  Required:
    name (str) — short title; must be non-empty.
    status (str) — see BDStatus.
    type (str) — see BDType.
    impact (str) — see BDImpact.
    is_breaking_change (bool) — true if existing application code may fail or produce
      different results when switching from the old driver to the new driver without changes.
      false for pure improvements where old callers automatically benefit (bug fixes that
      were erroneously failing before, enhancements that add new success paths, etc.).

  Optional:
    reviewed (bool) — triage flag; must be true/false when present.
    status_rationale (str) — required when reviewed is true (ODBC/JDBC convention).
    description (str) — extra context, migration notes, spec links.
    old_driver_behavior (str) — behavior of the legacy/reference driver.
    new_driver_behavior (str) — behavior of the universal driver.
    legacy_id (int) — Python, ODBC, and JDBC files only. The monolith integer this
      file replaces. Required when that integer is removed versus the baseline, and
      rejected while the integer is still a key in the monolith. It stays set after
      the baseline itself no longer has the key, so BD#<integer> keeps resolving.

  ODBC and JDBC rule:
    When reviewed is true, status_rationale must be a non-empty string.

  Conventions for new entries (not auto-filled by this script):
    impact: high
    is_breaking_change: true

  Rules:
    - IDs are unique positive integers. Uniqueness is checked against the raw file
      text (not just the parsed mapping) because ``yaml.safe_load`` silently keeps
      only the last of any duplicate key, which would otherwise hide the collision.
    - The raw ID-line count must equal the number of unique parsed entries; a
      mismatch means a duplicate key silently dropped an entry on load.
    - Only the fields listed above are allowed (no unknown keys). legacy_id is
      allowed on Python, ODBC, and JDBC per-file entries only.
    - Entries must not be removed versus the baseline, except a Python, ODBC, or
      JDBC integer key replaced by a file whose legacy_id is that key. The file
      keeps legacy_id after that deletion is committed.
"""

from __future__ import annotations

import enum
import os
import re
import subprocess
import sys
from dataclasses import dataclass, field
from pathlib import Path, PurePosixPath

import yaml

# Matches a top-level behavior-difference ID line, e.g. "  42:" nested directly
# under the "behavior_differences:" mapping (exactly two leading spaces).
_ID_LINE_RE = re.compile(r"^  (\d+):[ \t]*(?:#.*)?$")


class BDStatus(str, enum.Enum):
    """Lifecycle state of a behavioral difference."""

    # not yet triaged; default for new entries
    UNKNOWN = "unknown"
    # confirmed difference that must be fixed before GA
    TODO = "todo"
    # the difference has been eliminated; new driver now matches old
    FIXED = "fixed"
    # intentional and accepted (e.g. spec compliance, enhancement)
    ALLOWED = "allowed"


class BDType(str, enum.Enum):
    """Root cause classification of a behavioral difference."""

    # not yet classified; default for new entries
    UNKNOWN = "unknown"
    # new driver removes or renames a public API surface (method, property, connection parameter)
    API_INCOMPATIBILITY = "api_incompatibility"
    # new driver has an unintentional regression; old behavior was correct or expected by callers
    BUG = "bug"
    # new driver fixes incorrect old-driver behavior; old was wrong, new is correct
    BUGFIX = "bugfix"
    # new driver extends or improves functionality beyond what the old driver offered
    ENHANCEMENT = "enhancement"


class BDImpact(str, enum.Enum):
    """Severity for consumers migrating from the old driver to the new driver."""

    # edge case; affects very few callers or requires non-default configuration
    LOW = "low"
    # noticeable but limited; affects specific use cases or data types
    MEDIUM = "medium"
    # broad; affects common usage patterns or may break many callers
    HIGH = "high"


KNOWN_FIELDS = {
    "name",
    "status",
    "status_rationale",
    "type",
    "reviewed",
    "description",
    "old_driver_behavior",
    "new_driver_behavior",
    "impact",
    "is_breaking_change",
}

FILE_KNOWN_FIELDS = KNOWN_FIELDS | {"legacy_id"}

# Allowed parent directories of per-file entries.
BD_AREAS = frozenset({
    "auth",
    "config",
    "connection",
    "errors",
    "file-transfer",
    "metadata",
    "put-get",
    "query",
    "session",
    "types",
})

# Kebab-case. The first segment is at least two characters: ``ab`` and ``ab-c``
# match, ``a-b`` does not. A leading letter excludes an all-digit id.
_SLUG_RE = re.compile(r"^[a-z][a-z0-9]+(?:-[a-z0-9]+)*$")
_MAX_SLUG_LEN = 64

# Monolith path -> directory of per-file entries. Drivers absent from this map
# stay on the integer-key monolith only.
GATED_FILE_DIRS: dict[str, str] = {
    "python/BehaviorDifferences.yaml": "python/behavior_differences",
    "odbc_tests/BehaviorDifferences.yaml": "odbc_tests/behavior_differences",
    "jdbc/BehaviorDifferences.yaml": "jdbc/behavior_differences",
}

# Monolith paths whose entries, and per-file entries, need a non-empty
# status_rationale once reviewed is true.
STATUS_RATIONALE_REQUIRED = frozenset({
    "odbc_tests/BehaviorDifferences.yaml",
    "jdbc/BehaviorDifferences.yaml",
})

DEFAULT_IMPACT = BDImpact.HIGH
DEFAULT_IS_BREAKING_CHANGE = True

_REPO_ROOT = Path(__file__).resolve().parents[1]

# (workspace-relative path, absolute path)
BD_FILES: list[tuple[str, Path]] = [
    ("odbc_tests/BehaviorDifferences.yaml", _REPO_ROOT / "odbc_tests" / "BehaviorDifferences.yaml"),
    ("python/BehaviorDifferences.yaml", _REPO_ROOT / "python" / "BehaviorDifferences.yaml"),
    ("jdbc/BehaviorDifferences.yaml", _REPO_ROOT / "jdbc" / "BehaviorDifferences.yaml"),
    ("nodejs/BehaviorDifferences.yaml", _REPO_ROOT / "nodejs" / "BehaviorDifferences.yaml"),
]


def _rev_exists(rev: str) -> bool:
    try:
        subprocess.check_output(
            ["git", "rev-parse", "--verify", "--end-of-options", f"{rev}^{{commit}}"],
            stderr=subprocess.DEVNULL,
        )
        return True
    except (subprocess.CalledProcessError, OSError):
        return False


def _baseline_rev() -> str:
    """Revision the monolith diff compares against.

    Unset ``BD_BASE_SHA`` means HEAD, which is the parent during a local commit.
    CI sets ``BD_BASE_SHA`` to the pull-request base so a committed addition or
    removal is still visible. An unusable value falls back to HEAD.
    """
    baseline = os.environ.get("BD_BASE_SHA", "").strip()
    if baseline and _rev_exists(baseline):
        return baseline
    return "HEAD"


def _committed_ids(git_relpath: str, rev: str) -> set[int] | None:
    """Return BD IDs from ``rev`` of the monolith, or None when ``rev`` cannot be read.

    None skips the diff rules (new integer keys and removals). An empty set means
    the revision was read and contains no entries.
    """
    try:
        raw = subprocess.check_output(
            ["git", "show", f"{rev}:{git_relpath}"],
            stderr=subprocess.DEVNULL,
        )
        data = yaml.safe_load(raw)
        if not isinstance(data, dict):
            return set()
        entries = data.get("behavior_differences", {})
        if not isinstance(entries, dict):
            return set()
        return {int(k) for k in entries if isinstance(entries[k], dict)}
    except (subprocess.CalledProcessError, ValueError, TypeError, yaml.YAMLError):
        return None


def _raw_id_lines(text: str) -> list[int]:
    """Return every top-level BD ID as it literally appears in the file, in order.

    Scans raw text rather than the parsed mapping so that duplicate keys — which
    ``yaml.safe_load`` silently collapses to the last occurrence — are still
    visible. This is the foundation for the duplicate/count/ordering guards.
    """
    ids: list[int] = []
    for line in text.splitlines():
        match = _ID_LINE_RE.match(line)
        if match:
            ids.append(int(match.group(1)))
    return ids


def _raw_structure_errors(label: str, text: str, parsed_ids: set[int]) -> list[str]:
    """Guards that operate on the raw file text, catching failure modes that
    ``yaml.safe_load`` hides:

    1. Duplicate IDs — the same ``N:`` key appearing more than once. PyYAML keeps
       only the last occurrence, silently dropping the earlier entry, so the
       parsed-mapping duplicate check in ``check_monolith_text`` never sees it.
    2. Parse loss — the raw ID-line count not matching the number of unique
       entries the parser produced. A mismatch means a duplicate key silently
       merged two blocks on load (belt-and-suspenders alongside guard 1).
    """
    errors: list[str] = []
    raw_ids = _raw_id_lines(text)

    if not raw_ids:
        # No ID lines found; check_monolith_text reports the missing mapping.
        return errors

    # Guard 1: duplicate IDs (order-preserving, report each dup once).
    seen: set[int] = set()
    reported: set[int] = set()
    for bd_id in raw_ids:
        if bd_id in seen and bd_id not in reported:
            errors.append(f"{label} BD-{bd_id}: duplicate ID in file (YAML keeps only the last)")
            reported.add(bd_id)
        seen.add(bd_id)

    # Guard 2: raw block count must match the parser's unique-entry count. A
    # mismatch means a duplicate key silently merged two blocks on load.
    if len(raw_ids) != len(parsed_ids):
        errors.append(
            f"{label}: {len(raw_ids)} ID line(s) in file but {len(parsed_ids)} unique "
            f"entr{'y' if len(parsed_ids) == 1 else 'ies'} after parsing — a duplicate "
            f"key was silently dropped"
        )

    return errors


def _field_errors(
    prefix: str,
    entry: dict,
    *,
    known_fields: set[str],
    require_status_rationale: bool,
) -> list[str]:
    errors: list[str] = []

    unknown_fields = set(entry.keys()) - known_fields
    if unknown_fields:
        errors.append(f"{prefix}: unknown field(s): {', '.join(sorted(unknown_fields))}")

    name = entry.get("name")
    if not name or not isinstance(name, str) or not name.strip():
        errors.append(f"{prefix}: 'name' is required and must be non-empty")

    status = entry.get("status")
    if status is None:
        allowed = ", ".join(s.value for s in BDStatus)
        errors.append(f"{prefix}: 'status' is required (allowed: {allowed})")
    else:
        try:
            BDStatus(status)
        except ValueError:
            allowed = ", ".join(s.value for s in BDStatus)
            errors.append(f"{prefix}: invalid status '{status}' (allowed: {allowed})")

    bd_type = entry.get("type")
    if bd_type is None:
        allowed = ", ".join(s.value for s in BDType)
        errors.append(f"{prefix}: 'type' is required (allowed: {allowed})")
    else:
        try:
            BDType(bd_type)
        except ValueError:
            allowed = ", ".join(s.value for s in BDType)
            errors.append(f"{prefix}: invalid type '{bd_type}' (allowed: {allowed})")

    reviewed = entry.get("reviewed")
    if reviewed is not None and not isinstance(reviewed, bool):
        errors.append(f"{prefix}: 'reviewed' must be a boolean (true/false)")

    if require_status_rationale and reviewed is True:
        rationale = entry.get("status_rationale")
        if not rationale or not isinstance(rationale, str) or not rationale.strip():
            errors.append(
                f"{prefix}: non-empty 'status_rationale' is required when reviewed is true"
            )

    impact = entry.get("impact")
    if impact is None:
        allowed = ", ".join(s.value for s in BDImpact)
        errors.append(
            f"{prefix}: 'impact' is required"
            f" (allowed: {allowed}; default: {DEFAULT_IMPACT.value})"
        )
    else:
        try:
            BDImpact(impact)
        except ValueError:
            allowed = ", ".join(s.value for s in BDImpact)
            errors.append(
                f"{prefix}: invalid impact '{impact}'"
                f" (allowed: {allowed}; narrative impact text belongs in 'description')"
            )

    is_breaking = entry.get("is_breaking_change")
    if is_breaking is None:
        errors.append(
            f"{prefix}: 'is_breaking_change' is required"
            f" (boolean; default: {str(DEFAULT_IS_BREAKING_CHANGE).lower()})"
        )
    elif not isinstance(is_breaking, bool):
        errors.append(f"{prefix}: 'is_breaking_change' must be a boolean (true/false)")

    return errors


@dataclass
class MonolithCheck:
    errors: list[str]
    ids: set[int]
    # Integer keys present in committed_ids and absent from the text.
    # Empty when committed_ids is None (the baseline revision was not available).
    removed: set[int] = field(default_factory=set)


def check_monolith_text(
    git_relpath: str,
    text: str,
    committed_ids: set[int] | None,
    *,
    gated_file_dir: str | None = None,
) -> MonolithCheck:
    """Validate one monolith document.

    When ``gated_file_dir`` is set, a positive integer key that is not in
    ``committed_ids`` is an error, and removals are reported on ``removed``
    instead of as errors. ``check_migration`` decides whether each removal has
    a matching ``legacy_id``. When ``committed_ids`` is None, those diff rules
    are skipped.
    """
    errors: list[str] = []
    label = git_relpath

    data = yaml.safe_load(text)

    if not isinstance(data, dict) or "behavior_differences" not in data:
        errors.append(f"{label}: missing top-level 'behavior_differences' key")
        return MonolithCheck(errors, set())

    entries = data["behavior_differences"]
    if not isinstance(entries, dict):
        errors.append(f"{label}: 'behavior_differences' must be a mapping of ID → entry")
        return MonolithCheck(errors, set())

    seen_ids: set[int] = set()

    for raw_key, entry in entries.items():
        try:
            bd_id = int(raw_key)
        except (ValueError, TypeError):
            errors.append(f"{label} BD key '{raw_key}': ID must be an integer")
            continue

        prefix = f"{label} BD-{bd_id}"

        if bd_id <= 0:
            errors.append(f"{prefix}: ID must be positive")

        if bd_id in seen_ids:
            errors.append(f"{prefix}: duplicate ID")
        seen_ids.add(bd_id)

        if not isinstance(entry, dict):
            errors.append(f"{prefix}: entry must be a mapping, got {type(entry).__name__}")
            continue

        errors.extend(
            _field_errors(
                prefix,
                entry,
                known_fields=KNOWN_FIELDS,
                require_status_rationale=git_relpath in STATUS_RATIONALE_REQUIRED,
            )
        )

    errors.extend(_raw_structure_errors(label, text, seen_ids))

    removed: set[int] = set()
    if committed_ids is not None:
        removed = committed_ids - seen_ids
        if gated_file_dir is None:
            for bd_id in sorted(removed):
                errors.append(f"{label} BD-{bd_id}: entry was removed — removals are not allowed")
        else:
            added = {bd_id for bd_id in seen_ids if bd_id > 0} - committed_ids
            for bd_id in sorted(added):
                errors.append(
                    f"{label} BD-{bd_id}: new entries must be a file at "
                    f"{gated_file_dir}/<area>/<slug>.yaml"
                )

    return MonolithCheck(errors, seen_ids, removed)


@dataclass
class BdFilesCheck:
    errors: list[str]
    # legacy_id -> repo-relative file path. Duplicates are errors and only the
    # first path is kept.
    legacy_ids: dict[int, str] = field(default_factory=dict)


def _legacy_id_value(prefix: str, raw: object) -> tuple[int | None, str | None]:
    if isinstance(raw, bool) or not isinstance(raw, int) or raw <= 0:
        return None, f"{prefix}: 'legacy_id' must be a positive integer"
    return raw, None


def check_bd_files(
    dir_label: str,
    files: list[tuple[str, str]],
    *,
    require_status_rationale: bool = False,
) -> BdFilesCheck:
    """Validate per-file entries.

    ``files`` is ``(path relative to dir_label, YAML text)`` using forward
    slashes, each expected to be ``<area>/<slug>.yaml``. ODBC and JDBC pass
    ``require_status_rationale`` so a reviewed entry still needs a non-empty
    ``status_rationale``.
    """
    errors: list[str] = []
    legacy_ids: dict[int, str] = {}
    seen_slugs: dict[str, str] = {}
    areas = ", ".join(sorted(BD_AREAS))

    for rel, text in files:
        path_label = f"{dir_label}/{rel}"
        parts = PurePosixPath(rel).parts
        if len(parts) != 2 or PurePosixPath(rel).suffix != ".yaml":
            errors.append(f"{path_label}: expected <area>/<slug>.yaml")
            continue

        area, filename = parts
        slug = filename[: -len(".yaml")]
        if area not in BD_AREAS:
            errors.append(f"{path_label}: area '{area}' is not allowed (allowed: {areas})")
        if len(slug) > _MAX_SLUG_LEN or _SLUG_RE.fullmatch(slug) is None:
            errors.append(
                f"{path_label}: filename '{slug}' must be a kebab-case slug "
                f"matching {_SLUG_RE.pattern} and at most {_MAX_SLUG_LEN} characters"
            )
        elif slug in seen_slugs:
            errors.append(f"{path_label}: duplicate slug '{slug}' (also {seen_slugs[slug]})")
        else:
            seen_slugs[slug] = path_label

        try:
            data = yaml.safe_load(text)
        except yaml.YAMLError as exc:
            errors.append(f"{path_label}: failed to parse YAML: {exc}")
            continue

        if not isinstance(data, dict):
            errors.append(f"{path_label}: entry must be a mapping, got {type(data).__name__}")
            continue
        if "behavior_differences" in data:
            errors.append(
                f"{path_label}: file is one entry; do not wrap it in 'behavior_differences'"
            )

        errors.extend(
            _field_errors(
                path_label,
                data,
                known_fields=FILE_KNOWN_FIELDS,
                require_status_rationale=require_status_rationale,
            )
        )

        if "legacy_id" not in data:
            continue
        legacy_id, legacy_error = _legacy_id_value(path_label, data["legacy_id"])
        if legacy_error is not None or legacy_id is None:
            if legacy_error is not None:
                errors.append(legacy_error)
            continue
        if legacy_id in legacy_ids:
            errors.append(
                f"{path_label}: duplicate legacy_id {legacy_id} (also {legacy_ids[legacy_id]})"
            )
            continue
        legacy_ids[legacy_id] = path_label

    return BdFilesCheck(errors, legacy_ids)


def check_migration(
    monolith_label: str,
    current_ids: set[int],
    removed: set[int],
    legacy_ids: dict[int, str],
) -> list[str]:
    """Pair monolith deletions with per-file ``legacy_id`` values.

    A key in ``removed`` needs a file whose ``legacy_id`` is that integer.
    ``legacy_id`` is rejected while the integer is still in ``current_ids``.
    After the baseline no longer contains the integer, ``removed`` is empty and
    the file keeps ``legacy_id``.
    """
    errors: list[str] = []
    for bd_id in sorted(removed - set(legacy_ids)):
        errors.append(
            f"{monolith_label} BD-{bd_id}: entry was removed — removals are not allowed "
            f"unless a file sets legacy_id: {bd_id}"
        )
    for bd_id, path_label in sorted(legacy_ids.items()):
        if bd_id in current_ids:
            errors.append(
                f"{path_label}: legacy_id {bd_id} is still a key in {monolith_label}"
            )
    return errors


def _read_bd_dir(dir_path: Path) -> list[tuple[str, str]]:
    if not dir_path.is_dir():
        return []
    files: list[tuple[str, str]] = []
    for path in sorted(dir_path.rglob("*")):
        if path.is_file():
            rel = path.relative_to(dir_path).as_posix()
            files.append((rel, path.read_text(encoding="utf-8")))
    return files


def validate() -> list[str]:
    errors: list[str] = []
    baseline = _baseline_rev()
    for git_relpath, yaml_path in BD_FILES:
        if not yaml_path.is_file():
            errors.append(f"{git_relpath}: file not found at {yaml_path}")
            continue

        text = yaml_path.read_text(encoding="utf-8")
        committed_ids = _committed_ids(git_relpath, baseline)
        gated_file_dir = GATED_FILE_DIRS.get(git_relpath)
        monolith = check_monolith_text(
            git_relpath,
            text,
            committed_ids,
            gated_file_dir=gated_file_dir,
        )
        errors.extend(monolith.errors)

        if gated_file_dir is None:
            continue

        dir_path = _REPO_ROOT / gated_file_dir
        if dir_path.exists() and not dir_path.is_dir():
            errors.append(f"{gated_file_dir}: expected a directory of <area>/<slug>.yaml files")
            continue

        file_check = check_bd_files(
            gated_file_dir,
            _read_bd_dir(dir_path),
            require_status_rationale=git_relpath in STATUS_RATIONALE_REQUIRED,
        )
        errors.extend(file_check.errors)
        errors.extend(
            check_migration(
                git_relpath,
                monolith.ids,
                monolith.removed,
                file_check.legacy_ids,
            )
        )

    return errors


def main() -> int:
    errors = validate()
    if errors:
        print(f"BehaviorDifferences.yaml validation failed ({len(errors)} error(s)):\n")
        for err in errors:
            print(f"  - {err}")
        return 1
    for git_relpath, _ in BD_FILES:
        print(f"{git_relpath}: OK")
        gated_file_dir = GATED_FILE_DIRS.get(git_relpath)
        if gated_file_dir is not None:
            print(f"{gated_file_dir}: OK")
    return 0


if __name__ == "__main__":
    sys.exit(main())
