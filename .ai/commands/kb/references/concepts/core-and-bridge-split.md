---
description: How a driver call crosses from a language wrapper into the shared Rust core — the protobuf contract in database_driver_v1.proto, and the per-wrapper FFI mechanism (JNI, PyO3, N-API) that carries it.
no-pointer: true
---

# The Core-and-Bridge Split

Driver logic lives once, in `sf_core`. A language wrapper holds no protocol behavior: it converts its language's types into a protobuf message, hands that across an FFI boundary to `sf_core`, and converts the response back. Changing behavior for every driver at once means changing `sf_core`; changing how one language expresses that behavior means changing its wrapper.

## How it works

Three layers, in call order:

1. **The wrapper** (`jdbc/`, `python/`, `nodejs/`, `dotnet/`) — idiomatic API in the host language. `gosnowflake/` is a placeholder and implements none of this.
2. **The bridge crate** (`jdbc_bridge`, `python_bridge`, `nodejs_bridge`) — a Rust `cdylib` the host runtime loads. It owns the FFI surface and nothing else.
3. **`sf_core`** — the driver. It receives protobuf, not host-language values.

The contract between layers 2 and 3 is `sf_core::protobuf::apis`, generated from the single schema `protobuf/database_driver_v1.proto`. `RustTransport` is the entry point a bridge calls; `DriverProviders` and `WrapperPresets` in `sf_core::protobuf::apis::database_driver_v1` are how a bridge declares which wrapper it is and which defaults apply to it.

`odbc` has no separate bridge crate — it is itself a Rust workspace member that links `sf_core` directly, because the ODBC ABI is already C.

## What varies per wrapper

| Wrapper | Bridge crate | FFI mechanism |
|---|---|---|
| JDBC | `jdbc_bridge` | `jni` — `JNI_OnLoad`, `extern "system"` entry points |
| Python | `python_bridge` | `pyo3` with the `extension-module` feature, plus `pyo3-async-runtimes` on Tokio |
| Node.js | `nodejs_bridge` | `napi` v3 (`napi9` ABI), built via `napi-build` |
| ODBC | none | links `sf_core` directly; the C ABI is the boundary |

## Constraints

- A wrapper that needs new data from the core needs a change to `protobuf/database_driver_v1.proto` first. Adding the field to one bridge alone does not reach the core.
- A bridge is a `cdylib` loaded by the host runtime, so the wrapper's test setup must be able to find the built artifact. Each wrapper exposes that as its own environment variable — `CORE_PATH` for JDBC, `DRIVER_PATH` for ODBC — and a missing or stale build surfaces as a load failure rather than a test failure. The per-wrapper build and test procedure is in [../tools/README.md](../tools/README.md).
- Behavior that intentionally differs from the legacy driver for one wrapper is recorded per wrapper, not in the core; see [behavior-differences.md](behavior-differences.md).
