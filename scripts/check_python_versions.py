#!/usr/bin/env python3
"""Check that every list of supported CPython versions agrees.

``PARAMS["PyVersion"]`` in ``ci/test_matrix/models/python.py`` is the source
of truth. The cloud rotation, the wheel sets per platform, the wheel upload
steps, the Hatch matrices, the package classifiers, ``requires-python``, the
README, and the version literals in ``test-python.yml`` are compared with it.

Run from anywhere:
    python3 scripts/check_python_versions.py
"""

from __future__ import annotations

import argparse
import fnmatch
import importlib.util
import re
import sys
import tomllib
from pathlib import Path
from types import ModuleType

MODELS = "ci/test_matrix/models/python.py"
MAPPINGS = "ci/test_matrix/mappings/python.py"
WHEELS_WORKFLOW = ".github/workflows/_build-python-wheels.yml"
TEST_WORKFLOW = ".github/workflows/test-python.yml"
PYPROJECT = "python/pyproject.toml"
README = "python/README.md"

FULL_MATRIX_ENVS = ("dev", "test", "test-pandas")
SUBSET_MATRIX_ENVS = ("test-native-arrow", "packaging")

_VERSION_RE = re.compile(r"^3\.(?P<minor>\d+)(?P<ft>t?)$")
_UPLOAD_RE = re.compile(
    r"if:\s*contains\(fromJSON\(matrix\.python_versions_json\),\s*'(?P<version>[^']+)'\)"
    r"(?P<body>.*?)(?=\n\s*- name:|\Z)",
    re.S,
)
_ARTIFACT_RE = re.compile(r"name:\s*\$\{\{\s*matrix\.artifact_id\s*\}\}_py(?P<version>\S+)")
_PATH_RE = re.compile(r"path:\s*python/dist/(?P<glob>\S+)")
_CLASSIFIER_RE = re.compile(r"^Programming Language :: Python :: (?P<version>3\.\d+)$")
_REQUIRES_RE = re.compile(r">=\s*(?P<version>3\.\d+)")
_README_MATRIX_RE = re.compile(r"Supports Python matrix:\s*(?P<versions>.+)")
_README_FLOOR_RES = (
    re.compile(r"Python (?P<version>3\.\d+)\+"),
    re.compile(r"Python (?P<version>3\.\d+)\.0 \(or higher\)"),
)
_MATRIX_PY_RE = re.compile(r"matrix\.py\s*==\s*'(?P<version>[^']+)'")


def version_key(version: str) -> tuple[int, bool]:
    match = _VERSION_RE.match(version)
    if match is None:
        return (sys.maxsize, False)
    return (int(match.group("minor")), bool(match.group("ft")))


def wheel_filename(version: str) -> str:
    match = _VERSION_RE.match(version)
    assert match is not None
    tag = f"cp3{match.group('minor')}"
    abi = tag + match.group("ft")
    return f"snowflake_connector_python-1.0.0-{tag}-{abi}-manylinux_2_28_x86_64.whl"


def _format(versions: set[str]) -> str:
    return ", ".join(sorted(versions, key=version_key))


def _compare(where: str, actual: set[str], expected: set[str]) -> list[str]:
    parts = []
    if expected - actual:
        parts.append(f"missing {_format(expected - actual)}")
    if actual - expected:
        parts.append(f"has {_format(actual - expected)}, which PyVersion does not list")
    return [f"{where}: {'; '.join(parts)}"] if parts else []


def _subset(where: str, actual: set[str], supported: set[str]) -> list[str]:
    extra = actual - supported
    return [f"{where}: has {_format(extra)}, which PyVersion does not list"] if extra else []


def _load_module(root: Path, rel: str, name: str) -> ModuleType:
    spec = importlib.util.spec_from_file_location(name, root / rel)
    assert spec is not None and spec.loader is not None
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


def check_models(models: ModuleType, supported: set[str]) -> list[str]:
    errors = [
        f"{MODELS} PARAMS['PyVersion']: {version!r} is not 3.N or 3.Nt"
        for version in sorted(supported)
        if _VERSION_RE.match(version) is None
    ]
    errors += _compare(f"{MODELS} _CLOUD_FOR_PY", set(models._CLOUD_FOR_PY), supported)
    cells = {
        "PR_CELLS": getattr(models, "PR_CELLS", []),
        "MERGE_QUEUE_CELLS": getattr(models, "MERGE_QUEUE_CELLS", []),
    }
    for lane, lane_cells in getattr(models, "JSON_CELLS", {}).items():
        cells[f"JSON_CELLS[{lane!r}]"] = lane_cells
    for label, entries in cells.items():
        errors += _subset(f"{MODELS} {label}", {c["PyVersion"] for c in entries}, supported)
    return errors


def wheel_versions(mappings: ModuleType) -> set[str]:
    versions: set[str] = set()
    for row in mappings.PYTHON_PLATFORM.values():
        versions |= set(row["wheels"])
    return versions


def check_mappings(mappings: ModuleType, supported: set[str]) -> list[str]:
    errors: list[str] = []
    for (os_name, arch), row in mappings.PYTHON_PLATFORM.items():
        errors += _subset(f"{MAPPINGS} wheels for ({os_name}, {arch})", set(row["wheels"]), supported)
    sdist = set(getattr(mappings, "SDIST_PY", set()))
    errors += _compare(f"{MAPPINGS} wheels plus SDIST_PY", wheel_versions(mappings) | sdist, supported)
    return errors


def check_upload_steps(text: str, wheels: set[str], supported: set[str]) -> list[str]:
    errors: list[str] = []
    uploaded: set[str] = set()
    samples = {v: wheel_filename(v) for v in supported if _VERSION_RE.match(v)}
    for match in _UPLOAD_RE.finditer(text):
        version, body = match.group("version"), match.group("body")
        where = f"{WHEELS_WORKFLOW} upload step for {version}"
        if version in uploaded:
            errors.append(f"{where}: appears more than once")
        uploaded.add(version)
        artifact = _ARTIFACT_RE.search(body)
        if artifact is None or artifact.group("version") != version:
            errors.append(f"{where}: artifact name must be ${{{{ matrix.artifact_id }}}}_py{version}")
        path = _PATH_RE.search(body)
        if path is None:
            errors.append(f"{where}: has no python/dist/ path")
            continue
        if version not in samples:
            continue
        matched = {v for v, name in samples.items() if fnmatch.fnmatchcase(name, path.group("glob"))}
        if version not in matched:
            errors.append(f"{where}: {path.group('glob')} does not match {samples[version]}")
        if matched - {version}:
            errors.append(
                f"{where}: {path.group('glob')} also matches the {_format(matched - {version})} wheel"
            )
    errors += _compare(f"{WHEELS_WORKFLOW} upload steps", uploaded, wheels)
    return errors


def check_pyproject(pyproject: dict, supported: set[str]) -> list[str]:
    errors: list[str] = []
    envs = pyproject.get("tool", {}).get("hatch", {}).get("envs", {})
    for name in FULL_MATRIX_ENVS + SUBSET_MATRIX_ENVS:
        matrix = envs.get(name, {}).get("matrix")
        where = f"{PYPROJECT} [tool.hatch.envs.{name}.matrix]"
        if not matrix:
            errors.append(f"{where}: not found")
            continue
        versions = {v for entry in matrix for v in entry.get("python", [])}
        if name in FULL_MATRIX_ENVS:
            errors += _compare(where, versions, supported)
        else:
            errors += _subset(where, versions, supported)

    project = pyproject.get("project", {})
    classifiers = {
        m.group("version")
        for c in project.get("classifiers", [])
        if (m := _CLASSIFIER_RE.match(c))
    }
    minors = {v.rstrip("t") for v in supported}
    errors += _compare(f"{PYPROJECT} 'Programming Language :: Python :: 3.N' classifiers", classifiers, minors)

    floor = min(minors, key=version_key)
    requires = _REQUIRES_RE.search(project.get("requires-python", ""))
    if requires is None or requires.group("version") != floor:
        errors.append(f"{PYPROJECT} requires-python: expected >={floor}")
    return errors


def check_readme(text: str, supported: set[str]) -> list[str]:
    errors: list[str] = []
    matrix = _README_MATRIX_RE.search(text)
    if matrix is None:
        errors.append(f"{README}: no 'Supports Python matrix:' line")
    else:
        listed = {v.strip() for v in matrix.group("versions").split(",") if v.strip()}
        errors += _compare(f"{README} 'Supports Python matrix:' line", listed, supported)
    floor = min({v.rstrip("t") for v in supported}, key=version_key)
    for pattern in _README_FLOOR_RES:
        for match in pattern.finditer(text):
            if match.group("version") != floor:
                errors.append(f"{README}: {match.group()!r} does not name the minimum version {floor}")
    return errors


def check_test_workflow(text: str, supported: set[str]) -> list[str]:
    found = {m.group("version") for m in _MATRIX_PY_RE.finditer(text)}
    return _subset(f"{TEST_WORKFLOW} matrix.py literals", found, supported)


def check(root: Path) -> list[str]:
    models = _load_module(root, MODELS, "_check_python_versions_models")
    mappings = _load_module(root, MAPPINGS, "_check_python_versions_mappings")
    supported = set(models.PARAMS["PyVersion"])
    pyproject = tomllib.loads((root / PYPROJECT).read_text(encoding="utf-8"))
    return (
        check_models(models, supported)
        + check_mappings(mappings, supported)
        + check_upload_steps(
            (root / WHEELS_WORKFLOW).read_text(encoding="utf-8"), wheel_versions(mappings), supported
        )
        + check_pyproject(pyproject, supported)
        + check_readme((root / README).read_text(encoding="utf-8"), supported)
        + check_test_workflow((root / TEST_WORKFLOW).read_text(encoding="utf-8"), supported)
    )


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument("--root", type=Path, default=Path(__file__).resolve().parents[1])
    args = parser.parse_args(argv)
    errors = check(args.root.resolve())
    for error in errors:
        print(error, file=sys.stderr)
    return 1 if errors else 0


if __name__ == "__main__":
    sys.exit(main())
