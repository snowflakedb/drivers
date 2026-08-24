---
description: The per-language drivers this repo ships — each wrapper's directory, build toolchain, the release artifact it produces, and which have a release pipeline versus tests only.
no-pointer: true
---

# The Shipped Drivers

Six wrapper directories sit on top of `sf_core`. They are at different stages: three have a release pipeline, two are tested but not yet released from here, and one is a placeholder. Read the status column before assuming a wrapper ships.

| Wrapper | Directory | Build toolchain | Release artifact | Status |
|---|---|---|---|---|
| JDBC | `jdbc/` | Gradle (`build.gradle`, `./gradlew`) | Jars, plus a fat jar | Released — `build-jdbc-jars.yml`, `_build-jdbc-fatjar.yml` |
| ODBC | `odbc/` (Rust crate) + `odbc_tests/` | cargo, then the packaging images under `ci/` | OS packages (rpm, deb, and the `libsfodbc` shared library) | Released — `build-odbc-packages.yml` |
| Python | `python/` | `pyproject.toml` with hatch, and maturin for the native module | Wheels | Released — `build-python-release.yml`, `_build-python-wheels.yml` |
| Node.js | `nodejs/` | npm (`package.json`), Vitest for tests | npm package | Tested only — `test-nodejs.yml`; no release workflow in this repo |
| .NET | `dotnet/` | MSBuild (`Snowflake.Data.sln`, two projects: `Snowflake.Data` and `Snowflake.Data.Proto`) | — | Tested only — `test-dotnet.yml`; no release workflow in this repo |
| Go | `gosnowflake/` | `go.mod` | — | **Placeholder.** `driver.go` is a five-line `hello()` stub; no build or test workflow references it, only `labeler.yml` and `reviewers.yml` |

## How a wrapper relates to the core

A wrapper holds no protocol behavior. It converts host-language values into protobuf and calls its bridge crate, which calls `sf_core`. The mechanism and the contract are in [../../concepts/core-and-bridge-split.md](../../concepts/core-and-bridge-split.md); the bridge crates are in [../libraries/bridges.md](../libraries/bridges.md).

ODBC is the exception: it links `sf_core` directly with no bridge crate, since the ODBC ABI is already C.

## Building and testing one

Each wrapper builds with its own toolchain from its own directory, so there is no repo-wide build command. The runnable procedures live in `tools/`:

- [../../tools/jdbc-ud-tests.md](../../tools/jdbc-ud-tests.md)
- [../../tools/odbc-ud-tests.md](../../tools/odbc-ud-tests.md)
- [../../tools/python-ud-tests.md](../../tools/python-ud-tests.md)
- [../../tools/nodejs-ud-tests.md](../../tools/nodejs-ud-tests.md)

.NET and Go have no `tools/` reference; use `test-dotnet.yml` as the reference invocation for .NET.

## Gotchas

- **A wrapper's tests need its bridge built first.** The wrapper locates the built artifact through an environment variable that differs per wrapper (`CORE_PATH` for JDBC, `DRIVER_PATH` for ODBC), and a missing or stale build surfaces as a library-load error rather than a test failure.
- **`odbc_trace_tool/`** is a separate binary crate, not part of a wrapper. Its usage is in [../../tools/odbc-trace-replay.md](../../tools/odbc-trace-replay.md).
- Each released wrapper keeps its own `CHANGELOG.md` and its own `BehaviorDifferences.yaml`; see [../../concepts/behavior-differences.md](../../concepts/behavior-differences.md).
