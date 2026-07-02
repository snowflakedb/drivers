#!/usr/bin/env python3
"""Pre-commit validator for driver BehaviorDifferences.yaml files.

File shape:
  behavior_differences:
    <positive_int_id>:
      <fields...>

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

  ODBC-only rule:
    When reviewed is true, status_rationale must be a non-empty string.

  Conventions for new entries (not auto-filled by this script):
    impact: high
    is_breaking_change: true

  Rules:
    - IDs are unique positive integers.
    - Only the fields listed above are allowed (no unknown keys).
    - Entries must not be removed vs. HEAD (additions and edits only).
"""

from __future__ import annotations

import enum
import subprocess
import sys
from pathlib import Path

import yaml


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

ODBC_BD_RELPATH = "odbc_tests/BehaviorDifferences.yaml"

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


def _committed_ids(git_relpath: str) -> set[int]:
    """Return the set of BD IDs from the last committed version of the file."""
    try:
        raw = subprocess.check_output(
            ["git", "show", f"HEAD:{git_relpath}"],
            stderr=subprocess.DEVNULL,
        )
        data = yaml.safe_load(raw)
        if not isinstance(data, dict):
            return set()
        entries = data.get("behavior_differences", {})
        if not isinstance(entries, dict):
            return set()
        return {int(k) for k in entries if isinstance(entries[k], dict)}
    except (subprocess.CalledProcessError, ValueError, TypeError):
        return set()


def validate_file(git_relpath: str, yaml_path: Path) -> list[str]:
    errors: list[str] = []
    label = git_relpath

    if not yaml_path.is_file():
        errors.append(f"{label}: file not found at {yaml_path}")
        return errors

    with open(yaml_path, encoding="utf-8") as f:
        data = yaml.safe_load(f)

    if not isinstance(data, dict) or "behavior_differences" not in data:
        errors.append(f"{label}: missing top-level 'behavior_differences' key")
        return errors

    entries = data["behavior_differences"]
    if not isinstance(entries, dict):
        errors.append(f"{label}: 'behavior_differences' must be a mapping of ID → entry")
        return errors

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

        unknown_fields = set(entry.keys()) - KNOWN_FIELDS
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

        if git_relpath == ODBC_BD_RELPATH and reviewed is True:
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

    old_ids = _committed_ids(git_relpath)
    removed = old_ids - seen_ids
    if removed:
        for bd_id in sorted(removed):
            errors.append(f"{label} BD-{bd_id}: entry was removed — removals are not allowed")

    return errors


def validate() -> list[str]:
    errors: list[str] = []
    for git_relpath, yaml_path in BD_FILES:
        errors.extend(validate_file(git_relpath, yaml_path))
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
    return 0


if __name__ == "__main__":
    sys.exit(main())
