"""
Dotnet driver mapping table.

DOTNET_PLATFORM: (OS, Arch) → platform-specific metadata.
  sf_core_lib         (str)            Native library filename.
  cargo_flags         (str)            Extra flags for `cargo build --package sf_core`.
  cargo_target        (str, optional)  Target triple for cross-compiled native libraries;
                                       cargo writes them under target/<triple>/debug.
  dotnet_arch         (str, optional)  Architecture of the .NET SDK setup-dotnet installs;
                                       the runner's default architecture when absent.
  dotnet_build_flags  (str, optional)  Extra flags for `dotnet build`.
DOTNET_TFM: DotnetVersion → version-specific metadata.
"""

DOTNET_PLATFORM: dict[tuple[str, str], dict] = {
    ("ubuntu", "x64"): {"sf_core_lib": "libsf_core.so", "cargo_flags": ""},
    ("macos", "arm"):  {"sf_core_lib": "libsf_core.dylib", "cargo_flags": ""},
    ("windows", "x64"): {"sf_core_lib": "sf_core.dll", "cargo_flags": ""},
    # x86 builds sf_core with the features the ODBC x86 driver links: sonic-rs targets
    # x86_64 and aarch64 only today, so the default sonic-json feature is off.
    ("windows", "x86"): {"sf_core_lib": "sf_core.dll",
                         "cargo_flags": "--no-default-features --features protobuf",
                         "cargo_target": "i686-pc-windows-msvc",
                         "dotnet_arch": "x86",
                         # .NET Framework runs an AnyCPU exe as 64-bit; x86 makes every TFM 32-bit.
                         "dotnet_build_flags": "-p:PlatformTarget=x86"},
}

DOTNET_TFM: dict[str, dict] = {
    "net472": {"copy_native_lib": True},
    "net481": {"copy_native_lib": True},
    "net8.0": {"copy_native_lib": False},
    "net9.0": {"copy_native_lib": False},
    "net10.0": {"copy_native_lib": False},
    "net11.0": {"copy_native_lib": False},
}
