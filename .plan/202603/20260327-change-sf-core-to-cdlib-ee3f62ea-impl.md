# Implementation Plan

| Field | Value |
|-------|-------|
| Task | Change sf_core rust crate to cdlib. Compare size of release binary of sf_core after the change |
| Date | 2026-03-27 |
| Agent | task-ee3f62ea |
| Repository | snowflakedb/universal-driver |
| PRs | 1 |

## Overview

The change is a single-line edit in one file: switching `crate-type` from `["dylib", "rlib"]` to `["cdylib", "rlib"]` in `sf_core/Cargo.toml`. The output artifact filename (`libsf_core.so` / `sf_core.dll`) does not change, so no consumers (Python ctypes, CI scripts, dependent Rust crates using `rlib`) need updating. The binary size comparison is an informational task documented in the PR. Total diff: 1 line.

## PR Stack

### PR 1: Change sf_core crate-type from dylib to cdylib

**Description**: ## Summary

- Changes `sf_core`'s `crate-type` from `["dylib", "rlib"]` to `["cdylib", "rlib"]` in `sf_core/Cargo.toml`
- `cdylib` is the correct Rust crate type for C-ABI dynamic libraries intended for FFI consumption (Python ctypes, ODBC, JDBC); `sf_mini_core` already uses this type as the established pattern
- `dylib` exports all Rust public symbols and Rust-specific metadata; `cdylib` only exports `#[no_mangle] extern "C"` symbols, which matches what the library actually exposes
- Documents the release binary size difference between `dylib` and `cdylib` builds

## Binary Size Comparison

| Artifact | crate-type | Size |
|---|---|---|
| `libsf_core.so` (before) | `dylib` | _fill in_ |
| `libsf_core.so` (after) | `cdylib` | _fill in_ |

_Expected: `cdylib` produces a smaller artifact because it strips Rust-specific symbol exports and ABI metadata, keeping only the `extern "C"` surface._

## Test plan

- [ ] `cargo build --release --package sf_core` succeeds after the change
- [ ] `cargo test --package sf_core` passes
- [ ] `cargo check --workspace` passes (dependent crates `odbc`, `jdbc_bridge` use the `rlib` form and are unaffected)
- [ ] Record `ls -lh target/release/libsf_core.so` before and after; fill in the table above

🤖 Generated with [Claude Code](https://claude.com/claude-code)

**Scope**:
Modify exactly one file: **`sf_core/Cargo.toml`**

Change line 7 from:
```toml
crate-type = ["dylib", "rlib"]
```
to:
```toml
crate-type = ["cdylib", "rlib"]
```

No other files need to change:
- The output filename (`libsf_core.so` on Linux, `sf_core.dll` on Windows) is determined by the crate `name`, not the `crate-type`, so it stays the same.
- Rust crates that depend on `sf_core` (e.g. `odbc`, `jdbc_bridge`) link against the `rlib` form, which is unaffected.
- Python ctypes loads the library by filename; the filename does not change.
- CI scripts reference `--package sf_core` and artifact names like `libsf_core.so`; these remain valid.

For the binary size comparison, perform the following steps **before committing the change**:
1. `cargo build --release --package sf_core`
2. Record: `ls -lh target/release/libsf_core.so` (Linux) or `ls -lh target/release/sf_core.dll` (Windows)

Then make the one-line change, rebuild, and record the new size:
3. `cargo build --release --package sf_core`
4. Record: `ls -lh target/release/libsf_core.so`

Add both measurements to the PR description table.

Following the pattern already established by `sf_mini_core/Cargo.toml` (line 8: `crate-type = ["cdylib", "rlib"]`).

**Rationale**: The entire task is a one-line change in a single file. No splitting is needed or appropriate; the diff is trivially small and all changes are logically inseparable. The binary comparison is informational and belongs in the PR description, not in a separate PR.
