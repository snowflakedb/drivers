#!/usr/bin/env python3
import argparse
from pathlib import Path
import xml.etree.ElementTree as ET


def format_output(label: str, report_path: str, line_counter: tuple[int, int] | None) -> str:
    header = f"### {label} coverage"
    if line_counter is None:
        return f"{header}\n- Line coverage unavailable.\n"

    missed, covered = line_counter
    total = missed + covered
    if total <= 0:
        return f"{header}\n- Line coverage unavailable (total lines is zero).\n"

    pct = (covered / total) * 100.0
    return (
        f"{header}\n"
        f"- Report: `{report_path}`\n"
        f"- Line coverage: **{pct:.2f}%** ({covered}/{total})\n"
    )


def append_summary(summary_path: str, output: str) -> None:
    if not summary_path:
        return
    with Path(summary_path).open("a", encoding="utf-8") as summary_file:
        summary_file.write(output)


def main() -> None:
    parser = argparse.ArgumentParser(description="Extract overall line coverage from JaCoCo XML.")
    parser.add_argument("--report", required=True, help="Path to JaCoCo XML report")
    parser.add_argument("--label", default="JDBC", help="Label for output")
    parser.add_argument("--summary", default="", help="Optional GitHub step summary path")
    parser.add_argument("--allow-missing", action="store_true", help="Do not fail if report is missing")
    args = parser.parse_args()

    report = Path(args.report)
    if not report.is_file():
        output = f"### {args.label} coverage\n- Coverage report missing: `{args.report}`\n"
        print(output)
        append_summary(args.summary, output)
        if args.allow_missing:
            return
        raise SystemExit(f"[coverage] missing coverage report: {args.report}")

    root = ET.parse(report).getroot()
    line_counter = None
    for counter in root.findall("counter"):
        if counter.get("type") == "LINE":
            line_counter = (
                int(counter.get("missed", "0")),
                int(counter.get("covered", "0")),
            )
            break

    output = format_output(args.label, str(report), line_counter)
    print(output)
    append_summary(args.summary, output)


if __name__ == "__main__":
    main()
