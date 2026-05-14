"""Python driver coverage model — see models/__init__.py for the schema."""

PARAMS = {
    "OS":        ["ubuntu", "macos", "windows"],
    "Arch":      ["x64", "arm"],
    "Cloud":     ["aws", "gcp", "azure"],
    "PyVersion": ["3.10", "3.11", "3.12", "3.13", "3.14"],
    "HatchEnv":  ["test", "test-pandas"],
}


def is_valid(c):
    """Block-list: return False to forbid a combo, fall through to allow."""
    if c["OS"] == "windows" and c["Arch"] == "arm":
        # No CPython 3.10 build for Windows-on-ARM (tier-3 from 3.11).
        if c["PyVersion"] == "3.10":      return False
        # No pyarrow win_arm64 wheel; source-build fails on GHA windows-11-arm
        # runner (no Arrow C++ libs).
        if c["HatchEnv"] == "test-pandas": return False

    return True


CONSTRAINTS = [is_valid]

PR_CELLS = [
    {"OS": "ubuntu",  "Arch": "x64", "Cloud": "aws",   "PyVersion": "3.13", "HatchEnv": "test"},
    {"OS": "macos",   "Arch": "arm", "Cloud": "gcp",   "PyVersion": "3.10", "HatchEnv": "test-pandas"},
    {"OS": "windows", "Arch": "x64", "Cloud": "azure", "PyVersion": "3.14", "HatchEnv": "test"},
]

JSON_CELLS = {
    "pr": [],
    "merge": [
        {"OS": "ubuntu", "Arch": "x64", "Cloud": "aws", "PyVersion": "3.13", "HatchEnv": "test"},
        {"OS": "ubuntu", "Arch": "x64", "Cloud": "aws", "PyVersion": "3.13", "HatchEnv": "test-pandas"},
    ],
    "nightly": [],
}
