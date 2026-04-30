"""ODBC driver coverage model — see models/__init__.py for the schema."""

PARAMS = {
    "OS":    ["ubuntu", "macos", "windows"],
    "Arch":  ["x64", "x86", "arm"],
    "Cloud": ["aws", "gcp", "azure"],
}


def is_valid(c):
    """Block-list: return False to forbid a combo, fall through to allow."""
    if c["OS"] == "ubuntu":
        # ubuntu ODBC builds only x64.
        if c["Arch"] == "arm": return False
        if c["Arch"] == "x86": return False

    if c["OS"] == "macos":
        # macos ODBC builds only arm.
        if c["Arch"] == "x64": return False
        if c["Arch"] == "x86": return False

    # Windows ARM64 is built on the windows-11-arm runner; no exclusion needed.

    return True


CONSTRAINTS = [is_valid]

PR_CELLS = [
    {"OS": "ubuntu",  "Arch": "x64", "Cloud": "aws"},
    {"OS": "macos",   "Arch": "arm", "Cloud": "gcp"},
    {"OS": "windows", "Arch": "x64", "Cloud": "azure"},
    {"OS": "windows", "Arch": "x86", "Cloud": "aws"},
]

JSON_CELLS = {
    "pr": [],
    "merge": [
        {"OS": "ubuntu", "Arch": "x64", "Cloud": "aws"},
    ],
    "nightly": [],
}
