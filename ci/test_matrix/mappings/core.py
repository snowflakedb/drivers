"""
Rust core (sf_core) mapping table.

Single source of truth for every (OS, Arch) → core-specific lookup the
consolidated rust-core test job in test-rust-core.yml needs. Adding a new
lane is one row in CORE_PLATFORM (plus a matching entry in core.pict and
the relevant `if:` gates in the workflow).

Per-row keys:
  cargo_flags  (str, required)  Cargo feature flags interpolated into
                                cargo build/test/llvm-cov. Empty string means
                                "use crate defaults" (Windows ARM64 — FIPS
                                disabled upstream; see test-rust-core.yml
                                comments referencing aws-lc-rs#1057).
  coverage     (bool, required) When true, the consolidated `test` job runs
                                cargo llvm-cov (build+run+report) and uses
                                slim-mode: aggressive on cache save. When
                                false, runs plain cargo build / cargo test.
  cache_key    (str, required)  shared-key passed to the cargo-cache action.
                                Identical to the pre-consolidation values
                                (core-test, arm64-nonfips, x86-core-test) so
                                warm caches survive across the refactor.
  cargo_target (str, optional)  Rust target triple for cross-compilation.
                                Absent on host-native cells; presence gates
                                the `rustup target add` step.
  msvc_arch    (str, optional)  Passed to vcvarsall.bat <arch>. Acts as the
                                gating field for Windows-specific steps.
"""

CORE_PLATFORM: dict[tuple[str, str], dict] = {
    ("ubuntu",  "x64"): {
        "cargo_flags": "--all-features",
        "coverage": True,
        "cache_key": "core-test",
    },
    ("macos",   "arm"): {
        "cargo_flags": "--all-features",
        "coverage": True,
        "cache_key": "core-test",
    },
    ("windows", "arm"): {
        "cargo_flags": "",
        "cargo_target": "aarch64-pc-windows-msvc",
        "msvc_arch": "arm64",
        "coverage": False,
        "cache_key": "arm64-nonfips",
    },
    ("windows", "x86"): {
        "cargo_flags": "--no-default-features --features protobuf,vendored-openssl",
        "cargo_target": "i686-pc-windows-msvc",
        "msvc_arch": "x86",
        "coverage": False,
        "cache_key": "x86-core-test",
    },
}
