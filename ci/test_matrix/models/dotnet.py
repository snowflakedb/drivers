"""Dotnet driver coverage model — see models/__init__.py for the schema."""

PARAMS = {
    "OS":            ["ubuntu", "macos", "windows"],
    "Arch":          ["x64", "arm"],
    "Cloud":         ["aws", "gcp", "azure"],
    "DotnetVersion": ["net472", "net481", "net8.0", "net9.0", "net10.0"],
}

# Cloud rotation: each non-framework dotnet version maps to one cloud. Applied via
# CONSTRAINTS so it reduces both the merge pairwise pool and the nightly
# cartesian product. All clouds are covered across Python versions.
_CLOUD_FOR_DOTNET = {
    "net8.0": "gcp",
    "net9.0": "azure",
    "net10.0": "aws",
}

def is_valid(c):
    """Block-list: return False to forbid a combo, fall through to allow."""
    # ubuntu: x64 only.
    if c["OS"] == "ubuntu" and c["Arch"] != "x64":
        return False

    # macos: arm only.
    if c["OS"] == "macos" and c["Arch"] != "arm":
        return False

    # windows: x64 only.
    if c["OS"] == "windows" and c["Arch"] != "x64":
        return False

    # .NET Framework (net472, net481): Windows only.
    if c["DotnetVersion"] in ("net472", "net481") and c["OS"] != "windows":
        return False
        
    # products space dim restriction
    if c["DotnetVersion"] in ("net8.0", "net9.0", "net10.0") and _CLOUD_FOR_DOTNET[c["DotnetVersion"]] != c["Cloud"]:
        return False

    return True


CONSTRAINTS = [is_valid]


def merge_valid(c):
    """Pairwise-only block-list: combos returning False run at nightly only.

    macOS runner availability is limited. Keep only one macOS cell at merge
    (the explicit PR_CELL macos-arm-aws-net10.0); block macOS from the
    pairwise pool so other (Cloud, DotnetVersion) combos appear at nightly.
    """
    if c["OS"] == "macos":
        return False

    return True


MERGE_VALID = [merge_valid]

PR_CELLS = [
    {"OS": "ubuntu",  "Arch": "x64", "Cloud": "aws",   "DotnetVersion": "net10.0"},
    {"OS": "windows", "Arch": "x64", "Cloud": "azure", "DotnetVersion": "net481"},
]

MERGE_QUEUE_CELLS = [
    {"OS": "ubuntu", "Arch": "x64", "Cloud": "aws", "DotnetVersion": "net10.0"}
]

JSON_CELLS = {
    "pr": [], 
    "merge": [], 
    "nightly": []
}
