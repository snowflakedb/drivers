"""
Python driver mapping table.

Single source of truth for every (OS, Arch) → Python-specific lookup the
generator needs. Adding a new platform is one row in PYTHON_PLATFORM; adding
a new sdist-only Python version is one entry in SDIST_PY.

Per-row keys:
  cibw_key       (str, required) Platform key consumed by
                 _build-python-wheels.yml's `targets` input. Must be one of
                 {linux_x86, linux_aarch, macos_x86, macos_arm,
                  windows_x86, windows_arm} — the keys defined in that
                 workflow's inline PLATFORMS dict. Used by
                 generate_matrix.py --emit-build-targets to produce the
                 JSON the build workflow expects without a parallel
                 hardcoded translation table.
  wheel_artifact (str, required) Wheel artifact basename uploaded by the
                 _build-python-wheels.yml reusable workflow.
  wheels         (set[str], required) Python versions for which a wheel is
                 actually built. The contract with _build-python-wheels.yml's
                 `targets` argument — keep in sync.

SDIST_PY is a set of Python versions that always install from sdist
(no wheels are built for them — currently 3.10).
"""

PYTHON_PLATFORM: dict[tuple[str, str], dict] = {
    ("ubuntu",  "x64"): {"cibw_key": "linux_x86",   "wheel_artifact": "manylinux_x86_64", "wheels": {"3.13"}},
    ("ubuntu",  "arm"): {"cibw_key": "linux_aarch", "wheel_artifact": "manylinux_aarch64", "wheels": {"3.11", "3.14"}},
    ("macos",   "arm"): {"cibw_key": "macos_arm",   "wheel_artifact": "macosx_arm64",     "wheels": {"3.12", "3.14"}},
    ("macos",   "x64"): {"cibw_key": "macos_x86",   "wheel_artifact": "macosx_x86_64",    "wheels": {"3.11", "3.12", "3.13", "3.14"}},
    ("windows", "x64"): {"cibw_key": "windows_x86", "wheel_artifact": "win_amd64",        "wheels": {"3.11", "3.12", "3.14"}},
    ("windows", "arm"): {"cibw_key": "windows_arm", "wheel_artifact": "win_arm64",        "wheels": {"3.11", "3.12"}},
}

SDIST_PY: set[str] = {"3.10"}
