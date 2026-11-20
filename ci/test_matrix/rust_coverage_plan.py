#!/usr/bin/env python3
import json
import os
import sys
from pathlib import Path

import generate_matrix


def artifact_name(row, nightly=False):
    prefix = "coverage-nightly-rust-" if nightly else "coverage-rust-"
    return prefix + row["name"] + "-" + row["cloud_provider"]


def coverage_plan(active_rows, all_rows, nightly=False):
    coverage_rows = [row for row in all_rows if row.get("coverage")]
    baseline = (
        coverage_rows if nightly else generate_matrix.filter_active(coverage_rows, "pr")
    )
    active_coverage = [row for row in active_rows if row.get("coverage")]
    active = {artifact_name(row): row for row in active_coverage}
    if len(active) != len(active_coverage):
        raise ValueError("Rust coverage matrix cells must be unique")
    eligible = bool(baseline) and all(
        active.get(artifact_name(row)) == row for row in baseline
    )
    artifacts = sorted(artifact_name(row, nightly) for row in baseline)
    if len(set(artifacts)) != len(artifacts):
        raise ValueError("Rust coverage artifact names must be unique")
    return eligible, artifacts


def main():
    active = json.loads(os.environ["ACTIVE_MATRIX"])
    rows = generate_matrix.generate(generate_matrix.MODELS_DIR / "core.py", "core")
    eligible, artifacts = coverage_plan(active, rows)
    if eligible:
        Path(sys.argv[1]).write_text(json.dumps({"artifacts": artifacts}) + "\n")
    print("eligible=" + str(eligible).lower())
    nightly_eligible, nightly_artifacts = coverage_plan(active, rows, nightly=True)
    nightly_eligible = (
        nightly_eligible
        and os.environ.get("GITHUB_EVENT_NAME") in ("schedule", "workflow_dispatch")
        and os.environ.get("GITHUB_REF") == "refs/heads/main"
    )
    if nightly_eligible:
        Path(sys.argv[2]).write_text(
            json.dumps({"artifacts": nightly_artifacts}) + "\n"
        )
    print("nightly-eligible=" + str(nightly_eligible).lower())


if __name__ == "__main__":
    main()
