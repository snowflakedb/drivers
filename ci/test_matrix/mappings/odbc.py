"""
ODBC driver mapping table.

Single source of truth for every (OS, Arch) → ODBC-specific lookup the
generator needs. Adding a new lane is one row in ODBC_PLATFORM.

Per-row keys:
  driver_lib       (str, required) Shared-library file name. Linux/macOS use the
                   OS default; Windows x86 uses sfodbc32.dll.
  driver_artifact  (str, optional) Name of the artifact uploaded by the
                   build_odbc_driver job in test-odbc.yml. Omit on lanes that
                   have no GHA build today (Linux ARM, Windows ARM);
                   _build_gha_row treats absence as "skip this cell".
  msvc_arch        (str, optional) Passed to vcvarsall.bat <arch> on Windows
                   non-x64 cells.
  vcpkg_triplet    (str, Windows only) Triplet passed to `vcpkg install` and
                   used as the vcpkg cache-key segment. Required on every
                   Windows lane that runs ODBC tests; the workflow has no
                   default fallback.
  cache_key        (str, required when driver_artifact is set) Shared-key value
                   passed to actions/cargo-cache for the build_odbc_driver job.
                   validate_mappings raises if missing on a built lane.
  cargo_extra      (str, optional) Extra cargo flags consumed by
                   build_odbc_driver (e.g. "--features vendored-openssl").
                   Empty string is fine.
  cargo_target     (str, optional) Cross-compile target triple for
                   `cargo build --target <triple>`. Windows x86 uses
                   i686-pc-windows-msvc; Windows arm uses
                   arm64ec-pc-windows-msvc (the host triple on `windows-11-arm`
                   is aarch64-pc-windows-msvc, so arm64ec must be explicit).

To add a new platform: add an entry here. If the lane needs a GHA build, also
set driver_artifact + cache_key (+ optional cargo_extra/cargo_target). The
build matrix consumed by build_odbc_driver in test-odbc.yml is generated from
this table by `generate_matrix.py --emit-build-matrix`.
"""

ODBC_PLATFORM: dict[tuple[str, str], dict[str, str]] = {
    ("ubuntu",  "x64"): {"driver_lib": "libsfodbc.so",   "driver_artifact": "Linux x64",
                         "cache_key": "odbc"},
    ("ubuntu",  "arm"): {"driver_lib": "libsfodbc.so"},
    ("macos",   "arm"): {"driver_lib": "libsfodbc.dylib", "driver_artifact": "macOS ARM64",
                         "cache_key": "odbc"},
    ("windows", "x64"): {"driver_lib": "sfodbc.dll",      "driver_artifact": "Windows x64",
                         "vcpkg_triplet": "x64-windows", "cache_key": "odbc-x64",
                         "cargo_extra": "--features vendored-openssl"},
    ("windows", "x86"): {"driver_lib": "sfodbc32.dll",    "driver_artifact": "Windows x86",
                         "msvc_arch": "x86", "vcpkg_triplet": "x86-windows",
                         "cache_key": "odbc-x86",
                         "cargo_extra": "--no-default-features --features vendored-openssl",
                         "cargo_target": "i686-pc-windows-msvc"},
    ("windows", "arm"): {"driver_lib": "sfodbc.dll",      "driver_artifact": "Windows ARM64EC",
                         "msvc_arch": "arm64ec", "vcpkg_triplet": "arm64ec-windows",
                         "cache_key": "odbc-arm64ec",
                         "cargo_extra": "--features vendored-openssl",
                         "cargo_target": "arm64ec-pc-windows-msvc"},
}
