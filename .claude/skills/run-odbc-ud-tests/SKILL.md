---
name: run-odbc-ud-tests
description: >
  Runbook for building and running ODBC Universal Driver (UD) tests. Use
  when you need to compile the ODBC Rust driver, set up the C++ test harness,
  and execute ODBC tests via run.sh or ctest. Also use for: DRIVER_PATH
  errors, cmake/ninja/make build issues, unixodbc/iodbc setup, libsfodbc not
  found, ODBC ctest failures, or run_reference.sh comparison runs.
---

## ODBC Test Runner

All commands run from the **repo root** unless stated otherwise.

---

## Prerequisites

### macOS
```bash
brew install unixodbc cmake ninja ccache
# Optional: for iODBC variant
brew install libiodbc
```

### Linux (Debian/Ubuntu)
```bash
sudo apt-get install -y unixodbc-dev build-essential cmake ninja-build ccache zlib1g-dev
# CMake 4.0+ required:
python3 -m pip install "cmake>=4.0.3"
```

### All platforms
- Rust toolchain with `cargo` on PATH
- Python 3.x (for matrix/schema tooling)

---

## Step 1 — Credentials

Ensure `parameters.json` exists at the repo root and `PARAMETER_PATH` is
exported. Full setup procedure:

@.claude/rules/ud-credentials.md

---

## Step 2 — Build the ODBC driver

```bash
# Debug build (faster, used by run.sh by default):
cargo build --package odbc
# Output: target/debug/libsfodbc.dylib  (macOS)
#         target/debug/libsfodbc.so     (Linux)
#         target/debug/sfodbc.dll       (Windows)

# Release build:
cargo build --release --package odbc
```

`run.sh` calls `cargo build` automatically, so manual pre-build is optional.

---

## Step 3 — Run tests

### Full test suite (recommended)

```bash
./odbc_tests/run.sh
```

This script:
1. Runs `cargo build` (builds `libsfodbc`)
2. Sets `DRIVER_PATH=target/debug/libsfodbc.{so,dylib}`
3. Configures and builds the C++ Catch2 test harness via CMake
4. Pre-creates a shared test schema (`TEMP_TEST_SCHEMA_*`)
5. Runs ctest in parallel (`-j $(nproc)`)
6. Drops the schema on exit

### Discover available tests

After the CMake build has run at least once (i.e. `cmake-build/` exists):

```bash
# List all registered ctest test names — no credentials needed
ctest --test-dir odbc_tests/cmake-build -N
```

ctest test names follow the pattern `<category>_<file>:<Catch2 test case name>`, e.g.:

```
e2e_query_basic_execute_query:should execute simple SELECT returning single value
e2e_authentication_user_password:should authenticate with user/password
```

The `-R` regex matches against the full ctest test name, so you can filter by
Catch2 test case name directly (it's a substring of the full name).

Alternatively, grep the source files:

```bash
grep -r "TEST_CASE" odbc_tests/tests/e2e/ --include="*.cpp" -h | grep -oP '(?<=")[^"]+(?=")'
```

### Filter by test name/pattern

```bash
./odbc_tests/run.sh -R "test_pattern"
# -R is passed directly to ctest as a regex filter
# Pattern matches against full ctest name: <category>_<file>:<Catch2 test case name>
```

### Run against the official reference driver (for comparison)

```bash
./odbc_tests/run_reference.sh
# Builds Docker image with official Snowflake ODBC v3.16.0 and runs same suite
```

### Advanced: run tests directly via ctest (after build)

```bash
cd odbc_tests
ctest -j $(nproc) -C Debug --test-dir cmake-build --output-on-failure
ctest -j $(nproc) -C Debug --test-dir cmake-build -R "test_pattern" --output-on-failure
```

---

## Key environment variables

| Variable | Required | Default | Purpose |
|---|---|---|---|
| `DRIVER_PATH` | Yes (set by run.sh) | — | Path to compiled `libsfodbc.{so,dylib,dll}` |
| `PARAMETER_PATH` | Yes | `$(pwd)/parameters.json` | Snowflake test credentials |
| `DRIVER_TYPE` | Yes (set by run.sh) | `NEW` | Selects UD driver code path |
| `DRIVER_MANAGER` | No | `unixodbc` | Unix only: `unixodbc` or `iodbc` |
| `QUERY_RESULT_FORMAT` | No | Arrow | Set `JSON` to test JSON result format |
| `ODBC_TEST_SCHEMA` | No | auto | Pre-created shared schema name |
| `CTEST_FILTER` | No | — | ctest regex filter (alternative to `-R`) |

---

## Troubleshooting

### `odbc_config` not found

Install unixodbc dev package; `odbc_config` must be on PATH.

```bash
# macOS:  brew install unixodbc
# Linux:  sudo apt-get install unixodbc-dev
```

### CMake cache stale after generator change (Ninja ↔ Make)

```bash
rm -rf odbc_tests/cmake-build
./odbc_tests/run.sh
```

### Rebuild after Rust changes

`run.sh` always calls `cargo build` — just re-run it.
To rebuild only the C++ harness without recompiling Rust:

```bash
cd odbc_tests
cmake --build cmake-build -- -j $(nproc)
ctest -j $(nproc) -C Debug --test-dir cmake-build --output-on-failure
```
