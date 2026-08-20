"""Python driver coverage model — see models/__init__.py for the schema."""

PARAMS = {
    "OS":        ["ubuntu", "macos", "windows"],
    "Arch":      ["x64", "arm"],
    "Cloud":     ["aws", "gcp", "azure"],
    "PyVersion": ["3.10", "3.11", "3.12", "3.13", "3.14"],
    "HatchEnv":  ["test", "test-pandas", "test-native-arrow"],
}


# Cloud rotation: each Python version maps to one cloud. Applied via
# CONSTRAINTS so it reduces both the merge pairwise pool and the nightly
# cartesian product. All clouds are covered across Python versions.
_CLOUD_FOR_PY = {
    "3.10": "aws",
    "3.11": "gcp",
    "3.12": "azure",
    "3.13": "aws",
    "3.14": "gcp",
}


def is_valid(c):
    """Block-list: return False to forbid a combo, fall through to allow."""
    if c["OS"] == "windows" and c["Arch"] == "arm":
        # No CPython 3.10 build for Windows-on-ARM (tier-3 from 3.11).
        if c["PyVersion"] == "3.10":      return False
        # No pyarrow win_arm64 wheel; source-build fails on GHA windows-11-arm
        # runner (no Arrow C++ libs).
        if c["HatchEnv"] == "test-pandas": return False

    # Native-arrow rebuilds python_bridge with a Cargo feature the production
    # wheel does not ship. One Linux/py3.13/aws cell is enough; other combos
    # would multiply nightly cost, and the Cython↔native relationship is
    # untested on Windows (pre-1970 timestamps).
    if c["HatchEnv"] == "test-native-arrow":
        if c["OS"] != "ubuntu":      return False
        if c["Arch"] != "x64":       return False
        if c["Cloud"] != "aws":      return False
        if c["PyVersion"] != "3.13": return False

    # Cloud rotation: each Python version is tested against one cloud.
    if c["Cloud"] != _CLOUD_FOR_PY[c["PyVersion"]]:
        return False

    return True


CONSTRAINTS = [is_valid]


def merge_valid(c):
    """Pairwise-only block-list: combos returning False run at nightly only.

    macOS runner availability is the binding constraint on merge-queue
    throughput. There is no macOS PR cell — all macOS coverage is deferred
    to the merge queue. macos-15-intel (x64) runs via Rosetta 2 on GitHub
    workers and is very slow, so x64 is deferred entirely to nightly. Only
    one macOS ARM job runs at merge:

      * macos-arm: gcp + py3.11 + test-pandas  (gcp is the cloud for 3.11)

    Every other macOS combo (including all x64) runs at nightly via the
    unfiltered cartesian product.
    """
    if c["OS"] == "macos":
        if c["Arch"] == "x64":
            return False  # Rosetta 2 simulation on GHA workers — deferred to nightly
        if c["Arch"] == "arm":
            if c["PyVersion"] != "3.11":        return False
            if c["HatchEnv"] != "test-pandas":  return False

    return True


MERGE_VALID = [merge_valid]

PR_CELLS = [
    {"OS": "ubuntu",  "Arch": "x64", "Cloud": "aws", "PyVersion": "3.13", "HatchEnv": "test"},
    {"OS": "ubuntu",  "Arch": "x64", "Cloud": "aws", "PyVersion": "3.13", "HatchEnv": "test-native-arrow"},
    {"OS": "windows", "Arch": "x64", "Cloud": "gcp", "PyVersion": "3.14", "HatchEnv": "test"},
]

MERGE_QUEUE_CELLS = [
    {"OS": "ubuntu", "Arch": "x64", "Cloud": "aws", "PyVersion": "3.13", "HatchEnv": "test"},
    {"OS": "ubuntu", "Arch": "x64", "Cloud": "aws", "PyVersion": "3.13", "HatchEnv": "test-native-arrow"},
]

JSON_CELLS = {
    "pr": [],
    "merge": [
        {"OS": "ubuntu", "Arch": "x64", "Cloud": "aws", "PyVersion": "3.13", "HatchEnv": "test"},
        {"OS": "ubuntu", "Arch": "x64", "Cloud": "aws", "PyVersion": "3.13", "HatchEnv": "test-pandas"},
    ],
    "nightly": [],
}
