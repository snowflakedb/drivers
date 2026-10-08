#!/usr/bin/env python3
"""Collapse Vitest JUnit retry siblings into Surefire flakyFailure / rerunFailure.

Vitest's built-in junit reporter emits a sibling <testcase> per attempt, and
those siblings are often both passing. Ingest then counts extra attempts
without a rescued (fail-then-pass) signal. Merge same classname+name into one
case: flakyFailure only when a prior sibling actually failed.
"""

from __future__ import annotations

import argparse
import sys
import xml.etree.ElementTree as ET
from collections import OrderedDict
from pathlib import Path


def _status(case: ET.Element) -> str:
    if case.find("failure") is not None or case.find("error") is not None:
        return "failed"
    if case.find("skipped") is not None:
        return "skipped"
    return "passed"


def _failure_child(src: ET.Element, tag: str) -> ET.Element:
    fail = src.find("failure") if src.find("failure") is not None else src.find("error")
    attrib = {}
    text = None
    if fail is not None:
        attrib = {k: v for k, v in fail.attrib.items() if k in {"type", "message"}}
        text = fail.text
    if "type" not in attrib:
        attrib["type"] = "AssertionError"
    if "message" not in attrib:
        attrib["message"] = "vitest retry attempt failed"
    child = ET.Element(tag, attrib)
    if text:
        child.text = text
    return child


def collapse_group(siblings: list[ET.Element]) -> ET.Element:
    if len(siblings) == 1:
        return siblings[0]
    last = siblings[-1]
    prior_failed = [s for s in siblings[:-1] if _status(s) == "failed"]
    last_status = _status(last)
    if last_status == "passed" and prior_failed:
        for src in prior_failed:
            last.append(_failure_child(src, "flakyFailure"))
        return last
    if last_status == "failed" and prior_failed:
        for src in prior_failed:
            last.append(_failure_child(src, "rerunFailure"))
        return last
    return last


def annotate(junit_xml: str) -> str:
    declaration, body = "", junit_xml
    if junit_xml.lstrip().startswith("<?xml"):
        head, _, rest = junit_xml.partition("?>")
        declaration = head + "?>"
        body = rest
    root = ET.fromstring(body)
    suites = root.findall("testsuite")
    if root.tag == "testsuite":
        suites = [root]
    for suite in suites:
        cases = list(suite.findall("testcase"))
        groups: OrderedDict[tuple[str, str], list[ET.Element]] = OrderedDict()
        for case in cases:
            key = (case.get("classname") or "", case.get("name") or "")
            groups.setdefault(key, []).append(case)
            suite.remove(case)
        for siblings in groups.values():
            suite.append(collapse_group(siblings))
    xml = ET.tostring(root, encoding="unicode")
    if declaration:
        return declaration + xml
    return xml


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("junit", nargs="+", type=Path)
    args = parser.parse_args(argv)
    for path in args.junit:
        if not path.is_file():
            print(f"annotate_vitest_junit_retries: skip missing {path}", file=sys.stderr)
            continue
        updated = annotate(path.read_text(encoding="utf-8", errors="replace"))
        if not updated.lstrip().startswith("<?xml"):
            updated = '<?xml version="1.0" encoding="UTF-8"?>\n' + updated
        path.write_text(updated, encoding="utf-8")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
