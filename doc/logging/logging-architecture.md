# Universal Driver Logging Architecture

How logs flow from the Rust core into each wrapper. Unlike the logging
guidelines (which are prescriptive), this document is descriptive: it captures
how the logging pipeline is built today and changes when the implementation
does. For the author-facing rules, see [logging-guidelines.md](logging-guidelines.md).

- The core should emit all logs through the `tracing` ecosystem (`tracing` + `tracing-subscriber`).
- The `log` crate is not used.

> [TODO(SNOW-3744959)]: Document text vs structured logging - the core emits structured `tracing` events, but the wrapper bridge (FFI/JNI) forwards flattened text today.

---

## A single, process-wide pipeline

All logging is owned by one `LogManager` (`sf_core/src/logging/log_manager.rs`),
initialized **exactly once per process**, carrying the shared state.

It builds a single `tracing_subscriber::Registry` and installs it as the global default.
The registry fans out to a stack of layers:

- core file layer, via `tracing-appender`, when a log path is configured,
- optional wrapper sink (app sink - `Python CallbackLayer` / `JDBC SFLoggerLayer`),
- OpenTelemetry OTLP layer (disabled by default; enabled via `open_telemetry` in
  `LoggingConfig`),
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
- **JDBC** - a JNI `SFLoggerLayer` forwards events to SLF4J. Core-originated events land on
  `net.snowflake.client.CoreLogger`; wrapper round-trip events carry their originating logger name and
  are delivered onto it (see below).
- **ODBC** - no wrapper sink. The core writes to a file when a log path is configured.

---

## Wrapper logs round-tripping through core

Wrapper logs share the single core pipeline instead of bypassing it. Python and
JDBC use a `CoreLogger` that gates locally, crosses an FFI/JNI boundary carrying
`logger_name`, re-emits on the `sf_wrapper` target, and hands the record back to
the host logging framework. ODBC is Rust-native and emits `tracing` events
directly on the core `LogManager` dispatch.

**Levels.** DEBUG is the finest level every wrapper supports. Outbound, each
`CoreLogger` clamps to DEBUG. Inbound, core wire levels **3 and higher** (DEBUG,
legacy TRACE=4, and Rust `tracing::trace!`) are delivered as DEBUG rather than
dropped, so fine-grained core logs are not lost when the host logger has no
finer level.

### Python

Python wrapper modules use `get_logger(__name__)` (a `CoreLogger`).

The flow for one wrapper log call:

1. `CoreLogger` gates on the stdlib logger's level (`isEnabledFor`) - a filtered
   message never crosses FFI.
2. It sends the record to core via the `sf_core_log_event` FFI call, carrying
   `level`, `message`, `file`, `line`, `function`, and `logger_name` (the
   originating module logger name, e.g. `snowflake.connector.cursor._base`).
3. Core re-emits it as a `tracing` event on the `sf_wrapper` target. With
   Python's default `LoggingConfig` the **file layer is inactive** (`log_path` is
   unset) and the **OTLP layer is disabled** (`open_telemetry: false`), so the
   only layer that handles the event is the `CallbackLayer`. Wrapper log events
   are not sent to in-band telemetry (see below).
4. The `CallbackLayer` hands it back across FFI. For wrapper round-trip events
   `logger_name` is set, so the Python callback rebuilds the record on that
   module logger; core-originated events leave `logger_name` empty and land on
   `snowflake.connector._core`.

**Initialization / fallback.** `sf_core_log_event` returns `0` when the event
was accepted and non-zero when the pipeline is not live yet (before
`sf_core_init`) or unusable (interpreter shutdown). The FFI return code - not a
Python-side flag - is the single source of truth: on any non-zero result
`CoreLogger` emits the record straight onto the stdlib logger, so early-import
records are never lost.

**Python configuration.** Standard `logging` APIs - no special treatment.
`CoreLogger` and the inbound FFI callback both gate on the underlying stdlib
logger's level and handlers. By default `snowflake.connector` and
`snowflake.connector._core` use a `NullHandler` with `propagate=True`, so
`basicConfig`, root handlers, `dictConfig`, and `pytest caplog` work without
extra setup. Configure levels and handlers on those loggers (or any
`snowflake.connector.*` child) as usual.

### JDBC

JDBC funnels every logger through `SFLoggerFactory.getLogger(...)`, which returns
a `CoreLogger` (Java), so all wrapper modules round-trip without touching each
call site.

The flow mirrors Python across JNI instead of the C FFI:

1. `CoreLogger` gates on its delivery logger (`isInfoEnabled`, …) and formats
   the message before crossing JNI.
2. It calls `CoreLoggingBridge.logEvent` (JNI), carrying `level`, the formatted
   `message`, and `logger_name` (the originating Java logger, e.g.
   `net.snowflake.client.api.driver.SnowflakeDriver`). The first call loads
   `jdbc_bridge` via `NativeLibraryLoader`.
3. `jdbc_bridge` re-emits it on the shared `sf_wrapper` target (the same
   `wrapper_event!` macro the C FFI uses), so every core layer sees it.
4. The JNI `SFLoggerLayer` hands it back. Wrapper round-trip events carry a
   `logger_name`, so it delivers through `SFLoggerFactory.getDeliveryLogger` —
   which returns a *plain* JUL or SLF4J logger (never a `CoreLogger`), so a delivered
   record cannot re-enter the round-trip and loop. Core-originated events leave
   `logger_name` empty and land on `net.snowflake.client.CoreLogger`.

**Delivery backend.** `net.snowflake.jdbc.loggerImpl` selects the delivery logger,
defaulting to JUL (`net.snowflake.client.log.JDK14Logger`) for legacy driver
compatibility. Set to `net.snowflake.client.log.SLF4JLogger` to route delivery
through SLF4J instead.

**Initialization / fallback.** `logEvent` returns `0` when accepted and
non-zero when the pipeline is not live yet (before `JNI_OnLoad`); on any non-zero
result — or if the native call throws because the lib is genuinely unavailable —
`CoreLogger` emits straight onto its delivery logger, so records are never lost. A
throw latches the fallback per logger, since a failed native load never recovers
in-process.

**Source location.** Java 8 has no cheap single-frame access (StackWalker is
9+), so JDBC omits `file`/`line`/`function` rather than pay a full stack capture
per log; the record still lands on its originating logger.

### ODBC

ODBC emits `tracing` events directly on the shared core pipeline:

1. The first `SQLAllocHandle(SQL_HANDLE_ENV)` loads `sf.odbc.ini`, calls
   `LogManager::for_odbc()`, and stores the resulting dispatch on `OdbcGlobals`.
2. Every ODBC C API entry point installs that dispatch as the thread-local
   default for the duration of the call (`set_dispatch!` in `c_api.rs`), so
   wrapper `tracing::` output reaches the same layered subscriber as core events.
3. `OdbcGlobals::block_on` and `spawn` also set the dispatch on the thread or
   tokio task handling async work (worker threads do not inherit the caller's dispatch).
4. With `LogPath` configured in `sf.odbc.ini`, the file layer is the sink;
   there is no app sink. Core and wrapper events share one format and filtering policy.

---

## In-band telemetry

In-band telemetry is a **separate channel** from logging. It ships structured
product telemetry (e.g. `session_init`, `api_usage`, `wrapper_error`) to
Snowflake's `/telemetry/send` endpoint over the authenticated session - not
driver debug/info log text.

The `LogManager` installs an OpenTelemetry span exporter when a session registry
is present (always for Python, created at `sf_core_init`). That layer:

- exports **completed spans** tagged with `snowflake.session.id`, not `tracing`
  log events;
- is filtered to the `sf_core` target, so `sf_wrapper` round-trip log events
  never reach it;
- registers a session after login when the server returns
  `CLIENT_TELEMETRY_ENABLED=true` (defaults to enabled when absent);
- POSTs serialized span data using the live session token; spans are flushed on
  connection close while the token is still valid.

Python wrapper code also records telemetry explicitly via `TelemetryClient`
(`@api_telemetry` decorators, wrapper-error reporting) - those RPC calls create
`sf_core` spans that follow the same in-band export path. In-band telemetry is
not configured through Python `logging`; it is automatic and server-gated.

---

## Log filtering

Level filtering is **per output**, not a single global gate. The core owns the
`tracing` subscriber; each layer in the stack applies its own filter independently.

**Core-owned outputs** (file, stderr, OpenTelemetry OTLP, in-band telemetry):

- The **file layer** is active only when `log_path` is set; it honors
  `LoggingConfig.level` (default INFO; set via ODBC INI `LogLevel`). For Python,
  `log_path` is unset by default so the file layer is inactive.
- The **OTLP layer** is off unless `open_telemetry: true` in `LoggingConfig`
  (disabled by default for Python).
- **In-band telemetry** exports `sf_core` spans only; it does not receive
  wrapper log events (see "In-band telemetry" above).
- Other core layers have their own filters (e.g. stderr accepts ERROR only).

**Wrapper sink** (Python `CallbackLayer`, JDBC `SFLoggerLayer`):

- The core invokes the wrapper callback for each tracing event that reaches this layer.
- The **wrapper** applies its own level and routing rules before anything reaches the host
  application (Python `isEnabledFor`, JDBC JUL/SLF4J config).

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