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


def merge_valid(c):
    """Pairwise-only block-list: combos returning False run at nightly only.

    macOS runner availability is the binding constraint on merge-queue
    throughput. There is no macOS PR cell — all macOS coverage is deferred
    to the merge queue. To keep MQ macOS load capped at one Intel + one ARM
    job, pin each arch to a single representative combo via pairwise:

      * macos-arm:  gcp + py3.10 + test-pandas
      * macos-x64 (macos-15-intel): aws + py3.13 + test

    Every other macOS combo runs at nightly via the unfiltered cartesian
    product.
    """
    if c["OS"] == "macos":
        if c["Arch"] == "arm":
            if c["Cloud"] != "gcp":             return False
            if c["PyVersion"] != "3.10":        return False
            if c["HatchEnv"] != "test-pandas":  return False
        if c["Arch"] == "x64":
            if c["Cloud"] != "aws":             return False
            if c["PyVersion"] != "3.13":        return False
            if c["HatchEnv"] != "test":         return False

    return True


MERGE_VALID = [merge_valid]

PR_CELLS = [
    {"OS": "ubuntu",  "Arch": "x64", "Cloud": "aws",   "PyVersion": "3.13", "HatchEnv": "test"},
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
