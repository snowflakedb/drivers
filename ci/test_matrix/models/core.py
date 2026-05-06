"""Rust core (sf_core) coverage model — see models/__init__.py for the schema."""

PARAMS = {
    "OS":   ["ubuntu", "macos", "windows"],
    "Arch": ["x64", "arm", "x86"],
}


def is_valid(c):
    """Block-list: return False to forbid a combo, fall through to allow."""
    if c["OS"] == "ubuntu":
        # ubuntu builds only x64.
        if c["Arch"] == "arm": return False
        if c["Arch"] == "x86": return False

    if c["OS"] == "macos":
        # macos builds only arm.
        if c["Arch"] == "x64": return False
        if c["Arch"] == "x86": return False

    if c["OS"] == "windows":
        # windows builds arm and x86 today (no x64 build).
        if c["Arch"] == "x64": return False

    return True


CONSTRAINTS = [is_valid]

PR_CELLS = [
    {"OS": "ubuntu",  "Arch": "x64"},
    {"OS": "macos",   "Arch": "arm"},
    {"OS": "windows", "Arch": "arm"},
    {"OS": "windows", "Arch": "x86"},
]

JSON_CELLS = {"pr": [], "merge": [], "nightly": []}
