# Universal Driver Logging Architecture

How logs flow from the Rust core into each wrapper. Unlike the logging
guidelines (which are prescriptive), this document is descriptive: it captures
how the logging pipeline is built today and changes when the implementation
does. For the author-facing rules, see [logging-guidelines.md](logging-guidelines.md).

- The core should emit all logs through the `tracing` ecosystem (`tracing` + `tracing-subscriber`).
- The `log` crate is not used.

> [TODO(SNOW-3744959)]: Document text vs structured logging - the core emits structured `tracing` events, but the wrapper bridge (FFI/JNI) forwards flattened text today.

> [TODO(SNOW-3725848)]: All logs should go through core (i.e. even if we log from wrapper directly logs go round-trip)

---

## A single, process-wide pipeline

All logging is owned by one `LogManager` (`sf_core/src/logging/log_manager.rs`),
initialized **exactly once per process**, carrying the shared state.

It builds a single `tracing_subscriber::Registry` and installs it as the global default.
The registry fans out to a stack of layers:

- core file layer, via `tracing-appender`, when a log path is configured,
- optional wrapper sink (app sink - `Python CallbackLayer` / `JDBC SFLoggerLayer`),
- OpenTelemetry OTLP layer (when enabled),
- Snowflake in-band telemetry layer (when a session registry is present),
- optional ERROR-only stderr layer,
- a feature-gated perf-timing layer.

---

## Bridging core logs into the wrapper

The core does not write logs to stdout/stderr.
Instead, an "app sink" layer hands each record to the wrapper's own logging system,
so core and wrapper logs share one pipeline that the host application controls.

- **Python** - at init (`sf_core_init`), the wrapper registers a C callback (`CLogCallback`.
  Each event crosses FFI with its level, message, file, line, and function, and the Python side rebuilds a native
  `logging` record on the `snowflake.connector._core` logger.
  *This is the reference pattern for wrapper integration.*
- **JDBC** - a JNI `SFLoggerLayer` forwards events to SLF4J (`com.snowflake.jdbc.CoreLogger`).
- **ODBC** - no wrapper sink. The core writes to a file when a log path is configured.

---

## Log filtering

Level filtering is **per output**, not a single global gate. The core owns the
`tracing` subscriber; each layer in the stack applies its own filter independently.

**Core-owned outputs** (file, stderr, OpenTelemetry, in-band telemetry):

- The **file layer** honors `LoggingConfig.level` (default INFO; set via ODBC INI `LogLevel`).
- Other core layers have their own filters (e.g. stderr accepts ERROR only).

**Wrapper sink** (Python `CallbackLayer`, JDBC `SFLoggerLayer`):

- The core invokes the wrapper callback for each tracing event that reaches this layer.
- The **wrapper** applies its own level and routing rules before anything reaches the host
  application (Python `isEnabledFor`, JDBC SLF4J config).

These filters are independent: tightening the file-layer level does not reduce what the
wrapper sink receives, and wrapper log config does not affect core file output.

> [TODO(SNOW-3744966)]: Apply level filtering in core before the wrapper callback - today every event is formatted and crosses FFI before the wrapper drops it.

---

## stdout / stderr

> Note: The core never writes to stdout. It writes to **stderr only** when the
> stderr layer is explicitly enabled (off by default), and even then only at
> ERROR level - so by default nothing reaches the console and all output flows
> through the configured sinks (wrapper or file).
>
> The sole exception is logging-initialization failures: they occur before the
> pipeline exists, so they are reported directly via `eprintln!`.

---