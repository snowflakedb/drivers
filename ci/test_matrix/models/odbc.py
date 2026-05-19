"""ODBC driver coverage model — see models/__init__.py for the schema."""

PARAMS = {
    "OS":    ["ubuntu", "macos", "windows"],
    "Arch":  ["x64", "x86", "arm"],
    "Cloud": ["aws", "gcp", "azure"],
    # iODBC is a separate driver manager on macOS that ships 4-byte SQLWCHAR;
    # the driver picks the encoding at runtime from `sf.odbc.ini`. Only macOS
    # is exercised under iODBC today — Linux uses unixODBC, Windows uses the
    # OS DM. See odbc/README.md "Driver manager encoding".
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

    # iODBC only applies on macOS (Homebrew libiodbc); Linux/Windows have no
    # iODBC test lane.
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
    # Keep the iODBC cell at PR scope: it exercises the 4-byte SQLWCHAR path
    # the rest of the matrix never hits.
    {"OS": "macos",   "Arch": "arm", "Cloud": "aws",   "DM": "iodbc"},
]

JSON_CELLS = {
    "pr": [],
    "merge": [
        {"OS": "ubuntu", "Arch": "x64", "Cloud": "aws", "DM": "unixodbc"},
    ],
    "nightly": [],
}
