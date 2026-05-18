#!/usr/bin/env python3
"""
generate_matrix.py -- pairwise CI matrix generator.

Reads Python model files from ci/test_matrix/models/ and emits one JSON
matrix per driver for GitHub Actions consumption.

Usage:
  python ci/test_matrix/generate_matrix.py --driver odbc
  python ci/test_matrix/generate_matrix.py --driver python
  python ci/test_matrix/generate_matrix.py --driver core
  python ci/test_matrix/generate_matrix.py --all
  python ci/test_matrix/generate_matrix.py --driver odbc \\
      --event pull_request --emit-active                         # for use in CI workflows

Exit codes:
  0  success
  1  model constraint unsatisfiable / configuration error
"""

from __future__ import annotations

import argparse
import importlib.util
import itertools
import json
import sys
from pathlib import Path
from typing import Any

# ---------------------------------------------------------------------------
# Paths
# ---------------------------------------------------------------------------

REPO_ROOT = Path(__file__).parent.parent.parent
MODELS_DIR = REPO_ROOT / "ci" / "test_matrix" / "models"
GENERATED_DIR = REPO_ROOT / "ci" / "test_matrix" / "generated"

# ---------------------------------------------------------------------------
# Trigger level ordering (lower index = lower coverage, subset of next)
# ---------------------------------------------------------------------------

TRIGGER_LEVELS = ["pr", "merge", "nightly"]


# ---------------------------------------------------------------------------
# Mapping tables (per-driver lookup tables for runner labels, driver libs,
# wheel artifacts, cargo flags, etc.) live under ci/test_matrix/mappings/:
#   - mappings/shared.py  → GHA_RUNNER (used by every driver)
#   - mappings/odbc.py    → ODBC_PLATFORM (one row per (OS, Arch) lane)
#   - mappings/python.py  → PYTHON_PLATFORM, SDIST_PY
#   - mappings/core.py    → CORE_PLATFORM
# ---------------------------------------------------------------------------

# Make `from mappings import …` resolvable both when this file is run as a
# script (sys.path[0] = ci/test_matrix) and when imported via the test runner
# (sys.path[0] = repo root).
sys.path.insert(0, str(Path(__file__).parent))

from mappings import (  # noqa: E402
    GHA_RUNNER,
    ODBC_PLATFORM,
    PYTHON_PLATFORM,
    SDIST_PY,
    CORE_PLATFORM,
)


# ---------------------------------------------------------------------------
# Pairwise cover (greedy, no external dependencies)
# ---------------------------------------------------------------------------

def pairwise(
    param_values: list[list[str]],
    valid_predicate=None,
) -> list[tuple]:
    """
    Greedy pairwise cover.

    Iteratively picks the combination that covers the most not-yet-covered
    (parameter-i, value-i, parameter-j, value-j) pairs until all pairs are covered.

    `valid_predicate(combo_tuple) -> bool` filters candidate combos. When
    provided, only combos that pass the predicate are picked, AND the set of
    pairs to cover is restricted to pairs reachable via at least one valid
    combo. This prevents the solver from "covering" a pair via a combo that
    later gets dropped during routing (e.g. a Python wheel-less platform/py
    pair) — without that filter the picked combo would silently produce no
    matrix row, leaving a coverage hole at merge level.
    """
    indices = range(len(param_values))

    if valid_predicate is None:
        candidates = list(itertools.product(*param_values))
    else:
        candidates = [c for c in itertools.product(*param_values) if valid_predicate(c)]

    if not candidates:
        return []

    uncovered: set[tuple] = set()
    for c in candidates:
        for i, j in itertools.combinations(indices, 2):
            uncovered.add((i, c[i], j, c[j]))

    result: list[tuple] = []
    while uncovered:
        best: tuple | None = None
        best_score = -1
        for combo in candidates:
            score = sum(
                1 for i, j in itertools.combinations(indices, 2)
                if (i, combo[i], j, combo[j]) in uncovered
            )
            if score > best_score:
                best, best_score = combo, score
        assert best is not None
        result.append(best)
        uncovered -= {
            (i, best[i], j, best[j])
            for i, j in itertools.combinations(indices, 2)
        }
    return result


# ---------------------------------------------------------------------------
# Python-module model loader
# ---------------------------------------------------------------------------
# Each driver's coverage model is a Python module at `models/<driver>.py`
# exposing PARAMS / CONSTRAINTS / PR_CELLS / JSON_CELLS. See
# `models/__init__.py` for the schema.

_TRIGGER_LEVELS_FOR_JSON = ("pr", "merge", "nightly")


def load_model(
    path: Path,
) -> tuple[
    dict[str, list[str]],
    list,
    list,
    list[dict[str, str]],
    dict[str, list[dict[str, str]]],
]:
    """
    Load a model module and validate its shape.

    Returns (params, constraints, merge_valid, pr_cells, json_cells).
    Constraints and merge_valid are lists of callables `(combo) -> bool`
    returning True when the combo is valid. CONSTRAINTS gates *all* trigger
    levels (full cartesian product); MERGE_VALID additionally gates the
    pairwise pass — a combo blocked by MERGE_VALID still appears at nightly.
    Raises ValueError on any malformed input.
    """
    spec = importlib.util.spec_from_file_location(f"_model_{path.stem}", path)
    if spec is None or spec.loader is None:
        raise ValueError(f"could not load model module: {path}")
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)

    raw_params = getattr(module, "PARAMS", None)
    raw_constraints = getattr(module, "CONSTRAINTS", [])
    raw_merge_valid = getattr(module, "MERGE_VALID", [])
    raw_pr = getattr(module, "PR_CELLS", [])
    raw_json = getattr(module, "JSON_CELLS", {})

    if not isinstance(raw_params, dict) or not raw_params:
        raise ValueError(f"{path}: PARAMS must be a non-empty dict")

    # Defensive copy: insertion order survives (Python 3.7+) and downstream
    # mutation can't poison the loaded model.
    params = {name: list(values) for name, values in raw_params.items()}

    if not isinstance(raw_constraints, list):
        raise ValueError(f"{path}: CONSTRAINTS must be a list of callables")
    for i, c in enumerate(raw_constraints):
        if not callable(c):
            raise ValueError(
                f"{path}: CONSTRAINTS[{i}] must be callable; got {type(c).__name__}"
            )
    constraints = list(raw_constraints)

    if not isinstance(raw_merge_valid, list):
        raise ValueError(f"{path}: MERGE_VALID must be a list of callables")
    for i, c in enumerate(raw_merge_valid):
        if not callable(c):
            raise ValueError(
                f"{path}: MERGE_VALID[{i}] must be callable; got {type(c).__name__}"
            )
    merge_valid = list(raw_merge_valid)

    pr_cells = [_normalize_cell(c, params, path, "PR_CELLS") for c in raw_pr]

    json_cells: dict[str, list[dict[str, str]]] = {lvl: [] for lvl in _TRIGGER_LEVELS_FOR_JSON}
    if not isinstance(raw_json, dict):
        raise ValueError(f"{path}: JSON_CELLS must be a dict")
    for level, cells in raw_json.items():
        if level not in json_cells:
            raise ValueError(
                f"{path}: JSON_CELLS has unknown trigger level {level!r}; "
                f"expected one of {sorted(json_cells)}"
            )
        for c in cells:
            json_cells[level].append(
                _normalize_cell(c, params, path, f"JSON_CELLS[{level!r}]")
            )

    return params, constraints, merge_valid, pr_cells, json_cells


def _normalize_cell(
    raw: dict, params: dict[str, list[str]], path: Path, where: str
) -> dict[str, str]:
    """Validate an explicit-cell dict has exactly the declared parameter keys."""
    if not isinstance(raw, dict):
        raise ValueError(f"{path}: {where} entries must be dicts; got {raw!r}")
    missing = [p for p in params if p not in raw]
    extra = [k for k in raw if k not in params]
    if missing or extra:
        raise ValueError(
            f"{path}: {where} cell {raw!r} is malformed; "
            f"missing={missing} extra={extra}; expected keys={list(params)}"
        )
    return {p: raw[p] for p in params}


def apply_constraints(combo: dict[str, str], constraints: list) -> bool:
    """Return True if the combination satisfies every constraint predicate."""
    return all(predicate(combo) for predicate in constraints)


# ---------------------------------------------------------------------------
# Mapping-table validation
# ---------------------------------------------------------------------------

def validate_mappings(driver: str, all_combos: list[dict[str, str]]) -> None:
    """
    Fail loudly if a constraint-satisfying (OS, Arch) is missing from the
    runner / mapping tables. Without this, missing entries silently drop cells.
    """
    seen_pairs = {(c["OS"], c["Arch"]) for c in all_combos}
    for pair in sorted(seen_pairs):
        if pair not in GHA_RUNNER:
            raise RuntimeError(
                f"({pair[0]}, {pair[1]}) is allowed by the model but is not present "
                f"in GHA_RUNNER — add it to mappings/shared.py or constrain the model "
                f"so this combination is impossible."
            )
        # ODBC_PLATFORM is consumed by `_build_gha_row` only on the ODBC branch
        # (driver_lib + driver_artifact). Python rows don't read it today; the
        # core driver builds sf_core directly without producing or loading
        # sfodbc. Restrict the check to driver=="odbc" so that opening up an
        # OS/Arch lane in python.py doesn't force a placeholder row in
        # ODBC_PLATFORM.
        if driver == "odbc" and pair not in ODBC_PLATFORM:
            raise RuntimeError(
                f"({pair[0]}, {pair[1]}) is allowed by the ODBC model but missing "
                f"from ODBC_PLATFORM in mappings/odbc.py — add a row there with at "
                f"least a 'driver_lib' entry, or constrain the model."
            )
        if driver == "odbc" and "driver_artifact" not in ODBC_PLATFORM[pair]:
            raise RuntimeError(
                f"({pair[0]}, {pair[1]}) is allowed by the ODBC model but "
                f"ODBC_PLATFORM has no 'driver_artifact' for it — add a matching "
                f"entry to build_odbc_driver in .github/workflows/test-odbc.yml first, "
                f"then set 'driver_artifact' on the row in mappings/odbc.py."
            )
        if driver == "odbc" and "driver_artifact" in ODBC_PLATFORM[pair] and "cache_key" not in ODBC_PLATFORM[pair]:
            raise RuntimeError(
                f"({pair[0]}, {pair[1]}) is in ODBC_PLATFORM with a driver_artifact "
                f"but missing 'cache_key'. This field is required by "
                f"--emit-build-matrix to generate the build_odbc_driver job's "
                f"include array — set it to the cargo-cache shared-key value "
                f"used by that lane (e.g. 'odbc', 'odbc-x64', 'odbc-arm64ec')."
            )
        # PYTHON_PLATFORM is required on every Python lane: _build_gha_row reads
        # wheel_artifact + wheels from it. Missing rows silently drop py3.11+ cells
        # (and emit py3.10 as sdist), so fail loud here instead.
        if driver == "python" and pair not in PYTHON_PLATFORM:
            raise RuntimeError(
                f"({pair[0]}, {pair[1]}) is allowed by the Python model but missing "
                f"from PYTHON_PLATFORM in mappings/python.py — add a row there with "
                f"'cibw_key' + 'wheel_artifact' + 'wheels', or constrain the model."
            )
        if driver == "python" and "cibw_key" not in PYTHON_PLATFORM[pair]:
            raise RuntimeError(
                f"({pair[0]}, {pair[1]}) is in PYTHON_PLATFORM but missing 'cibw_key'. "
                f"This field is required for --emit-build-targets to translate "
                f"(OS, Arch) to the platform key consumed by "
                f"_build-python-wheels.yml. Set it to one of "
                f"linux_x86, linux_aarch, macos_x86, macos_arm, windows_x86, "
                f"or windows_arm — see that workflow's inline PLATFORMS dict."
            )
        if driver == "core" and pair not in CORE_PLATFORM:
            raise RuntimeError(
                f"({pair[0]}, {pair[1]}) is allowed by the core model but missing "
                f"from CORE_PLATFORM in mappings/core.py — add a row there or "
                f"constrain the model."
            )


# ---------------------------------------------------------------------------
# Reference variants (json result_format)
# ---------------------------------------------------------------------------
#
# JSON variants live in each model file's `JSON_CELLS` dict, keyed by
# trigger level ("pr"/"merge"/"nightly"). Each row is duplicated after
# pairwise generation with result_format="json" appended; the dict key
# selects the trigger level. Core has no JSON axis (sf_core's
# result-format coverage is exercised by ODBC and Python).


# ---------------------------------------------------------------------------
# Row construction
# ---------------------------------------------------------------------------

def _make_name(combo: dict[str, str], result_format: str | None = None) -> str:
    parts = [combo["OS"], combo["Arch"], combo["Cloud"]]
    py = combo.get("PyVersion")
    hatch_env = combo.get("HatchEnv")
    if py:
        parts.append(f"py{py}")
    if hatch_env and hatch_env != "test":
        parts.append(hatch_env)
    if result_format:
        parts.append(result_format)
    return "-".join(parts)


def _build_gha_row(
    combo: dict[str, str],
    trigger: str,
    is_python: bool,
    result_format: str | None = None,
) -> dict[str, Any] | None:
    """Build a GHA row dict. Returns None if the cell should be skipped."""
    os_, arch, cloud = combo["OS"], combo["Arch"], combo["Cloud"]
    py = combo.get("PyVersion")
    hatch_env = combo.get("HatchEnv")

    runner = GHA_RUNNER.get((os_, arch))
    if runner is None:
        return None

    row: dict[str, Any] = {
        "name": _make_name(combo, result_format),
        "os": runner,  # GitHub Actions runner label (used in runs-on and `matrix.os == 'X'` checks)
        "cloud_provider": cloud,
        "trigger_level": trigger,
    }
    if py:
        row["py"] = py
    if result_format:
        row["result_format"] = result_format

    if is_python:
        # Wheel vs sdist routing:
        #   - py in SDIST_PY (3.10): always sdist; do not set wheel_artifact.
        #   - py in PYTHON_PLATFORM[(os, arch)]["wheels"]: wheel exists.
        #   - else: no wheel built and not sdist-supported -> skip.
        platform = PYTHON_PLATFORM.get((os_, arch))
        if py in SDIST_PY:
            pass  # workflow takes the sdist path when wheel_artifact is unset
        elif py and platform is not None and py in platform["wheels"]:
            row["wheel_artifact"] = platform["wheel_artifact"]
        else:
            return None
        row["hatch_env"] = hatch_env or "test"
    else:
        platform = ODBC_PLATFORM.get((os_, arch))
        if platform is None or "driver_artifact" not in platform:
            return None
        row["driver_artifact"] = platform["driver_artifact"]
        row["driver_lib"] = platform["driver_lib"]
        # Windows-x86 needs the explicit msvc_arch and vcpkg_triplet that the
        # workflow's defaults (amd64/x64-windows) would otherwise hide.
        for key in ("msvc_arch", "vcpkg_triplet"):
            if key in platform:
                row[key] = platform[key]

    return row


def _build_core_row(combo: dict[str, str], trigger: str) -> dict[str, Any] | None:
    """
    Build a row for the consolidated rust-core test job.

    sf_core has no Cloud axis (tests use a single E2E_TLS_SERVER and don't
    parameterize over cloud providers). Returns None for any (OS, Arch) pair
    that GHA doesn't run today.
    """
    os_, arch = combo["OS"], combo["Arch"]
    runner = GHA_RUNNER.get((os_, arch))
    if runner is None:
        return None

    # Display name encodes only what's visible in the GHA UI: platform + arch,
    # plus a "-nonfips" suffix on Windows ARM64 where the cell intentionally
    # disables FIPS (mirrors the old test_windows_arm64_nonfips job name).
    name_parts = [os_, arch]
    if (os_, arch) == ("windows", "arm"):
        name_parts.append("nonfips")
    name = "-".join(name_parts)

    platform = CORE_PLATFORM[(os_, arch)]
    row: dict[str, Any] = {
        "name": name,
        "os": runner,  # GitHub Actions runner label
        "trigger_level": trigger,
        "cargo_flags": platform["cargo_flags"],
        "coverage": platform["coverage"],
        "cache_key": platform["cache_key"],
    }
    if "cargo_target" in platform:
        row["cargo_target"] = platform["cargo_target"]
    if "msvc_arch" in platform:
        row["msvc_arch"] = platform["msvc_arch"]
    return row


# ---------------------------------------------------------------------------
# Core generator
# ---------------------------------------------------------------------------

def generate(model_path: Path, driver: str) -> list[dict]:
    """
    Run pairwise generation for a model file.

    Returns the list of GitHub Actions matrix rows.
    """
    params, constraints, merge_valid, pr_cells, json_cells = load_model(model_path)
    param_names = list(params.keys())
    param_values = list(params.values())

    # All valid combinations (full cartesian product filtered by constraints)
    all_combos: list[dict[str, str]] = [
        combo for combo in (
            dict(zip(param_names, values))
            for values in itertools.product(*param_values)
        )
        if apply_constraints(combo, constraints)
    ]

    validate_mappings(driver, all_combos)

    is_python = "PyVersion" in params or "HatchEnv" in params
    is_core = driver == "core"

    # Routing-aware pairwise cover: only consider combos that survive
    # _build_*_row routing. Without this, the solver may pick a combo (e.g.
    # windows-arm × py3.13) that gets silently dropped at row-build time
    # because the wheel doesn't exist, leaving the (windows, arm) value pair
    # technically "covered" in the abstract pairwise sense but with zero
    # actual matrix rows at merge level.
    #
    # MERGE_VALID adds an extra block-list that only applies to the pairwise
    # pass: combos rejected here still flow through to nightly, but never
    # land in pr/merge. Use it to keep expensive lanes (e.g. limited macOS
    # runners) out of the merge queue without losing nightly coverage.
    def _routing_valid(combo_tuple: tuple) -> bool:
        combo = dict(zip(param_names, combo_tuple))
        if not apply_constraints(combo, constraints):
            return False
        if not apply_constraints(combo, merge_valid):
            return False
        # `trigger` doesn't influence the build/skip decision, so any
        # placeholder value works. Pass "merge" for clarity.
        if is_core:
            return _build_core_row(combo, "merge") is not None
        return _build_gha_row(combo, "merge", is_python) is not None

    pairwise_keys: set[tuple] = {
        tuple(dict(zip(param_names, pair)).values())
        for pair in pairwise(param_values, valid_predicate=_routing_valid)
    }

    # Explicit PR cells from [pr] section; fall back to first 3 pairwise rows
    # if the model has no [pr] section.
    if pr_cells:
        pr_keys: set[tuple] = {tuple(c.values()) for c in pr_cells}
        # Warn about PR cells that violate constraints and will produce no output row.
        valid_keys = {tuple(c.values()) for c in all_combos}
        for cell in pr_cells:
            if tuple(cell.values()) not in valid_keys:
                print(
                    f"WARNING: [pr] cell {cell} violates constraints — no row will be emitted",
                    file=sys.stderr,
                )
    else:
        pr_keys = set()
        for key in list(pairwise_keys)[:3]:
            pr_keys.add(key)

    # Assign trigger levels:
    #   pr      - cells listed in [pr] (explicit) or first 3 pairwise rows (fallback)
    #   merge   - remaining pairwise rows; pr cells are also included at merge level
    #             (cumulative filter: merge runs pr+merge)
    #   nightly - everything else (full matrix)
    for combo in all_combos:
        key = tuple(combo.values())
        if key in pr_keys:
            combo["_trigger"] = "pr"
        elif key in pairwise_keys:
            combo["_trigger"] = "merge"
        else:
            combo["_trigger"] = "nightly"

    combos = sorted(all_combos, key=lambda c: TRIGGER_LEVELS.index(c["_trigger"]))

    gha_rows: list[dict] = []

    for combo in combos:
        trigger = combo.pop("_trigger")
        if is_core:
            row = _build_core_row(combo, trigger)
        else:
            row = _build_gha_row(combo, trigger, is_python)
        if row is not None:
            gha_rows.append(row)

    # Append result_format=json reference variants. Core has no JSON variants —
    # sf_core's result-format coverage is exercised by ODBC and Python.
    # Variants are declared in the model file under [json_pr] / [json_merge] /
    # [json_nightly] sections; each row is duplicated with result_format="json".
    if is_core:
        return gha_rows

    valid_keys = {tuple(combo.get(p) for p in param_names) for combo in all_combos}
    for trigger, cells in json_cells.items():
        for cell in cells:
            key = tuple(cell[p] for p in param_names)
            if key not in valid_keys:
                print(
                    f"WARNING: [json_{trigger}] cell {cell} violates constraints "
                    f"or is outside the model's parameter values — no row will be emitted",
                    file=sys.stderr,
                )
                continue
            row = _build_gha_row(dict(cell), trigger, is_python, result_format="json")
            if row is not None:
                gha_rows.append(row)

    return gha_rows


# ---------------------------------------------------------------------------
# Trigger-level filtering (for runtime use in CI workflows)
# ---------------------------------------------------------------------------

EVENT_TO_LEVEL = {
    "pull_request": "pr",
    "pull_request_target": "pr",
    "push": "merge",
    "merge_group": "merge",
    "schedule": "nightly",
    "workflow_dispatch": "nightly",
}


# PR labels that force a higher trigger level than the event would produce.
# When multiple are present the highest-scope label wins.
LABEL_TO_LEVEL = {
    "ci:scope-merge":   "merge",
    "ci:scope-nightly": "nightly",
}


def level_for_event(event: str | None) -> str:
    """
    Map a GitHub Actions event name to a trigger level.
    Falls back to 'pr' for unknown events.
    """
    return EVENT_TO_LEVEL.get(event or "", "pr")


def level_for_event_and_labels(event: str | None, labels: list[str] | None) -> str:
    """
    Resolve the active trigger level. PR labels override the event mapping;
    when multiple scope-up labels are present, the highest level wins.
    Labels can only upgrade scope — they never downgrade an event's level.
    Unknown labels are ignored.
    """
    base = level_for_event(event)
    if not labels:
        return base
    requested = [LABEL_TO_LEVEL[l] for l in labels if l in LABEL_TO_LEVEL]
    if not requested:
        return base
    candidates = [base] + requested
    return max(candidates, key=TRIGGER_LEVELS.index)


def filter_active(rows: list[dict], level: str) -> list[dict]:
    """Return rows whose trigger_level is at or below `level` (cumulative)."""
    if level not in TRIGGER_LEVELS:
        raise ValueError(f"unknown trigger level: {level!r}; expected one of {TRIGGER_LEVELS}")
    cap = TRIGGER_LEVELS.index(level)
    return [r for r in rows if TRIGGER_LEVELS.index(r["trigger_level"]) <= cap]


# ---------------------------------------------------------------------------
# File I/O helpers
# ---------------------------------------------------------------------------

def write_json(path: Path, data: list[dict]) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(data, indent=2) + "\n")


# ---------------------------------------------------------------------------
# CLI entry points
# ---------------------------------------------------------------------------

def run_driver(driver: str) -> bool:
    """Generate matrices for one driver. Returns True if OK."""
    model_path = MODELS_DIR / f"{driver}.py"
    if not model_path.exists():
        print(f"ERROR: model file not found: {model_path}", file=sys.stderr)
        return False

    gha_rows = generate(model_path, driver)

    gha_path = GENERATED_DIR / f"{driver}-gha.json"
    write_json(gha_path, gha_rows)
    print(f"Wrote {gha_path} ({len(gha_rows)} rows)")
    return True


def emit_active(driver: str, event: str | None, labels: list[str] | None = None) -> None:
    """
    Print `matrix=<json>` for the rows active at the level implied by `event`,
    optionally upgraded by scope-up PR labels (see LABEL_TO_LEVEL). Suitable
    for appending to $GITHUB_OUTPUT inside a workflow step.
    """
    model_path = MODELS_DIR / f"{driver}.py"
    gha_rows = generate(model_path, driver)
    level = level_for_event_and_labels(event, labels)
    active = filter_active(gha_rows, level)
    print(f"matrix={json.dumps(active)}")


def build_targets(driver: str, event: str | None, labels: list[str] | None = None) -> dict[str, list[str]]:
    """
    Return the wheel-build targets for `driver` at the trigger level implied
    by `event`, in the JSON shape consumed by _build-python-wheels.yml's
    `targets:` input. Currently only supported for the python driver.

    Output shape:
        {
            "linux_x86":  ["3.13"],
            "macos_arm":  ["3.12"],
            ...
        }

    A platform key appears only if at least one active test row references
    a wheel from it; py versions within a platform are deduplicated and
    sorted. Sdist-only py versions (SDIST_PY) are naturally excluded
    because their rows carry no `wheel_artifact`.
    """
    if driver != "python":
        raise ValueError(
            f"--emit-build-targets is only supported for the python driver; got {driver!r}"
        )
    model_path = MODELS_DIR / f"{driver}.py"
    gha_rows = generate(model_path, driver)
    level = level_for_event_and_labels(event, labels)
    active = filter_active(gha_rows, level)

    # Reverse-lookup wheel_artifact -> (os, arch) so we can fetch cibw_key.
    artifact_to_pair: dict[str, tuple[str, str]] = {
        meta["wheel_artifact"]: pair for pair, meta in PYTHON_PLATFORM.items()
    }

    targets: dict[str, set[str]] = {}
    for row in active:
        artifact = row.get("wheel_artifact")
        if not artifact:
            # py3.10 sdist rows and any other no-wheel rows are skipped: the
            # build workflow doesn't need to produce a wheel for them.
            continue
        pair = artifact_to_pair[artifact]
        cibw_key = PYTHON_PLATFORM[pair]["cibw_key"]
        targets.setdefault(cibw_key, set()).add(row["py"])

    return {key: sorted(versions) for key, versions in sorted(targets.items())}


def emit_build_targets(driver: str, event: str | None, labels: list[str] | None = None) -> None:
    """
    Print `targets=<json>` for the wheel-build targets active at the level
    implied by `event`, optionally upgraded by scope-up PR labels (see
    LABEL_TO_LEVEL). Suitable for appending to $GITHUB_OUTPUT inside a
    workflow step. Currently only supported for the python driver.
    """
    print(f"targets={json.dumps(build_targets(driver, event, labels))}")


def build_matrix(driver: str, event: str | None, labels: list[str] | None = None) -> list[dict]:
    """
    Return the GHA `include:` array for the driver-build job at the trigger
    level implied by `event`. Currently only supported for the odbc driver.

    Each entry carries the build-relevant fields the build_odbc_driver job
    in test-odbc.yml needs:
        {
            "name":            <driver_artifact value, used as both display
                                name and uploaded artifact suffix>,
            "os":              <GHA runner label>,
            "driver_lib":      <library file name, e.g. libsfodbc.so>,
            "driver_artifact": <same as name; kept for symmetry with test rows>,
            "cache_key":       <actions/cargo-cache shared-key value>,
            "cargo_extra":     <optional extra cargo flags; absent on lanes
                                that don't set them>,
            "cargo_target":    <optional cross-compile target>,
            "msvc_arch":       <optional, Windows non-x64 only>,
            "vcpkg_triplet":   <optional, Windows only>,
        }

    A platform appears at most once per call: lanes are deduplicated by
    (OS, Arch). The same lane is included regardless of how many active
    test rows reference its driver — one build per lane covers all rows.
    """
    if driver != "odbc":
        raise ValueError(
            f"--emit-build-matrix is only supported for the odbc driver; got {driver!r}"
        )
    model_path = MODELS_DIR / f"{driver}.py"
    gha_rows = generate(model_path, driver)
    level = level_for_event_and_labels(event, labels)
    active = filter_active(gha_rows, level)

    # Reverse-lookup driver_artifact -> (os, arch) so we can fetch full
    # ODBC_PLATFORM metadata. Only pairs that have a driver_artifact set
    # are eligible — Linux ARM lanes (driver_artifact absent) are skipped.
    artifact_to_pair: dict[str, tuple[str, str]] = {
        meta["driver_artifact"]: pair
        for pair, meta in ODBC_PLATFORM.items()
        if "driver_artifact" in meta
    }

    seen_pairs: set[tuple[str, str]] = set()
    matrix: list[dict] = []
    for row in active:
        artifact = row.get("driver_artifact")
        if not artifact:
            # Unbuilt lanes (Linux ARM) — no build job needed.
            continue
        pair = artifact_to_pair[artifact]
        if pair in seen_pairs:
            continue
        seen_pairs.add(pair)
        meta = ODBC_PLATFORM[pair]
        entry: dict = {
            "name":            meta["driver_artifact"],
            "os":              GHA_RUNNER[pair],
            "driver_lib":      meta["driver_lib"],
            "driver_artifact": meta["driver_artifact"],
            "cache_key":       meta["cache_key"],
        }
        for optional_key in ("cargo_extra", "cargo_target", "msvc_arch", "vcpkg_triplet"):
            if optional_key in meta:
                entry[optional_key] = meta[optional_key]
        matrix.append(entry)

    # Stable order: by name. Reproducible output regardless of which test row
    # happened to introduce a lane first.
    matrix.sort(key=lambda r: r["name"])
    return matrix


def emit_build_matrix(driver: str, event: str | None, labels: list[str] | None = None) -> None:
    """
    Print `matrix=<json>` for the driver-build matrix active at the level
    implied by `event`, optionally upgraded by scope-up PR labels (see
    LABEL_TO_LEVEL). Suitable for appending to $GITHUB_OUTPUT inside a
    workflow step. Currently only supported for the odbc driver.
    """
    print(f"matrix={json.dumps(build_matrix(driver, event, labels))}")


def main() -> None:
    parser = argparse.ArgumentParser(description="Pairwise CI matrix generator")
    group = parser.add_mutually_exclusive_group(required=True)
    group.add_argument("--driver", choices=["odbc", "python", "core"], help="Generate for a single driver")
    group.add_argument("--all", action="store_true", help="Regenerate all drivers")
    parser.add_argument(
        "--event",
        help="GitHub Actions event name (e.g. pull_request, merge_group, schedule). "
             "Used with --emit-active to pick the trigger level.",
    )
    parser.add_argument(
        "--labels",
        default="",
        help="Comma-separated PR label names. Labels in LABEL_TO_LEVEL "
             "(ci:scope-merge, ci:scope-nightly) override --event when "
             "--emit-active is set; highest-scope label wins.",
    )
    parser.add_argument(
        "--emit-active",
        action="store_true",
        help="Print 'matrix=<json>' for the active level (requires --driver and --event). "
             "Used inside CI workflow steps to feed strategy.matrix.include.",
    )
    parser.add_argument(
        "--emit-build-targets",
        action="store_true",
        help="Print 'targets=<json>' for the wheel-build targets active at the trigger "
             "level implied by --event (requires --driver and --event). Currently only "
             "supported for the python driver — feeds _build-python-wheels.yml's "
             "`targets:` input.",
    )
    parser.add_argument(
        "--emit-build-matrix",
        action="store_true",
        help="Print 'matrix=<json>' for the driver-build matrix active at the trigger "
             "level implied by --event (requires --driver and --event). Currently only "
             "supported for the odbc driver — feeds build_odbc_driver's matrix.include "
             "in test-odbc.yml.",
    )
    args = parser.parse_args()

    if args.emit_active:
        if args.all or not args.driver:
            parser.error("--emit-active requires --driver and is incompatible with --all")
        if args.emit_build_targets or args.emit_build_matrix:
            parser.error("--emit-active is mutually exclusive with --emit-build-targets and --emit-build-matrix")
        labels = [l.strip() for l in args.labels.split(",") if l.strip()]
        emit_active(args.driver, args.event, labels)
        return

    if args.emit_build_targets:
        if args.all or not args.driver:
            parser.error("--emit-build-targets requires --driver and is incompatible with --all")
        if args.emit_build_matrix:
            parser.error("--emit-build-targets and --emit-build-matrix are mutually exclusive")
        labels = [l.strip() for l in args.labels.split(",") if l.strip()]
        emit_build_targets(args.driver, args.event, labels)
        return

    if args.emit_build_matrix:
        if args.all or not args.driver:
            parser.error("--emit-build-matrix requires --driver and is incompatible with --all")
        labels = [l.strip() for l in args.labels.split(",") if l.strip()]
        emit_build_matrix(args.driver, args.event, labels)
        return

    drivers = ["odbc", "python", "core"] if args.all else [args.driver]
    ok = True
    for driver in drivers:
        if not run_driver(driver):
            ok = False

    if not ok:
        sys.exit(1)


if __name__ == "__main__":
    main()
