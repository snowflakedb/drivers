"""Rust core (sf_core) coverage model — see models/__init__.py for the schema."""

PARAMS = {
    "OS":    ["ubuntu", "macos", "windows"],
    "Arch":  ["x64", "arm", "x86"],
    # Backing-cloud axis. Each cell decodes a different
    # `parameters_${Cloud}.json.gpg` and exports `cloud_provider` as
    # an env var to the test process so tests can gate on it.
    # PR / merge-queue cells are pinned to `aws`; gcp and azure
    # appear only at pairwise / nightly trigger levels.
    "Cloud": ["aws", "gcp", "azure"],
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
    {"OS": "ubuntu",  "Arch": "x64", "Cloud": "aws"},
    {"OS": "macos",   "Arch": "arm", "Cloud": "aws"},
    {"OS": "windows", "Arch": "arm", "Cloud": "aws"},
    {"OS": "windows", "Arch": "x86", "Cloud": "aws"},
]

MERGE_QUEUE_CELLS = [
    {"OS": "ubuntu", "Arch": "x64", "Cloud": "aws"},
]

JSON_CELLS = {"pr": [], "merge": [], "nightly": []}
