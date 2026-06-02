"""ODBC driver coverage model — see models/__init__.py for the schema."""

PARAMS = {
    "OS":    ["ubuntu", "macos", "windows"],
    "Arch":  ["x64", "x86", "arm"],
    "Cloud": ["aws", "gcp", "azure"],
    "DM":    ["unixodbc", "iodbc"],
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

    if c["DM"] == "iodbc":
        if c["OS"] != "macos": return False

    return True


CONSTRAINTS = [is_valid]


def merge_valid(c):
    """Pairwise-only block-list: combos returning False run at nightly only.

    macOS runner availability is scarce. Hold MQ macOS load to a single
    job (the explicit PR_CELL macos-arm-gcp, which still runs at merge
    cumulatively) by blocking macOS from the pairwise pool entirely.
    Other (macos, Cloud) cells run at nightly via the unfiltered cartesian
    product.
    """
    if c["OS"] == "macos": return False

    return True


MERGE_VALID = [merge_valid]

PR_CELLS = [
    {"OS": "ubuntu",  "Arch": "x64", "Cloud": "aws",   "DM": "unixodbc"},
    {"OS": "macos",   "Arch": "arm", "Cloud": "gcp",   "DM": "unixodbc"},
    {"OS": "windows", "Arch": "x64", "Cloud": "azure", "DM": "unixodbc"},
    {"OS": "windows", "Arch": "x86", "Cloud": "aws",   "DM": "unixodbc"},
    {"OS": "macos",   "Arch": "arm", "Cloud": "aws",   "DM": "iodbc"},
]

MERGE_QUEUE_CELLS = [
    {"OS": "ubuntu", "Arch": "x64", "Cloud": "aws", "DM": "unixodbc"},
]

JSON_CELLS = {
    "pr": [],
    "merge": [
        {"OS": "ubuntu", "Arch": "x64", "Cloud": "aws", "DM": "unixodbc"},
    ],
    "nightly": [],
}
