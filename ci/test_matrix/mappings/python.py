"""
Python driver mapping table.

Single source of truth for every (OS, Arch) → Python-specific lookup the
generator needs. Adding a new platform is one row in PYTHON_PLATFORM; adding
a new sdist-only Python version is one entry in SDIST_PY.

Per-row keys:
  wheel_artifact (str, required) Wheel artifact basename uploaded by the
                 _build-python-wheels.yml reusable workflow.
  wheels         (set[str], required) Python versions for which a wheel is
                 actually built. The contract with _build-python-wheels.yml's
                 `targets` argument — keep in sync.

SDIST_PY is a set of Python versions that always install from sdist
(no wheels are built for them — currently 3.10).
"""

PYTHON_PLATFORM: dict[tuple[str, str], dict] = {
    ("ubuntu",  "x64"): {"wheel_artifact": "manylinux_x86_64", "wheels": {"3.13"}},
    ("ubuntu",  "arm"): {"wheel_artifact": "manylinux_aarch64", "wheels": {"3.11", "3.14"}},
    ("macos",   "arm"): {"wheel_artifact": "macosx_arm64",     "wheels": {"3.12", "3.14"}},
    ("macos",   "x64"): {"wheel_artifact": "macosx_x86_64",    "wheels": {"3.11", "3.12", "3.13", "3.14"}},
    ("windows", "x64"): {"wheel_artifact": "win_amd64",        "wheels": {"3.11", "3.12", "3.14"}},
    ("windows", "arm"): {"wheel_artifact": "win_arm64",        "wheels": {"3.11", "3.12"}},
}

SDIST_PY: set[str] = {"3.10"}
