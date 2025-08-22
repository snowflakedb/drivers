#!/usr/bin/env python3
"""
Extract integration test results from a combined (unit + integration) test report.
"""
import argparse
import json
import sys
from pathlib import Path


def extract_integration_tests(combined_report_path: str, output_path: str):
    """Extract only integration tests from a combined test report."""
    
    try:
        with open(combined_report_path, 'r') as f:
            combined_data = json.load(f)
    except FileNotFoundError:
        print(f"Error: Could not find report file: {combined_report_path}", file=sys.stderr)
        sys.exit(1)
    except json.JSONDecodeError as e:
        print(f"Error: Invalid JSON in report file: {e}", file=sys.stderr)
        sys.exit(1)
    
    # Filter tests to only include integration tests (tests/integ/)
    all_tests = combined_data.get("tests", [])
    integ_tests = [
        test for test in all_tests 
        if "tests/integ/" in test.get("nodeid", "")
    ]
    
    # Create new report with only integration tests
    integ_report = combined_data.copy()
    integ_report["tests"] = integ_tests
    
    # Update summary statistics
    integ_summary = integ_report.get("summary", {})
    integ_summary["total"] = len(integ_tests)
    integ_summary["passed"] = len([t for t in integ_tests if t.get("outcome") == "passed"])
    integ_summary["failed"] = len([t for t in integ_tests if t.get("outcome") == "failed"])
    integ_summary["skipped"] = len([t for t in integ_tests if t.get("outcome") == "skipped"])
    integ_summary["error"] = len([t for t in integ_tests if t.get("outcome") == "error"])
    
    # Write the filtered report
    try:
        with open(output_path, 'w') as f:
            json.dump(integ_report, f, indent=2)
        print(f"Extracted {len(integ_tests)} integration tests from {len(all_tests)} total tests")
        print(f"Integration-only report saved to: {output_path}")
    except Exception as e:
        print(f"Error writing output file: {e}", file=sys.stderr)
        sys.exit(1)


def main():
    parser = argparse.ArgumentParser(description="Extract integration test results from combined report")
    parser.add_argument("--input", required=True, help="Path to combined test report JSON")
    parser.add_argument("--output", required=True, help="Path to output integration-only report JSON")
    
    args = parser.parse_args()
    extract_integration_tests(args.input, args.output)


if __name__ == "__main__":
    main()
