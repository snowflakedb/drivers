"""Rust core FIPS smoke-test platform metadata."""

CORE_FIPS_PLATFORM: dict[tuple[str, str], dict] = {
    ("ubuntu", "x64"): {
        # Keep the linker-flagged target separate from pre-flag Ubuntu x64 caches.
        "cache_key": "core-fips-ubuntu-x64-rustflags-v1",
    },
    ("ubuntu", "arm"): {
        "cache_key": "core-fips-ubuntu-arm",
    },
    ("macos", "x64"): {
        "cache_key": "core-fips-macos-x64",
    },
    ("macos", "arm"): {
        "cache_key": "core-fips-macos-arm",
    },
    ("windows", "x64"): {
        "cache_key": "core-fips-windows-x64",
        "runner": "windows-2022",
        "msvc_arch": "x64",
    },
}
