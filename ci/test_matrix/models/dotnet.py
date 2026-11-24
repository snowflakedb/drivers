"""Dotnet driver coverage model — see models/__init__.py for the schema.

Suites
------
unit     unit + interop (integ). Dimensions: OS, version. Cloud is a dummy
         aws value in PARAMS only so the cartesian product stays rectangular;
         unit rows do not emit cloud_provider and do not decode secrets.
e2e      E2E + Wiremock, new driver. Dimensions: OS, version, cloud.
e2e_old  E2E + Wiremock, old driver. One unix/net11.0 cell.

Kind is framework (net481 on PR/merge, plus net472 on nightly; Windows only)
vs modern (net8.0 / net9.0 / net10.0 / net11.0). Interop has no net8.0 TFM; that cell
runs unit only. Windows ships x64 and x86 (both on windows-latest).
"""

PARAMS = {
    "OS":            ["ubuntu", "macos", "windows"],
    "Arch":          ["x64", "arm", "x86"],
    "Cloud":         ["aws", "azure", "gcp"],
    "DotnetVersion": ["net472", "net481", "net8.0", "net9.0", "net10.0", "net11.0"],
    "Mode":          ["unit", "e2e", "e2e_old"],
}

_ARCHS_FOR_OS = {
    "ubuntu": {"x64"},
    "macos": {"arm"},
    "windows": {"x64", "x86"},
}

_MODERN = ("net8.0", "net9.0", "net10.0", "net11.0")

# One cloud per (OS, TFM) for E2E. Each OS sees every provider. Windows x64
# and x86 share this map so both arches get the same TFM→cloud pairing.
_E2E_CLOUD = {
    ("ubuntu",  "net11.0"): "aws",
    ("ubuntu",  "net10.0"): "aws",
    ("ubuntu",  "net8.0"):  "azure",
    ("ubuntu",  "net9.0"):  "gcp",
    ("macos",   "net11.0"): "gcp",
    ("macos",   "net10.0"): "gcp",
    ("macos",   "net8.0"):  "aws",
    ("macos",   "net9.0"):  "azure",
    ("windows", "net481"):  "azure",
    ("windows", "net472"):  "gcp",
    ("windows", "net11.0"): "gcp",
    ("windows", "net10.0"): "gcp",
    ("windows", "net8.0"):  "aws",
    ("windows", "net9.0"):  "azure",
}


def _supported_tfm(os_name: str, tfm: str) -> bool:
    if tfm in ("net472", "net481"):
        return os_name == "windows"
    return tfm in _MODERN


def is_valid(c):
    """Block-list: return False to forbid a combo, fall through to allow."""
    if c["Arch"] not in _ARCHS_FOR_OS[c["OS"]]:
        return False
    if not _supported_tfm(c["OS"], c["DotnetVersion"]):
        return False

    if c["Mode"] == "e2e_old":
        return (
            c["OS"] == "ubuntu"
            and c["Arch"] == "x64"
            and c["Cloud"] == "aws"
            and c["DotnetVersion"] == "net11.0"
        )

    if c["Mode"] == "unit":
        return c["Cloud"] == "aws"

    if c["Mode"] == "e2e":
        return c["Cloud"] == _E2E_CLOUD[(c["OS"], c["DotnetVersion"])]

    return False


CONSTRAINTS = [is_valid]


def merge_valid(_c):
    """Pairwise adds no extra cells. MERGE_CELLS lists the merge extras."""
    return False


MERGE_VALID = [merge_valid]

PR_CELLS = [
    {"OS": "ubuntu",  "Arch": "x64", "Cloud": "aws",   "DotnetVersion": "net11.0", "Mode": "unit"},
    {"OS": "windows", "Arch": "x64", "Cloud": "aws",   "DotnetVersion": "net481",  "Mode": "unit"},
    {"OS": "windows", "Arch": "x86", "Cloud": "aws",   "DotnetVersion": "net481",  "Mode": "unit"},
    {"OS": "ubuntu",  "Arch": "x64", "Cloud": "aws",   "DotnetVersion": "net11.0", "Mode": "e2e"},
    {"OS": "macos",   "Arch": "arm", "Cloud": "gcp",   "DotnetVersion": "net11.0", "Mode": "e2e"},
    {"OS": "windows", "Arch": "x64", "Cloud": "azure", "DotnetVersion": "net481",  "Mode": "e2e"},
    {"OS": "windows", "Arch": "x86", "Cloud": "azure", "DotnetVersion": "net481",  "Mode": "e2e"},
    {"OS": "ubuntu",  "Arch": "x64", "Cloud": "aws",   "DotnetVersion": "net11.0", "Mode": "e2e_old"},
]

# OS × kind for unit; remaining OS × cloud cells for E2E (including windows x86).
MERGE_CELLS = [
    {"OS": "macos",   "Arch": "arm", "Cloud": "aws",   "DotnetVersion": "net11.0", "Mode": "unit"},
    {"OS": "windows", "Arch": "x64", "Cloud": "aws",   "DotnetVersion": "net11.0", "Mode": "unit"},
    {"OS": "windows", "Arch": "x86", "Cloud": "aws",   "DotnetVersion": "net11.0", "Mode": "unit"},
    {"OS": "ubuntu",  "Arch": "x64", "Cloud": "azure", "DotnetVersion": "net8.0",  "Mode": "e2e"},
    {"OS": "ubuntu",  "Arch": "x64", "Cloud": "gcp",   "DotnetVersion": "net9.0",  "Mode": "e2e"},
    {"OS": "macos",   "Arch": "arm", "Cloud": "aws",   "DotnetVersion": "net8.0",  "Mode": "e2e"},
    {"OS": "macos",   "Arch": "arm", "Cloud": "azure", "DotnetVersion": "net9.0",  "Mode": "e2e"},
    {"OS": "windows", "Arch": "x64", "Cloud": "aws",   "DotnetVersion": "net8.0",  "Mode": "e2e"},
    {"OS": "windows", "Arch": "x64", "Cloud": "gcp",   "DotnetVersion": "net11.0", "Mode": "e2e"},
    {"OS": "windows", "Arch": "x86", "Cloud": "aws",   "DotnetVersion": "net8.0",  "Mode": "e2e"},
    {"OS": "windows", "Arch": "x86", "Cloud": "gcp",   "DotnetVersion": "net11.0", "Mode": "e2e"},
]

# Explicit MQ so windows x86 (on PR) does not run in the merge queue.
MERGE_QUEUE_CELLS = [
    {"OS": "ubuntu",  "Arch": "x64", "Cloud": "aws",   "DotnetVersion": "net11.0", "Mode": "unit"},
    {"OS": "windows", "Arch": "x64", "Cloud": "aws",   "DotnetVersion": "net481",  "Mode": "unit"},
    {"OS": "ubuntu",  "Arch": "x64", "Cloud": "aws",   "DotnetVersion": "net11.0", "Mode": "e2e"},
    {"OS": "macos",   "Arch": "arm", "Cloud": "gcp",   "DotnetVersion": "net11.0", "Mode": "e2e"},
    {"OS": "windows", "Arch": "x64", "Cloud": "azure", "DotnetVersion": "net481",  "Mode": "e2e"},
    {"OS": "ubuntu",  "Arch": "x64", "Cloud": "aws",   "DotnetVersion": "net11.0", "Mode": "e2e_old"},
]

JSON_CELLS = {
    "pr": [],
    "merge_queue": [],
    "merge": [],
    "nightly": [],
}
