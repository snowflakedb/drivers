"""
Dotnet driver mapping table.

DOTNET_PLATFORM: (OS, Arch) → platform-specific metadata (native lib filename).
DOTNET_TFM: DotnetVersion → version-specific metadata (test invocation command).
"""

DOTNET_PLATFORM: dict[tuple[str, str], dict] = {
    ("ubuntu", "x64"): {"sf_core_lib": "libsf_core.so", "cargo_flags": ""},
    ("macos", "arm"):  {"sf_core_lib": "libsf_core.dylib", "cargo_flags": ""},
    ("windows", "x64"): {"sf_core_lib": "sf_core.dll", "cargo_flags": "--features vendored-openssl"},
}

DOTNET_TFM: dict[str, dict] = {
    "net472":  {"test_runner": "dotnet test --no-build --framework net472 --verbosity detailed", "copy_native_lib": True},
    "net481":  {"test_runner": "dotnet test --no-build --framework net481 --verbosity detailed", "copy_native_lib": True},
    "net8.0":  {"test_runner": "dotnet run --no-build --framework net8.0", "copy_native_lib": False},
    "net9.0":  {"test_runner": "dotnet run --no-build --framework net9.0", "copy_native_lib": False},
    "net10.0": {"test_runner": "dotnet run --no-build --framework net10.0", "copy_native_lib": False},
}
