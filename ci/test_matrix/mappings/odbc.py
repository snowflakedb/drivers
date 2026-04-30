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

To add a new platform: add an entry here. If the lane needs a GHA build, also
add a build entry to build_odbc_driver in test-odbc.yml and set
driver_artifact to the matching name.
"""

ODBC_PLATFORM: dict[tuple[str, str], dict[str, str]] = {
    ("ubuntu",  "x64"): {"driver_lib": "libsfodbc.so",   "driver_artifact": "Linux x64"},
    ("ubuntu",  "arm"): {"driver_lib": "libsfodbc.so"},
    ("macos",   "arm"): {"driver_lib": "libsfodbc.dylib", "driver_artifact": "macOS ARM64"},
    ("windows", "x64"): {"driver_lib": "sfodbc.dll",      "driver_artifact": "Windows x64",
                         "vcpkg_triplet": "x64-windows"},
    ("windows", "x86"): {"driver_lib": "sfodbc32.dll",    "driver_artifact": "Windows x86",
                         "msvc_arch": "x86", "vcpkg_triplet": "x86-windows"},
    ("windows", "arm"): {"driver_lib": "sfodbc.dll",      "driver_artifact": "Windows ARM64",
                         "msvc_arch": "arm64", "vcpkg_triplet": "arm64-windows"},
}
