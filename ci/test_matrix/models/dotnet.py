"""Dotnet driver coverage model — see models/__init__.py for the schema."""

PARAMS = {
    "OS":            ["ubuntu", "macos", "windows"],
    "Arch":          ["x64", "arm", "x86"],
    "Cloud":         ["aws", "gcp", "azure"],
    "DotnetVersion": ["net472", "net481", "net8.0", "net9.0", "net10.0", "net11.0"],
    "Mode":          ["regression", "interop"],
}

# Cloud rotation: each non-framework dotnet version maps to one cloud. Applied via
# CONSTRAINTS so it reduces both the merge pairwise pool and the nightly
# cartesian product. All clouds are covered across Python versions.
_CLOUD_FOR_DOTNET = {
    "net8.0": "gcp",
    "net9.0": "aws",
    "net10.0": "azure",
    "net11.0": "aws",
}

# Interop tests only support these TFMs (must match the test project's TargetFrameworks).
_INTEROP_TFMS = {"net472", "net9.0", "net10.0", "net11.0"}

# Regression tests only support these TFMs (must match Snowflake.Data.Tests.Unit TargetFrameworks).
_REGRESSION_TFMS = {"net481", "net8.0", "net9.0", "net10.0", "net11.0"}


def is_valid(c):
    """Block-list: return False to forbid a combo, fall through to allow."""
    # ubuntu: x64 only.
    if c["OS"] == "ubuntu" and c["Arch"] != "x64":
        return False

    # macos: arm only.
    if c["OS"] == "macos" and c["Arch"] != "arm":
        return False

    # windows: x64, and x86 for a 32-bit .NET process over a 32-bit sf_core.
    if c["OS"] == "windows" and c["Arch"] not in ("x64", "x86"):
        return False

    # .NET Framework (net472, net481): Windows only.
    if c["DotnetVersion"] in ("net472", "net481") and c["OS"] != "windows":
        return False

    # products space dim restriction
    if c["DotnetVersion"] in _CLOUD_FOR_DOTNET and _CLOUD_FOR_DOTNET[c["DotnetVersion"]] != c["Cloud"]:
        return False

    # --- Mode / TFM constraints ---
    # Interop tests only target a subset of TFMs.
    if c["Mode"] == "interop" and c["DotnetVersion"] not in _INTEROP_TFMS:
        return False

    # Regression tests only target TFMs the main test project builds.
    if c["Mode"] == "regression" and c["DotnetVersion"] not in _REGRESSION_TFMS:
        return False

    # Interop doesn't need cloud variation — pin to a single value to collapse the dimension.
    if c["Mode"] == "interop" and c["Cloud"] != "aws":
        return False

    return True


CONSTRAINTS = [is_valid]


def merge_valid(c):
    """Pairwise-only block-list: combos returning False run at nightly only.

    macOS runner availability is limited. Keep only the explicit macOS
    PR_CELLS (net10.0 interop, net11.0 regression and interop) at merge; block
    macOS from the pairwise pool so other (Cloud, DotnetVersion) combos appear
    at nightly.
    """
    if c["OS"] == "macos":
        return False

    if c["Arch"] == "x86":
        return False

    return True


MERGE_VALID = [merge_valid]

PR_CELLS = [
    {"OS": "windows", "Arch": "x64", "Cloud": "aws",   "DotnetVersion": "net481",  "Mode": "regression"},
    {"OS": "windows", "Arch": "x86", "Cloud": "azure", "DotnetVersion": "net481",  "Mode": "regression"},
    {"OS": "ubuntu",  "Arch": "x64", "Cloud": "aws",   "DotnetVersion": "net10.0", "Mode": "regression"},
    {"OS": "ubuntu",  "Arch": "x64", "Cloud": "gcp",   "DotnetVersion": "net11.0", "Mode": "regression"},
    {"OS": "windows", "Arch": "x64", "Cloud": "aws",   "DotnetVersion": "net11.0", "Mode": "regression"},
    {"OS": "macos",   "Arch": "arm", "Cloud": "azure", "DotnetVersion": "net11.0", "Mode": "regression"},
    {"OS": "windows", "Arch": "x64", "Cloud": "aws",   "DotnetVersion": "net472",  "Mode": "interop"},
    {"OS": "ubuntu",  "Arch": "x64", "Cloud": "aws",   "DotnetVersion": "net10.0", "Mode": "interop"},
    {"OS": "windows", "Arch": "x64", "Cloud": "aws",   "DotnetVersion": "net10.0", "Mode": "interop"},
    {"OS": "macos",   "Arch": "arm", "Cloud": "aws",   "DotnetVersion": "net10.0", "Mode": "interop"},
    {"OS": "ubuntu",  "Arch": "x64", "Cloud": "aws",   "DotnetVersion": "net11.0", "Mode": "interop"},
    {"OS": "windows", "Arch": "x64", "Cloud": "azure", "DotnetVersion": "net11.0", "Mode": "interop"},
    {"OS": "macos",   "Arch": "arm", "Cloud": "gcp",   "DotnetVersion": "net11.0", "Mode": "interop"},
]

MERGE_QUEUE_CELLS = [
    {"OS": "ubuntu", "Arch": "x64", "Cloud": "aws", "DotnetVersion": "net11.0", "Mode": "regression"},
]

JSON_CELLS = {
    "pr": [],
    "merge": [],
    "nightly": []
}
