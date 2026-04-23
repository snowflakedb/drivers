#!/usr/bin/env python3
"""Pre-commit validator for BehaviorDifferences.yaml.

Enforces:
  - Each entry has a non-empty 'name'
  - 'status' is present and one of the allowed values
  - 'type' is present and one of the allowed values
  - IDs are positive integers with no duplicates
  - Only known fields are used
  - No existing entries are removed (only additions/modifications allowed)
"""

import subprocess
import sys
from pathlib import Path

import yaml

ALLOWED_STATUSES = {"unknown", "todo", "fixed", "allowed"}
ALLOWED_TYPES = {"api_incompatibility", "bug", "enhancement", "unknown"}
ALLOWED_REVIEWED = {True, False}
KNOWN_FIELDS = {
    "name",
    "status",
    "type",
    "reviewed",
    "description",
    "old_driver_behavior",
    "new_driver_behavior",
    "impact",
}

YAML_RELPATH = "odbc_tests/BehaviorDifferences.yaml"
YAML_PATH = Path(__file__).parent / "BehaviorDifferences.yaml"


def _committed_ids() -> set[int]:
    """Return the set of BD IDs from the last committed version of the file."""
    try:
        raw = subprocess.check_output(
            ["git", "show", f"HEAD:{YAML_RELPATH}"],
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


def validate() -> list[str]:
    errors: list[str] = []

    with open(YAML_PATH, encoding="utf-8") as f:
        data = yaml.safe_load(f)

    if not isinstance(data, dict) or "behavior_differences" not in data:
        errors.append("Missing top-level 'behavior_differences' key")
        return errors

    entries = data["behavior_differences"]
    if not isinstance(entries, dict):
        errors.append("'behavior_differences' must be a mapping of ID → entry")
        return errors

    seen_ids: set[int] = set()

    for raw_key, entry in entries.items():
        try:
            bd_id = int(raw_key)
        except (ValueError, TypeError):
            errors.append(f"BD key '{raw_key}': ID must be an integer")
            continue

        if bd_id <= 0:
            errors.append(f"BD-{bd_id}: ID must be positive")

        if bd_id in seen_ids:
            errors.append(f"BD-{bd_id}: duplicate ID")
        seen_ids.add(bd_id)

        if not isinstance(entry, dict):
            errors.append(f"BD-{bd_id}: entry must be a mapping, got {type(entry).__name__}")
            continue

        unknown_fields = set(entry.keys()) - KNOWN_FIELDS
        if unknown_fields:
            errors.append(f"BD-{bd_id}: unknown field(s): {', '.join(sorted(unknown_fields))}")

        name = entry.get("name")
        if not name or not isinstance(name, str) or not name.strip():
            errors.append(f"BD-{bd_id}: 'name' is required and must be non-empty")

        status = entry.get("status")
        if status is None:
            errors.append(f"BD-{bd_id}: 'status' is required (allowed: {', '.join(sorted(ALLOWED_STATUSES))})")
        elif status not in ALLOWED_STATUSES:
            errors.append(f"BD-{bd_id}: invalid status '{status}' (allowed: {', '.join(sorted(ALLOWED_STATUSES))})")

        bd_type = entry.get("type")
        if bd_type is None:
            errors.append(f"BD-{bd_id}: 'type' is required (allowed: {', '.join(sorted(ALLOWED_TYPES))})")
        elif bd_type not in ALLOWED_TYPES:
            errors.append(f"BD-{bd_id}: invalid type '{bd_type}' (allowed: {', '.join(sorted(ALLOWED_TYPES))})")

        reviewed = entry.get("reviewed")
        if reviewed is not None and not isinstance(reviewed, bool):
            errors.append(f"BD-{bd_id}: 'reviewed' must be a boolean (true/false)")

    old_ids = _committed_ids()
    removed = old_ids - seen_ids
    if removed:
        for bd_id in sorted(removed):
            errors.append(f"BD-{bd_id}: entry was removed — removals are not allowed")

    return errors


def main() -> int:
    errors = validate()
    if errors:
        print(f"BehaviorDifferences.yaml validation failed ({len(errors)} error(s)):\n")
        for err in errors:
            print(f"  - {err}")
        return 1
    print("BehaviorDifferences.yaml: OK")
    return 0


if __name__ == "__main__":
    sys.exit(main())
