#!/usr/bin/env python3
import argparse, json, os, sys
from typing import Dict, Iterable, Set


def load_json(path: str) -> dict:
    try:
        with open(path, "r", encoding="utf-8") as f:
            return json.load(f)
    except FileNotFoundError:
        print(f"[compare] missing file: {path}", file=sys.stderr)
        sys.exit(2)

def extract_test_outcomes(report_data: dict) -> Dict[str, str]:
    """Extract test outcomes from pytest-json-report format.
    
    Returns a dict mapping nodeid -> outcome for all tests in the report.
    """
    tests = report_data.get("tests", [])
    return {test["nodeid"]: test["outcome"] for test in tests}

def filter_universal_by_reference_nodeids(universal_outcomes: Dict[str, str], reference_nodeids: Set[str]) -> Dict[str, str]:
    """Filter universal test outcomes to only include tests that also ran for reference.
    
    Args:
        universal_outcomes: All universal test results (nodeid -> outcome)
        reference_nodeids: Set of nodeids that actually ran for reference driver
        
    Returns:
        Filtered universal outcomes containing only tests that also ran for reference
    """
    return {
        nodeid: outcome 
        for nodeid, outcome in universal_outcomes.items() 
        if nodeid in reference_nodeids
    }


def categorize_test_outcomes(outcomes: Dict[str, str]) -> Dict[str, Set[str]]:
    """Categorize tests by outcome in a single pass."""
    categories = {
        "passed": set(),
        "failed": set(), 
        "skipped": set()
    }
    
    for test_id, outcome in outcomes.items():
        if outcome == "passed":
            categories["passed"].add(test_id)
        elif outcome in ("failed", "error"):
            categories["failed"].add(test_id)
        elif outcome == "skipped":
            categories["skipped"].add(test_id)
    
    return categories


def format_test_list(title: str, test_ids: Iterable[str], limit: int = 80) -> str:
    """Format a list of test IDs as markdown."""
    test_set = set(test_ids)
    lines = [f"### {title} ({len(test_set)})"]
    
    if not test_set:
        return "\n".join(lines + ["_none_", ""])
    
    for i, test_id in enumerate(sorted(test_set)):
        if i >= limit:
            lines.append(f"- … and {len(test_set) - limit} more")
            break
        lines.append(f"- `{test_id}`")
    
    lines.append("")
    return "\n".join(lines)


def main():
    parser = argparse.ArgumentParser(description="Compare test results between universal and reference drivers")
    parser.add_argument("--py", required=True, help="Python version label, e.g. 3.12")
    parser.add_argument("--os", required=True, help="OS version label, e.g. ubuntu-latest")
    parser.add_argument("--universal", required=True, help="Path to universal driver JSON report")
    parser.add_argument("--reference", required=True, help="Path to reference driver JSON report")
    parser.add_argument("--summary", default="", help="Path to GITHUB_STEP_SUMMARY (optional)")
    parser.add_argument("--fail-on-regressions", type=int, default=0, help="1 to exit nonzero if regressions exist")
    args = parser.parse_args()

    # Load and extract all test outcomes from both reports
    all_universal_outcomes = extract_test_outcomes(load_json(args.universal))
    reference_outcomes = extract_test_outcomes(load_json(args.reference))
    
    # Filter universal to only tests that ran for reference (in case there were some tests apart from integration added)
    reference_nodeids = set(reference_outcomes.keys())
    universal_outcomes = filter_universal_by_reference_nodeids(all_universal_outcomes, reference_nodeids)

    # Categorize outcomes in single pass for each driver
    universal_categories = categorize_test_outcomes(universal_outcomes)
    reference_categories = categorize_test_outcomes(reference_outcomes)

    # Extract categorized sets for readability
    universal_passed = universal_categories["passed"]
    universal_failed = universal_categories["failed"]
    universal_skipped = universal_categories["skipped"]
    
    reference_passed = reference_categories["passed"]
    reference_failed = reference_categories["failed"]
    reference_skipped = reference_categories["skipped"]

    # Analyze differences
    regressions_from_pass = reference_passed & universal_failed  # Reference passed, universal failed
    regressions_from_fail = reference_failed & universal_passed  # Reference failed, universal passed
    both_failing = reference_failed & universal_failed
    universal_only_skipped = universal_skipped - reference_skipped
    reference_only_skipped = reference_skipped - universal_skipped

    # Generate report=
    header = f"## Universal vs Reference — Python {args.py} on {args.os}\n"
    summary_stats = (
        f"- Universal (all tests): {len(all_universal_outcomes)} tests\n"
        f"- Universal (matched with reference tests): {len(universal_outcomes)} | "
        f"pass {len(universal_passed)} / fail {len(universal_failed)} / skip {len(universal_skipped)}\n"
        f"- Reference: {len(reference_outcomes)} | "
        f"pass {len(reference_passed)} / fail {len(reference_failed)} / skip {len(reference_skipped)}\n\n"
    )
    
    comparison_details = "".join([
        format_test_list("Regressions from passing (ref ✅ / universal ❌)", regressions_from_pass),
        format_test_list("Regressions from failing (ref ❌ / universal ✅)", regressions_from_fail),
        format_test_list("Both failing", both_failing),
        format_test_list("Skipped only on universal", universal_only_skipped),
        format_test_list("Skipped only on reference", reference_only_skipped),
    ])
    
    report = header + summary_stats + comparison_details
    print(report)

    # Write to GitHub step summary if requested
    if args.summary:
        try:
            with open(args.summary, "a", encoding="utf-8") as summary_file:
                summary_file.write(report)
        except Exception as e:
            print(f"[compare] could not write summary: {e}", file=sys.stderr)

    # Exit with error code if regressions_from_pass found and requested
    if args.fail_on_regressions and (regressions_from_pass or regressions_from_fail):
        sys.exit(1)

if __name__ == "__main__":
    main()