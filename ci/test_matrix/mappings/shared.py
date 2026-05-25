"""
Cross-driver runner mapping table.

Used by every driver (odbc, python, core) to resolve a logical (OS, Arch) cell
to the concrete GitHub Actions runner label it runs on.
"""

# (OS, Arch) -> GitHub Actions runner label.
GHA_RUNNER = {
    ("ubuntu", "x64"): "ubuntu-latest",
    ("ubuntu", "arm"): "ubuntu-24.04-arm",
    ("macos", "arm"): "macos-latest",
    ("macos", "x64"): "macos-15-intel",
    ("windows", "x64"): "windows-latest",
    ("windows", "x86"): "windows-latest",
    ("windows", "arm"): "windows-11-arm",
}
