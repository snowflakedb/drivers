#!/usr/bin/env python3
"""Unified validator for all BehaviorDifferences.yaml files.

Enforces across every BehaviorDifferences.yaml in the repo:
  - Top-level 'behavior_differences' key present
  - IDs are positive integers with no duplicates
  - Each entry contains only known fields
  - Required fields: name, status, type, reviewed
  - 'status' is one of the allowed values
  - 'type' is one of the allowed values
  - 'reviewed' is a boolean
  - Optional 'is_breaking_change' is a boolean when present
  - No existing entries are removed (additions/modifications only)

Run manually against all files:
    python3 scripts/validate_behavior_differences.py

Or against specific files (as pre-commit does):
    python3 scripts/validate_behavior_differences.py path/to/BehaviorDifferences.yaml ...
"""

import subprocess
import sys
from pathlib import Path

import yaml

ALLOWED_STATUSES = {"unknown", "todo", "fixed", "allowed"}
ALLOWED_TYPES = {"api_incompatibility", "bug", "bugfix", "enhancement", "unknown"}
REQUIRED_FIELDS = {"name", "status", "type", "reviewed"}
OPTIONAL_FIELDS = {
    "is_breaking_change",
    "impact",
    "description",
    "old_driver_behavior",
    "new_driver_behavior",
}
KNOWN_FIELDS = REQUIRED_FIELDS | OPTIONAL_FIELDS


def _repo_root() -> Path:
    result = subprocess.check_output(
        ["git", "rev-parse", "--show-toplevel"], text=True
    )
    return Path(result.strip())


def _committed_ids(rel_path: str) -> set[int]:
    """Return BD IDs present in the last committed version of the file."""
    try:
        raw = subprocess.check_output(
            ["git", "show", f"HEAD:{rel_path}"],
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


def validate_file(path: Path, repo_root: Path) -> list[str]:
    errors: list[str] = []

    try:
        rel_path = str(path.relative_to(repo_root))
    except ValueError:
        rel_path = str(path)

    try:
        with open(path, encoding="utf-8") as f:
            data = yaml.safe_load(f)
    except Exception as exc:
        return [f"{rel_path}: failed to parse YAML: {exc}"]

    if not isinstance(data, dict) or "behavior_differences" not in data:
        return [f"{rel_path}: missing top-level 'behavior_differences' key"]

    entries = data["behavior_differences"]
    if not isinstance(entries, dict):
        return [f"{rel_path}: 'behavior_differences' must be a mapping of ID -> entry"]

    seen_ids: set[int] = set()

    for raw_key, entry in entries.items():
        try:
            bd_id = int(raw_key)
        except (ValueError, TypeError):
            errors.append(f"{rel_path}: key '{raw_key}' is not an integer ID")
            continue

        prefix = f"{rel_path} BD-{bd_id}"

        if bd_id <= 0:
            errors.append(f"{prefix}: ID must be a positive integer")

        if bd_id in seen_ids:
            errors.append(f"{prefix}: duplicate ID")
        seen_ids.add(bd_id)

        if not isinstance(entry, dict):
            errors.append(
                f"{prefix}: entry must be a mapping, got {type(entry).__name__}"
            )
            continue

        unknown = set(entry.keys()) - KNOWN_FIELDS
        if unknown:
            errors.append(
                f"{prefix}: unknown field(s): {', '.join(sorted(unknown))}"
            )

        name = entry.get("name")
        if not name or not isinstance(name, str) or not name.strip():
            errors.append(f"{prefix}: 'name' is required and must be a non-empty string")

        status = entry.get("status")
        if status is None:
            errors.append(
                f"{prefix}: 'status' is required"
                f" (allowed: {', '.join(sorted(ALLOWED_STATUSES))})"
            )
        elif status not in ALLOWED_STATUSES:
            errors.append(
                f"{prefix}: invalid status '{status}'"
                f" (allowed: {', '.join(sorted(ALLOWED_STATUSES))})"
            )

        bd_type = entry.get("type")
        if bd_type is None:
            errors.append(
                f"{prefix}: 'type' is required"
                f" (allowed: {', '.join(sorted(ALLOWED_TYPES))})"
            )
        elif bd_type not in ALLOWED_TYPES:
            errors.append(
                f"{prefix}: invalid type '{bd_type}'"
                f" (allowed: {', '.join(sorted(ALLOWED_TYPES))})"
            )

        reviewed = entry.get("reviewed")
        if reviewed is None:
            errors.append(f"{prefix}: 'reviewed' is required")
        elif not isinstance(reviewed, bool):
            errors.append(f"{prefix}: 'reviewed' must be a boolean (true/false)")

        is_breaking = entry.get("is_breaking_change")
        if is_breaking is not None and not isinstance(is_breaking, bool):
            errors.append(
                f"{prefix}: 'is_breaking_change' must be a boolean (true/false)"
            )

    old_ids = _committed_ids(rel_path)
    for bd_id in sorted(old_ids - seen_ids):
        errors.append(f"{rel_path} BD-{bd_id}: entry was removed — removals are not allowed")

    return errors


def main() -> int:
    repo_root = _repo_root()

    if len(sys.argv) > 1:
        paths = [Path(p).resolve() for p in sys.argv[1:]]
    else:
        paths = sorted(repo_root.rglob("BehaviorDifferences.yaml"))

    if not paths:
        print("No BehaviorDifferences.yaml files found.")
        return 0

    all_errors: list[str] = []
    for path in paths:
        errors = validate_file(path, repo_root)
        if errors:
            all_errors.extend(errors)
        else:
            print(f"{path.relative_to(repo_root)}: OK")

    if all_errors:
        print(f"\nValidation failed ({len(all_errors)} error(s)):\n")
        for err in all_errors:
            print(f"  - {err}")
        return 1

    return 0


if __name__ == "__main__":
    sys.exit(main())
