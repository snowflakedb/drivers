#!/usr/bin/env python3
"""
Test selector: determines which tests to run based on changed files.

Key principle:
  - Wrapper changes only affect THAT wrapper (ODBC change -> ODBC tests only)
  - Core changes propagate to ALL wrappers for the matching feature
    (core auth change -> core auth tests + ODBC auth + Python auth + JDBC auth)
  - Shared infrastructure changes -> ALL tests for ALL components
  - Tests are grouped by level: unit, integ, e2e

CI integration:
  - GHA: when a suite runs it runs the full unit-test group; uses detect-changes run_* flags to decide which suites run or skip
  - Buildkite: uses this script with --group integ,e2e for per-feature filtering

Usage:
    # Auto-detect base ref and changed files (works on Buildkite and locally)
    python ci/select_tests.py --driver rust
    python ci/select_tests.py --driver odbc --group integ,e2e

    # Combined groups (Buildkite runs integ + e2e in one step)
    python ci/select_tests.py --driver odbc --group integ,e2e
    python ci/select_tests.py --driver python --group integ,e2e

    # Single group
    python ci/select_tests.py --driver java --group e2e
    python ci/select_tests.py --driver java --group unit

    # Provide changed files explicitly
    echo "sf_core/src/auth.rs" | python ci/select_tests.py --driver rust --group e2e --stdin

    # Output as JSON
    python ci/select_tests.py --driver rust --json

Output:
    <filter>  — run selected tests matching the filter
    ALL       — run all tests for this driver/group
    SKIP      — no relevant changes, skip this driver entirely
"""

import argparse
import fnmatch
import functools
import json
import os
import re
import subprocess
import sys
from typing import Dict, List, Optional, Set

try:
    import yaml
except ImportError:
    yaml = None


# Which components can produce tests for a given driver
DRIVER_SOURCES = {
    "rust": ["core"],
    "odbc": ["core", "odbc"],
    "python": ["core", "python"],
    "java": ["core", "java"],
}


def detect_base_ref(explicit_base_ref: Optional[str] = None) -> str:
    """Determine the git base ref from CI environment or explicit override.

    Priority: explicit --base-ref flag > Buildkite env > default (origin/main).
    """
    if explicit_base_ref is not None:
        return explicit_base_ref

    if os.environ.get("BUILDKITE"):
        base_branch = os.environ.get("BUILDKITE_PULL_REQUEST_BASE_BRANCH") or "main"
        return f"origin/{base_branch}"

    return "origin/main"


def load_config(config_path: str) -> Dict:
    """Load the test matrix YAML config."""
    if yaml is None:
        print("ERROR: PyYAML required. Install: pip install pyyaml", file=sys.stderr)
        sys.exit(1)
    with open(config_path) as f:
        return yaml.safe_load(f)


def _ensure_base_ref_available(base_ref: str) -> None:
    """Fetch the base ref if not already available (handles shallow clones)."""
    try:
        subprocess.run(
            ["git", "cat-file", "-t", base_ref],
            stdout=subprocess.PIPE, stderr=subprocess.PIPE, universal_newlines=True, check=True,
        )
    except subprocess.CalledProcessError:
        remote_branch = base_ref.replace("origin/", "", 1) if base_ref.startswith("origin/") else base_ref
        print("Fetching {} (not available locally)...".format(remote_branch), file=sys.stderr)
        try:
            subprocess.run(
                ["git", "fetch", "origin", remote_branch, "--depth=1"],
                stdout=subprocess.PIPE, stderr=subprocess.PIPE, universal_newlines=True, check=True,
            )
        except subprocess.CalledProcessError:
            print("WARNING: failed to fetch {}".format(remote_branch), file=sys.stderr)


def get_changed_files(base_ref: str) -> Optional[List[str]]:
    """Get changed files from git diff. Returns [] when no files changed, None if diff fails (run all)."""
    _ensure_base_ref_available(base_ref)
    for cmd in [
        ["git", "diff", "--name-only", f"{base_ref}...HEAD"],
        ["git", "diff", "--name-only", base_ref],
    ]:
        try:
            result = subprocess.run(cmd, stdout=subprocess.PIPE, stderr=subprocess.PIPE,
                                       universal_newlines=True, check=True)
            return [f.strip() for f in result.stdout.strip().splitlines() if f.strip()]
        except subprocess.CalledProcessError:
            continue
    print("WARNING: git diff failed, running all tests", file=sys.stderr)
    return None


@functools.lru_cache(maxsize=256)
def _glob_to_regex(pattern: str) -> "re.Pattern[str]":
    """Convert a glob pattern to a compiled regex, with proper ** semantics.

    Results are cached so repeated calls with the same pattern avoid
    redundant regex compilation.

    - ``*``  matches anything except ``/``
    - ``**`` matches zero or more path components
    - ``?``  matches a single non-``/`` character
    """
    result = ""
    i = 0
    while i < len(pattern):
        if pattern[i:i + 3] == "**/":
            result += "(?:.+/)?"
            i += 3
        elif pattern[i:i + 2] == "**":
            result += ".*"
            i += 2
        elif pattern[i] == "*":
            result += "[^/]*"
            i += 1
        elif pattern[i] == "?":
            result += "[^/]"
            i += 1
        else:
            result += re.escape(pattern[i])
            i += 1
    return re.compile("^" + result + "$")


def matches_pattern(filepath: str, pattern: str) -> bool:
    """Check if a filepath matches a glob pattern (supports **)."""
    if "**" not in pattern:
        return fnmatch.fnmatch(filepath, pattern)
    return bool(_glob_to_regex(pattern).match(filepath))


def file_matches_any(filepath: str, patterns: List[str]) -> bool:
    """Check if a file matches any pattern in the list."""
    return any(matches_pattern(filepath, p) for p in patterns)


SINGLE_GROUPS = ("unit", "integ", "e2e")

# How to join multiple filters for each driver's test runner:
#   rust:   "|" (pipe) — cargo test uses substring matching, so the pipeline
#           splits on "|" and runs cargo test once per filter
#   odbc:   "|" (regex OR) — ctest uses regex natively
#   python: " " (space) — pytest takes space-separated paths
#   java:   "|" (pipe) — pipeline splits into multiple --tests args
DRIVER_SEPARATOR = {
    "rust": "|",
    "odbc": "|",
    "python": " ",
    "java": "|",
}


def _parse_group(group_str: str) -> List[str]:
    """Parse a group string into a list of individual groups.

    Accepts: "all", a single group name, or comma-separated (e.g. "integ,e2e").
    """
    if group_str == "all":
        return list(SINGLE_GROUPS)
    return [g.strip() for g in group_str.split(",")]


def _extract_filters_for_group(driver_tests: Optional[Dict], groups: List[str]) -> List[str]:
    """Extract test filter(s) from a driver's test config for the requested groups.

    Args:
        driver_tests: A dict with group keys, a plain string (legacy), or None.
        groups: list of group names, e.g. ["integ", "e2e"].

    Returns a list of filter strings (may be empty).
    """
    if driver_tests is None:
        return []

    if isinstance(driver_tests, str):
        return [driver_tests]

    if not isinstance(driver_tests, dict):
        return []

    filters = []
    for g in groups:
        val = driver_tests.get(g)
        if val:
            filters.append(val)
    return filters


def select_tests(driver: str, changed_files: Optional[List[str]], config: Dict, group: str = "all") -> str:
    """Determine which tests to run for a driver given changed files.

    Args:
        driver: "rust", "odbc", "python", or "java"
        changed_files: list of changed file paths, or None (run all)
        config: parsed test-matrix.yml
        group: "all", a single group, or comma-separated (e.g. "integ,e2e")

    Returns:
        str: test filter, "ALL", or "SKIP"
    """
    groups = _parse_group(group)
    if changed_files is None:
        return "ALL"

    # 1. Check shared fallback — triggers ALL tests for ALL components
    shared_fallbacks = config.get("shared_fallback_paths", [])
    for f in changed_files:
        if file_matches_any(f, shared_fallbacks):
            return "ALL"

    # 2. Determine which config components can produce tests for this driver
    source_components = DRIVER_SOURCES.get(driver, [])

    collected_filters = []
    claimed_files = set()  # Files matched by at least one rule

    for component_name in source_components:
        component = config.get(component_name)
        if not component:
            continue

        # 2a. Check component-level fallback
        component_fallbacks = component.get("fallback_paths", [])
        for f in changed_files:
            if file_matches_any(f, component_fallbacks):
                return "ALL"

        # 2b. Match rules
        rules = component.get("rules", [])
        matched_rules = set()

        for rule in rules:
            rule_name = rule.get("name", "")
            rule_paths = rule.get("paths", [])
            already_matched = rule_name in matched_rules

            for f in changed_files:
                if file_matches_any(f, rule_paths):
                    driver_tests = rule.get("tests", {}).get(driver)
                    filters = _extract_filters_for_group(driver_tests, groups)
                    if filters:
                        claimed_files.add(f)
                        if not already_matched:
                            matched_rules.add(rule_name)
                            collected_filters.extend(filters)
                            already_matched = True
                    else:
                        return "ALL"

    if collected_filters:
        separator = DRIVER_SEPARATOR.get(driver, "|")

        # For space-separated drivers (python), individual filter values may contain
        # multiple space-separated paths (e.g. "tests/unit/test_types.py tests/unit/test_arrow.py").
        # Split these into individual items before deduplicating.
        if separator == " ":
            expanded = []
            for f in collected_filters:
                expanded.extend(f.split())
            collected_filters = expanded

        # Deduplicate while preserving order
        seen = set()
        unique = []
        for f in collected_filters:
            if f not in seen:
                seen.add(f)
                unique.append(f)
        return separator.join(unique)

    # 3. Check for unclaimed files in relevant directories.
    component_prefixes = {
        "core": ["sf_core/"],
        "odbc": ["odbc/", "odbc_tests/"],
        "python": ["python/"],
        "java": ["jdbc/", "jdbc_bridge/"],
    }

    has_unclaimed_relevant_files = False
    for component_name in source_components:
        prefixes = component_prefixes.get(component_name, [])
        for f in changed_files:
            if f in claimed_files:
                continue
            if any(f.startswith(p) for p in prefixes):
                has_unclaimed_relevant_files = True
                break

    if has_unclaimed_relevant_files:
        return "ALL"

    return "SKIP"


def _validate_group(value: str) -> str:
    """Validate --group value: 'all', a single group, or comma-separated."""
    valid = set(SINGLE_GROUPS) | {"all"}
    for g in value.split(","):
        g = g.strip()
        if g not in valid:
            raise argparse.ArgumentTypeError(
                f"invalid group '{g}': choose from {', '.join(sorted(valid))}"
            )
    return value


def main():
    parser = argparse.ArgumentParser(description="Select tests based on changed files")
    parser.add_argument("--driver", required=True, choices=["rust", "odbc", "python", "java"])
    parser.add_argument("--group", default="all", type=_validate_group,
                        help="Test group(s): unit, integ, e2e, all, or comma-separated "
                             "(e.g. integ,e2e). Default: all")
    parser.add_argument("--base-ref", default=None,
                        help="Git base ref for diff. Auto-detected from CI env if omitted")
    parser.add_argument("--config", default=os.path.join(os.path.dirname(__file__), "test-matrix.yml"))
    parser.add_argument("--stdin", action="store_true", help="Read changed files from stdin")
    parser.add_argument("--json", action="store_true", help="Output as JSON")
    parser.add_argument("--verbose", action="store_true", help="Print debug info to stderr")
    args = parser.parse_args()

    base_ref = detect_base_ref(args.base_ref)
    config = load_config(args.config)

    if args.stdin:
        changed_files = [line.strip() for line in sys.stdin if line.strip()]
    else:
        changed_files = get_changed_files(base_ref)

    result = select_tests(args.driver, changed_files, config, group=args.group)

    if args.verbose:
        print(f"Base ref: {base_ref}", file=sys.stderr)
        if changed_files:
            print(f"Changed files ({len(changed_files)}):", file=sys.stderr)
            for f in changed_files:
                print(f"  {f}", file=sys.stderr)
        print(f"Driver: {args.driver}, Group: {args.group}", file=sys.stderr)
        print(f"Result: {result}", file=sys.stderr)

    if args.json:
        print(json.dumps({
            "driver": args.driver,
            "group": args.group,
            "filter": result if result not in ("ALL", "SKIP") else None,
            "run_all": result == "ALL",
            "skip": result == "SKIP",
            "changed_files_count": len(changed_files) if changed_files else 0,
        }))
    else:
        print(result)


if __name__ == "__main__":
    main()
