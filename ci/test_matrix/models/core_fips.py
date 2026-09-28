"""Rust core FIPS smoke-test coverage model."""

PARAMS = {
    "OS": ["ubuntu", "macos", "windows"],
    "Arch": ["x64", "arm"],
}


def is_valid(c):
    """Block-list: return False to forbid a combo, fall through to allow."""
    if c["OS"] == "windows":
        if c["Arch"] == "arm":
            return False
    return True


CONSTRAINTS = [is_valid]

PR_CELLS = [
    {"OS": "ubuntu", "Arch": "x64"},
    {"OS": "ubuntu", "Arch": "arm"},
    {"OS": "macos", "Arch": "x64"},
    {"OS": "macos", "Arch": "arm"},
    {"OS": "windows", "Arch": "x64"},
]

MERGE_QUEUE_CELLS = [
    {"OS": "ubuntu", "Arch": "x64"},
]

JSON_CELLS = {"pr": [], "merge": [], "nightly": []}
