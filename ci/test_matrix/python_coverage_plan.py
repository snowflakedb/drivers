#!/usr/bin/env python3
import json
import os
import sys
from pathlib import Path

import generate_matrix


def artifact_name(row):
    parts = [row["os"], row["py"], row["hatch_env"], row["cloud_provider"]]
    if row.get("result_format"):
        parts.append(row["result_format"])
    return "coverage-python-" + "-".join(parts)


def coverage_plan(active_rows, all_rows):
    baseline = generate_matrix.filter_active(all_rows, "pr")
    active = {row["name"]: row for row in active_rows}
    eligible = bool(baseline) and all(
        active.get(row["name"]) == row for row in baseline
    )
    artifacts = sorted(artifact_name(row) for row in baseline)
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


if __name__ == "__main__":
    main()
