"""Validate that conda runtime dependencies match pyproject.toml."""

from __future__ import annotations

import re
import sys

from pathlib import Path

import tomllib
import yaml


_RUN_ONLY_ALLOWED = {"python"}


def python_dir() -> Path:
    return Path(__file__).resolve().parents[3]


def normalize_name(name: str) -> str:
    return name.strip().lower().replace("_", "-")


def split_requirement(req: str) -> tuple[str, str]:
    req = req.split(";", 1)[0]
    req = req.split("#", 1)[0]
    req = req.strip()
    if not req:
        return "", ""

    match = re.search(r"(<=|>=|==|!=|~=|<|>|=)", req)
    if match:
        name = req[: match.start()].strip()
        spec = req[match.start() :].strip()
    else:
        parts = req.split()
        name = parts[0] if parts else ""
        spec = ""

    name = name.split("[", 1)[0]

    spec = re.sub(r"\s*,\s*", ",", spec)
    spec = re.sub(r"\s+", "", spec)
    return normalize_name(name), spec


def get_pyproject_dependencies(pyproject_path: Path) -> dict[str, str]:
    with pyproject_path.open("rb") as fh:
        data = tomllib.load(fh)

    project = data.get("project")
    if not project:
        raise RuntimeError(f"Missing [project] table in {pyproject_path}")
    if "dependencies" not in project:
        raise RuntimeError(f"Missing [project].dependencies in {pyproject_path}")

    deps: dict[str, str] = {}
    for item in project["dependencies"]:
        name, spec = split_requirement(item)
        if name and name not in _RUN_ONLY_ALLOWED:
            deps[name] = spec
    return deps


def get_meta_run_requirements(meta_path: Path) -> dict[str, str]:
    text = meta_path.read_text(encoding="utf-8")
    text = re.sub(r"\{#.*?#\}", "", text, flags=re.DOTALL)
    cleaned = "\n".join(
        line
        for line in text.splitlines()
        if "{%" not in line and "%}" not in line and not ("{{" in line and "}}" in line)
    )

    try:
        data = yaml.safe_load(cleaned) or {}
    except Exception as exc:
        raise RuntimeError(f"Failed to parse YAML for {meta_path}") from exc

    run_items = ((data.get("requirements") or {}).get("run")) or []

    deps: dict[str, str] = {}
    for index, item in enumerate(run_items):
        if not isinstance(item, str):
            raise TypeError(
                f"requirements.run entry at index {index} in {meta_path} must be a "
                f"string; got {type(item).__name__}: {item!r}"
            )
        name, spec = split_requirement(item)
        if name and name not in _RUN_ONLY_ALLOWED:
            deps[name] = spec
    return deps


def compare_deps(expected: dict[str, str], actual: dict[str, str]) -> str:
    missing = sorted(set(expected) - set(actual))
    extra = sorted(set(actual) - set(expected))
    mismatched = sorted(name for name in set(expected) & set(actual) if expected[name] != actual[name])

    if not (missing or extra or mismatched):
        return ""

    lines: list[str] = []
    if missing:
        lines.append("Missing in meta.yaml run:")
        lines += [f"  - {name} ({expected[name] or 'no spec'})" for name in missing]
    if extra:
        lines.append("Extra in meta.yaml run:")
        lines += [f"  - {name} ({actual[name] or 'no spec'})" for name in extra]
    if mismatched:
        lines.append("Version spec mismatches:")
        lines += [f"  - {name}: pyproject.toml='{expected[name]}' vs meta.yaml='{actual[name]}'" for name in mismatched]
    return "\n".join(lines)


def main() -> int:
    root = python_dir()
    expected = get_pyproject_dependencies(root / "pyproject.toml")
    actual = get_meta_run_requirements(root / "ci" / "anaconda" / "recipe" / "meta.yaml")

    diff = compare_deps(expected, actual)
    if not diff:
        return 0
    print(diff)
    return 1


if __name__ == "__main__":
    sys.exit(main())
