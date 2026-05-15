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
  cross_compile  (bool, optional) True if this lane cannot build the wheel
                 on the same runner that runs tests. Set on:
                   - linux_aarch (cibuildwheel cross-compiles via QEMU on
                     an x86 host; tests run on a native ARM runner)
                   - linux_x86   (cibuildwheel runs in a manylinux container;
                     simpler to keep tests outside the container via the
                     artifact path)
                   - windows_arm (cibuildwheel does not support win_arm64;
                     a manual build path lives in _build-python-wheels.yml
                     and is not currently inlined into python_checks)
                 cross_compile rows download the wheel artifact produced by
                 build_wheels (existing flow). Other rows (macos_arm,
                 macos_x86, windows_x64) build the wheel inline on the test
                 runner via cibuildwheel — see the colocated build steps in
                 python_checks (test-python.yml).
  cibw_arch     (str, required when cross_compile is False) cibuildwheel
                 platform tag passed via CIBW_BUILD (e.g. "macosx_arm64",
                 "win_amd64"). Same value the build workflow uses.
  cargo_target  (str, optional) Rust target triple for cross-compilation
                 builds. Used by sf_core's cargo build when building for a
                 non-native arch. Currently unused for native lanes
                 (defaults to host triple).
  cargo_extra_args (str, required when cross_compile is False) Cargo flags
                 used to build sf_core inside the colocated job. Mirrors
                 the corresponding entry in _build-python-wheels.yml's
                 PLATFORMS dict.
  lib_name      (str, required when cross_compile is False) Filename of the
                 sf_core shared library produced by cargo. Copied into
                 python/src/snowflake/connector/_core/ before cibuildwheel
                 picks it up.

SDIST_PY is a set of Python versions that always install from sdist
(no wheels are built for them — currently 3.10).
"""

PYTHON_PLATFORM: dict[tuple[str, str], dict] = {
    ("ubuntu",  "x64"): {
        "cibw_key": "linux_x86",
        "wheel_artifact": "manylinux_x86_64",
        "wheels": {"3.13"},
        # Container build (manylinux_2_28_x86_64) — colocation deferred;
        # keep on artifact path.
        "cross_compile": True,
    },
    ("ubuntu",  "arm"): {
        "cibw_key": "linux_aarch",
        "wheel_artifact": "manylinux_aarch64",
        "wheels": {"3.11", "3.14"},
        # QEMU cross-compile on an x86 host; cannot run tests on the build
        # runner. Keep on artifact path.
        "cross_compile": True,
    },
    ("macos",   "arm"): {
        "cibw_key": "macos_arm",
        "wheel_artifact": "macosx_arm64",
        "wheels": {"3.12", "3.14"},
        "cibw_arch": "macosx_arm64",
        "cargo_extra_args": "--features vendored-openssl --config profile.release.opt-level=2",
        "lib_name": "libsf_core.dylib",
    },
    ("macos",   "x64"): {
        "cibw_key": "macos_x86",
        "wheel_artifact": "macosx_x86_64",
        "wheels": {"3.11", "3.12", "3.13", "3.14"},
        "cibw_arch": "macosx_x86_64",
        "cargo_extra_args": "--features vendored-openssl --config profile.release.opt-level=2",
        "lib_name": "libsf_core.dylib",
    },
    ("windows", "x64"): {
        "cibw_key": "windows_x86",
        "wheel_artifact": "win_amd64",
        "wheels": {"3.11", "3.12", "3.14"},
        "cibw_arch": "win_amd64",
        "cargo_extra_args": "--features vendored-openssl --config profile.release.opt-level=2 --config profile.release.strip=false",
        "lib_name": "sf_core.dll",
    },
    ("windows", "arm"): {
        "cibw_key": "windows_arm",
        "wheel_artifact": "win_arm64",
        "wheels": {"3.11", "3.12"},
        # cibuildwheel does not support win_arm64; manual build path in
        # _build-python-wheels.yml. Inline colocation deferred.
        "cross_compile": True,
    },
}

SDIST_PY: set[str] = {"3.10"}
