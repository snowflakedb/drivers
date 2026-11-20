#!/usr/bin/env python3
import json
import os
import sys
from pathlib import Path

import generate_matrix


def artifact_name(row, nightly=False):
    parts = [row["os"], row["py"], row["hatch_env"], row["cloud_provider"]]
    if row.get("result_format"):
        parts.append(row["result_format"])
    prefix = "coverage-nightly-python-" if nightly else "coverage-python-"
    return prefix + "-".join(parts)


def coverage_plan(active_rows, all_rows, nightly=False):
    baseline = all_rows if nightly else generate_matrix.filter_active(all_rows, "pr")
    active = {row["name"]: row for row in active_rows}
    eligible = bool(baseline) and all(
        active.get(row["name"]) == row for row in baseline
    )
    artifacts = sorted(artifact_name(row, nightly) for row in baseline)
    if len(set(artifacts)) != len(artifacts):
        raise ValueError("Python coverage artifact names must be unique")
    return eligible, artifacts


def main():
    active = json.loads(os.environ["ACTIVE_MATRIX"])
    rows = generate_matrix.generate(generate_matrix.MODELS_DIR / "python.py", "python")
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
