# ADR: ODBC WireMock E2E Test CI — PR #1024

## Goal

`odbc_tests` and `pre-commit` CI lanes on all platforms (Linux x64, macOS ARM64,
Windows x64) pass without build or formatting errors for the WireMock-based
logout E2E tests.

## Context

- Repo: snowflakedb/universal-driver
- PR / branch: #1024 / SNOW-2872349-odbc-logout-wiremock-test
- Target jobs: odbc_tests (all platforms), odbc_reference_tests, pre-commit
- Runner: ubuntu-latest, macos-14, windows-latest

## Current failure

- Category: build failure
- Error: `error: variable 'std::atomic<int> ready_count' has initializer but incomplete type` / `'thread' is not a member of 'std'`
- Run: 25155305580

## Observations

[2026-04-30] Run 25146120227 — `O_WRONLY` and `open` not declared in scope on Linux.
Source: odbc_tests (Linux x64, aws), WiremockClient.hpp:156

[2026-04-30] Run 25146120227 — `sys/socket.h` not found on Windows (fatal C1083).
Source: odbc_tests (Windows x64, aws), WiremockClient.hpp:5

[2026-04-30] Run 25146120227 — clang-format 18 (CI) reformats `ss << "SERVER=..."` chain differently than clang-format 22 (local).
Source: pre-commit, diff of WiremockClient.hpp lines 206-215

[2026-04-30] Run 25155305580 — After Iteration 1 fix, Windows and pre-commit pass. Linux/macOS fail: `std::atomic<int> ready_count has initializer but incomplete type` and `'thread' is not a member of 'std'`.
Source: odbc_tests (Linux x64, aws), logout.cpp:83,86

## Hypotheses

[H1] [superseded] — Missing `#include <fcntl.h>` for `open()`/`O_WRONLY` — [Observation 1]
Superseded: fixed in Iteration 1, but revealed deeper issue.

[H2] [superseded] — POSIX-only headers (`sys/socket.h`) fail on Windows — [Observation 2]
Superseded: fixed in Iteration 1 with `#ifndef _WIN32`.

[H3] [superseded] — clang-format 18 vs 22 divergence on chained `<<` — [Observation 3]
Superseded: fixed in Iteration 1 with separate `ss <<` statements.

[H4] [current] — When restructuring logout.cpp with `#ifndef _WIN32` guards, `#include <atomic>`, `#include <thread>`, and `#include <vector>` were dropped. These are needed for the concurrent test case. — [Observation 4]

## Iterations

### Iteration 1 — POSIX guard + fcntl + clang-format fix

**Motivation**: Build failures on all three platforms + pre-commit clang-format failure.

**Observation**: Linux: `O_WRONLY not declared`. Windows: `sys/socket.h not found`. Pre-commit: clang-format diff on `<<` chain.

**Hypothesis**: Missing include, missing platform guard, clang-format version divergence.

**Fit**: Each error has a direct 1:1 cause visible in the log.

**Falsification**: If the fix resolves all three error categories, hypothesis confirmed.

**Change**:
- File: `WiremockClient.hpp` — add `#include <fcntl.h>`, wrap entire file in `#ifndef _WIN32`, rewrite `ss <<` as separate statements, check `pclose()` return
- File: `logout.cpp` — add `#ifdef _WIN32` SKIP(), add `CHECK(second_ret == SQL_ERROR)` assertions

**Commit**: e9c031a5

**Observations afterwards**: Windows ODBC tests now PASS. Pre-commit now PASSES. Linux/macOS: NEW failure — `std::atomic<int>` incomplete type, `std::thread` not a member of `std`. Category changed from "missing POSIX header" to "missing standard C++ headers".

**Conclusion**: partially supported — Windows + pre-commit fixed. Linux/macOS have a new build error caused by dropped includes.

**Next step**: Iteration 2 — restore missing includes inside `#ifndef _WIN32`.

### Iteration 2 — Restore missing <atomic>, <thread>, <vector> includes

**Motivation**: Build failure on Linux/macOS — incomplete type `std::atomic<int>`, `std::thread` not a member of `std`.

**Observation**: `logout.cpp:83: error: variable 'std::atomic<int> ready_count' has initializer but incomplete type`. `logout.cpp:86: error: 'thread' is not a member of 'std'`.

**Hypothesis**: When restructuring logout.cpp to add `#ifdef _WIN32` SKIP guards, the `#include <atomic>`, `#include <thread>`, and `#include <vector>` headers were removed. They must be inside the `#ifndef _WIN32` block since they're only used by the concurrent test code within that block.

**Fit**: Direct cause — the includes are absent from the file. The error messages match exactly.

**Falsification**: If restoring these three includes resolves the Linux/macOS build failure, hypothesis confirmed.

**Change**:
- File: `logout.cpp` — add `#include <atomic>`, `#include <thread>`, `#include <vector>` inside the `#ifndef _WIN32` block

**Commit**: 8feedd66

**Observations afterwards**: Build succeeded on all platforms (Linux, macOS, Windows). Tests now RUN but fail at runtime: `SQLSTATE=IM002 NativeError=0 — [unixODBC][Driver Manager]Data source name not found and no default driver specified`. Both TEST_CASEs fail at `connect_to_wiremock()` line 29 (`REQUIRE_ODBC`). Category changed from "build failure" to "test assertion failure (connection)".

**Conclusion**: confirmed — missing includes fixed. But revealed next issue: missing ODBC driver registration.

**Next step**: Iteration 3 — add `configure_driver_string()` to connection string.

### Iteration 3 — Add ODBC driver registration to WireMock connection string

**Motivation**: Test assertion failure — `SQLSTATE=IM002` "Data source name not found and no default driver specified" on Linux and macOS.

**Observation**: `logout.cpp:29: FAILED: REQUIRE_THAT(OdbcResult(ret, dbc), OdbcMatchers::Succeeded())` with expansion `SQL_ERROR [0] SQLSTATE=IM002`. Every other ODBC test calls `configure_driver_string(ss)` from `test_setup.hpp` which prepends `DRIVER={SnowflakeDSIIDriver}` and registers the driver with unixODBC. `get_wiremock_connection_string()` omitted this.

**Hypothesis**: The ODBC Driver Manager needs a `DRIVER=` or `DSN=` directive in the connection string to know which driver library to load. `configure_driver_string()` both registers the driver and adds the directive. Without it, `SQLDriverConnect` returns IM002.

**Fit**: IM002 is the exact SQLSTATE for "driver not found". All other tests call `configure_driver_string`. The WireMock connection string is the only one that skips it.

**Falsification**: If adding `configure_driver_string(ss)` to `get_wiremock_connection_string()` resolves IM002 on Linux/macOS, hypothesis confirmed.

**Change**:
- File: `WiremockClient.hpp` — add `#include "test_setup.hpp"`, call `configure_driver_string(ss)` before appending `SERVER=localhost;...`, change `std::ostringstream` to `std::stringstream` (required by `configure_driver_string` signature)

**Commit**: pending

**Observations afterwards**: pending

**Conclusion**: pending

**Next step**: pending

## Confirmed conclusions

- [H1–H3] POSIX guards, missing `<fcntl.h>`, and clang-format rewrite confirmed by Iteration 1 (Windows + pre-commit green).
- [H4] Missing `<atomic>/<thread>/<vector>` confirmed by Iteration 2 (builds pass on all platforms).

## Deferred items

- Jenkins/Buildkite failures appear to be infrastructure-level issues unrelated to this PR's code changes (user confirmed: ignore Jenkins jobs)
- Python test failures on Windows are pre-existing on the base branch, not caused by ODBC test changes
