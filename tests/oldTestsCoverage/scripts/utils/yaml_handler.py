"""Load and save YAML mapping files, preserving structure and comments where possible."""

from pathlib import Path
from typing import Any

import yaml


YAML_DIR = Path(__file__).resolve().parent.parent.parent  # tests/oldTestsCoverage/

DRIVER_FILES = {
    "odbc": YAML_DIR / "odbc.yaml",
    "jdbc": YAML_DIR / "jdbc.yaml",
    "python": YAML_DIR / "python.yaml",
}


def get_yaml_path(driver: str) -> Path:
    """Return the YAML path for a driver, raising ValueError if unknown."""
    if driver not in DRIVER_FILES:
        raise ValueError(f"Unknown driver '{driver}'. Choose from: {list(DRIVER_FILES.keys())}")
    return DRIVER_FILES[driver]


def load_mapping_yaml(driver: str) -> dict[str, Any]:
    """Load a driver's YAML mapping file and return the parsed dict."""
    path = get_yaml_path(driver)
    with open(path) as f:
        return yaml.safe_load(f)


def save_mapping_yaml(driver: str, data: dict[str, Any]) -> Path:
    """Write the mapping dict back to disk. Returns the path written."""
    path = get_yaml_path(driver)
    with open(path, "w") as f:
        yaml.dump(
            data,
            f,
            default_flow_style=False,
            allow_unicode=True,
            sort_keys=False,
            width=120,
        )
    return path


def get_all_test_entries(data: dict[str, Any]) -> list[tuple[str, dict]]:
    """Yield (file_path, entry) tuples from the 'tests' section."""
    results = []
    tests = data.get("tests", {})
    if not tests:
        return results
    for file_path, entries in tests.items():
        if entries:
            for entry in entries:
                results.append((file_path, entry))
    return results


def count_by_status(data: dict[str, Any]) -> dict[str, int]:
    """Count tests by mapping status."""
    counts = {"unmapped": 0, "partial": 0, "mapped": 0, "not-applicable": 0}
    for _, entry in get_all_test_entries(data):
        ud_tests = entry.get("ud_tests", [])
        status = entry.get("status")
        if status and status in counts:
            counts[status] += 1
        elif not ud_tests:
            counts["unmapped"] += 1
        else:
            counts["mapped"] += 1
    return counts
