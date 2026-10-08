#!/usr/bin/env python3
"""Inject Jenkins flakyFailure / rerunFailure into CTest JUnit.

CTest ``--repeat until-pass:N --output-junit`` keeps only the last attempt.
The verbose log still lists every Failed/Passed line, so we reconstruct retry
history and write Surefire-style children the dashboard parser already reads.
"""

from __future__ import annotations

import argparse
import re
import sys
import xml.etree.ElementTree as ET
from collections import defaultdict
from pathlib import Path

TEST_LINE = re.compile(
    r"Test\s+#\d+:\s+(.+?)\s+\.{3,}\s*(?:\*\*\*)?(Passed|Failed|Timeout|Exception|Not Run)",
    re.IGNORECASE,
)

FAILED = {"failed", "timeout", "exception"}


def parse_attempts(log_text: str) -> dict[str, list[str]]:
    attempts: dict[str, list[str]] = defaultdict(list)
    for match in TEST_LINE.finditer(log_text):
        name, outcome = match.group(1).rstrip(), match.group(2).lower()
        attempts[name].append("failed" if outcome in FAILED else outcome.replace(" ", "").lower())
    return dict(attempts)


def _testcase_name(case: ET.Element) -> str:
    name = case.get("name") or ""
    classname = case.get("classname") or ""
    if classname and name.startswith(classname):
        return name
    if classname and name:
        return f"{classname}: {name}"
    return name or classname


def annotate(junit_xml: str, attempts: dict[str, list[str]]) -> str:
    root = ET.fromstring(junit_xml)
    suites = root.findall(".//testcase")
    if root.tag == "testcase":
        suites = [root]
    for case in suites:
        key = _testcase_name(case)
        history = attempts.get(key) or attempts.get(case.get("name") or "")
        if not history or len(history) < 2:
            continue
        last = history[-1]
        prior = history[:-1]
        failed_prior = [h for h in prior if h == "failed"]
        if not failed_prior:
            continue
        if last == "passed":
            for i, _ in enumerate(failed_prior, start=1):
                child = ET.Element(
                    "flakyFailure",
                    {"type": "CTestFailure", "message": f"ctest attempt {i} failed"},
                )
                case.append(child)
        elif last == "failed":
            for i, _ in enumerate(failed_prior, start=1):
                child = ET.Element(
                    "rerunFailure",
                    {"type": "CTestFailure", "message": f"ctest attempt {i} failed"},
                )
                case.append(child)
    return ET.tostring(root, encoding="unicode")


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--junit", required=True, type=Path)
    parser.add_argument("--log", required=True, type=Path)
    args = parser.parse_args(argv)
    if not args.junit.is_file() or not args.log.is_file():
        print(f"annotate_ctest_junit_retries: missing {args.junit} or {args.log}", file=sys.stderr)
        return 0
    xml_text = args.junit.read_text(encoding="utf-8", errors="replace")
    log_text = args.log.read_text(encoding="utf-8", errors="replace")
    updated = annotate(xml_text, parse_attempts(log_text))
    if not updated.startswith("<?xml"):
        updated = '<?xml version="1.0" encoding="UTF-8"?>\n' + updated
    args.junit.write_text(updated, encoding="utf-8")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
