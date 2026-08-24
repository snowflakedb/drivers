---
description: sf_core, the Rust crate holding all driver behavior — its public module surface, the two boundaries it exposes (protobuf apis and c_api), and where its tests live.
no-pointer: true
---

# `sf_core`

The crate that *is* the driver. Authentication, the REST protocol, result chunk handling, Arrow deserialization, file transfer, logging, and telemetry live here once, and the wrappers share them. A behavior change that should apply to every driver is a change in this crate.

## Layout

`sf_core/src/`, with roughly twenty-five public modules declared in `lib.rs`. The ones worth knowing by name:

| Module | Holds |
|---|---|
| `apis` | The protobuf-facing entry surface a bridge calls. `RustTransport` lives under here. |
| `protobuf` | The generated types for `protobuf/database_driver_v1.proto`. |
| `c_api` | The C-ABI surface, used where a caller links the library directly rather than through a bridge. |
| `auth` (private), `refresh`, `crl` | Authentication, token refresh, and certificate revocation. |
| `rest`, `http`, `tls` | The wire layer. |
| `query_types`, `chunks`, `arrow_utils`, `compression` | Executing a query and decoding its results. |
| `file_manager`, `stage_binding`, `fs_adapter`, `fs_lock` | PUT/GET and stage handling. |
| `config`, `env_vars` | Configuration resolution. |
| `logging`, `telemetry`, `perf_timing`, `diagnostic` | Observability. |
| `sensitive` | Values that must not be logged. |

`crate-type = ["cdylib", "rlib"]`, so it is both a shared library the wrappers load and a Rust dependency the bridges link.

## The two boundaries

- **protobuf** — the normal path. A bridge builds a message and calls into `apis`. See [../../concepts/core-and-bridge-split.md](../../concepts/core-and-bridge-split.md).
- **`c_api`** — a direct C surface. ODBC uses this rather than a bridge crate.

Adding a field a wrapper needs starts at `protobuf/database_driver_v1.proto`, not in the bridge.

## Tests

`sf_core/tests/` holds the crate's integration and e2e suites, and unit tests sit inline behind `#[cfg(test)]`. Both are searched when attributing legacy coverage — see [../../tools/old-driver-coverage-mapping.md](../../tools/old-driver-coverage-mapping.md). CI entry points are `test-rust-core.yml` and `test-rust-core-spcs.yml`.

## Gotchas

- **A stale build is the usual cause of a wrapper test failing to start.** The wrapper loads the compiled `sf_core` artifact by path; if it was not rebuilt the failure reads as a library-load error, not an assertion failure. Each wrapper's `tools/` reference gives its build step and the variable that points at the artifact.
- Error types and their conversions across the boundary follow [../best-practices/rust-error-handling.md](../best-practices/rust-error-handling.md).
- Anything routed through `sensitive` is excluded from logs on purpose; do not add a debug print that bypasses it.
