# Universal Driver Logging Guidelines

Knowledge base and source of truth for client-side logging practices across the Rust core and all wrappers.

## High-level principles

- **Access credentials never leave the process.** Tokens, passwords, passcodes, and private keys must never appear in logs, on any level. Query result data (values, schema metadata, rowsets, record batches) must never appear in logs - there is no opt-in. Other sensitive data (e.g. query text, bindings) may appear in logs only when the user explicitly opts in.
- **Redact at the source.** Wrap or redact sensitive values before they enter the logging pipeline - never pass secrets through as plain strings for downstream filtering.
- **Log at boundaries and lifecycle transitions.** Public API entry points, HTTP calls, and exceptions are must-log events. Significant internal lifecycle events - retries, re-authentication, token refresh, connection recovery - should be logged at **INFO** or **WARN** as appropriate.
- **The host application stays in control.** The core forwards logs to the wrapper's own logging system.
- **Default safe, opt in to verbose.** Risky output is off by default and only enabled deliberately, with the trade-off
  made explicit to the end user.

### Logging across core and wrappers

How logs flow from the core into each wrapper is described separately
in [logging-architecture.md](logging-architecture.md).

---

## Log levels

The driver supports **ERROR**, **WARN**, **INFO**, and **DEBUG**:

- **ERROR** - unrecoverable failures and unhandled exceptions.
- **WARN** - handled but notable failures (e.g. retried errors, degraded behavior).
- **INFO** - significant operational events that let a user or support engineer understand what the driver did without turning on DEBUG. Examples: HTTP round-trips, wrapper public API entry/exit, connection lifecycle, authentication steps, retries, token refresh, opt-in query text/parameters.
- **DEBUG** - core API entry/exit, third-party error cause messages, verbose diagnostics.

#### Rules

- `.ai/review/universal-driver-logging-rust.yaml` (`ud-log-core-uses-tracing-not-stdio`) - core emits logs via
  `tracing` (not the `log` crate) and never writes to stdout/stderr (except logging-init failures).

---

## What to never log

The following parameters must never appear in logs, on any log level, in any code path (core or wrappers):

- tokens (`id_token`, `id_token_password`, `master_token`, `mfa_token`, `session_token`, `token`, etc.),
- passcodes (`passcode`, totp, etc.),
- passwords (any!, including `password`, `private_key_file_pwd`, `proxy_password`, etc.)
- private key contents and passwords (`private_key`, `private_key_file_pwd`, etc.),
- HTTP headers that carry credentials or session state, including but not limited to `Authorization`, `Proxy-Authorization`, and `Set-Cookie`,
- URL query strings and fragments (can carry passcodes, passwords, tokens, etc.),
- presigned URLs, stage master keys, and encryption IVs.

Logging a **hash** of a sensitive value is permitted when correlation is needed during debugging.

### What can be logged

The following parameters are safe to be logged:

- **Authentication**:
    - `authenticator` (type of the authenticator used),
    - `oktaUrl` (only public and generic URL, no sensitive identifiers),
    - `oktausername`,
- **Snowflake environment**:
    - `account`,
    - `database`,
    - `host` (`serverUrl`),
    - `protocol`,
    - `role`,
    - `schema`,
    - `tokenValidityTime`,
    - `user`,
    - `warehouse`,
- **User environment**:
    - `nonProxyHosts` (list of the hosts which are not proxied),
    - `passcodeInPassword` (boolean describing if the passcode is in password),
    - `private_key_file` (path to the file),
    - `proxyHost`,
    - `proxyPort`,
    - `proxyProtocol`,
    - `proxyUser`,
    - `useProxy`,
- **Query/session identification**
    - `queryId`,
    - `requestId`,
    - `sessionId`,

### Object serialization

To enforce this at the code level, sensitive values in the core must be wrapped in `SensitiveString`
(rather than plain `String`) so they are redacted in `Debug`/`Display` output and zeroized on drop.

### Rules

- `.ai/review/universal-driver-security.yaml` - enforces wrapping sensitive fields in `SensitiveString`.
- `.ai/review/universal-driver-logging.yaml` (`ud-log-never-log-secrets`) - language-agnostic: flags any
  logging call (core or wrapper) that emits a sensitive parameter's value.
- `.ai/review/universal-driver-logging-rust.yaml` (`ud-log-no-revealed-sensitive-in-macro`) - Rust core: flags
  `.reveal()` / plain sensitive fields interpolated into `tracing` macros.

---

## Query logging

Query text and query parameters are off by default and gated behind dedicated connection parameters:

- **`log_query_text`** (boolean, default `false`) - controls whether the query text is logged.
- **`log_query_parameters`** (boolean, default `false`) - controls whether the query parameters (bindings) are logged.

Rules for both parameters:

- When enabled, query text and parameters should be logged at **INFO** level.
- If a driver supports per-query configuration, the parameter may additionally be configured per query.
- Both must be documented as carrying a security risk of exposing confidential information in logs.
- When either is enabled, the driver must emit a **WARN**-level log message stating this risk.

### Rules

- `.ai/review/universal-driver-logging.yaml` (`ud-log-query-text-and-params-gated`) - enforces that query text
  and parameters are gated behind `log_query_text` / `log_query_parameters`, logged at INFO, with a risk warning.

---

## Query result data

Query result payloads are customer data and must **never** appear in logs - on any log level, in any code path. Unlike [query text and bindings](#query-logging), there is no connection parameter to opt in.

This includes:

- row/cell values from JSON rowsets,
- Arrow record batches and base64 chunk bodies,
- file-transfer (PUT/GET) result bodies,
- result-set schema metadata (column names, types, and related rowtype/field descriptors returned with a query result),
- any serialization of the above (`{rowset:?}`, `batch.to_pydict()`, `{rowtype:?}`, etc.).

Column names that appear in the SQL statement itself, and query parameter/bindings values, are **not** result data - they follow the opt-in rules in [Query logging](#query-logging) (`log_query_text` / `log_query_parameters`).

Safe to log about a result:

- row/column counts,
- `queryId`, chunk count, rowset variant/type,
- lifecycle events ("fetch complete", "chunk download started") without payload.

### Rules

- `.ai/review/universal-driver-logging.yaml` (`ud-log-never-log-client-data`) - flags any logging call that emits query result payloads.

---

## Public API

Public API entry points should be logged on both entry and exit, so a single call can be traced end to end:

- **INFO** - on entering and exiting a public API entry point in the **wrapper**.
- **DEBUG** - on entering and exiting a procedure defined and exposed in the **core**.

> [TODO(SNOW-2881781)]: Define wrapper-level configuration for enabling and controlling public API entry/exit logging - this can be expensive at scale.

> [TODO(SNOW-3725853)]: Should this logging be coupled with telemetry in wrappers?

### Rules

- `.ai/review/universal-driver-logging.yaml` (`ud-log-public-api-entry-and-exit`) - wrapper entry points log
  entry+exit at INFO; exposed core procedures log entry+exit at DEBUG.

---

## Exceptions

All exceptions must be logged - at **ERROR** for unrecoverable failures, **WARN** for handled/retried ones (see [Log levels](#log-levels)).

The exception **type / class name** should always be logged.
How the **message** is handled depends on who authored the exception:

**Exceptions we define (e.g. `snafu` errors in the core):**

- The full message should be logged by default. We control the wording, and secrets are wrapped in `SensitiveString`, so
  the message is safe.

**Internal / underlying causes (errors we did not author, e.g. `reqwest`, OS/TLS libraries):**

- Only the cause's **type / class name** should be logged by default. Some libraries may embed sensitive context in the
  message.
- The raw cause message may be logged at **DEBUG** level, where the extra verbosity is an explicit, opt-in trade-off.

### Stack traces

A stack trace should be logged only for exceptions that propagate out unhandled or terminate a flow.
Exceptions that are caught and handled as part of normal control flow do not need a stack trace.

Only their structure should be logged - the frames (function name, file, line).
Captured **argument or local-variable values** from the frames must never be logged:
some runtimes/tooling can render these, and they may contain secrets (e.g. a `password` argument).

A stack trace should be logged at the same level as the exception it accompanies (WARN/ERROR).

### Error messages

Errors in the core are constructed with `snafu`, giving each error a typed, human-readable message.
When authoring those messages, treat them like any other log line: they must not contain any of the
sensitive data listed in [What to never log](#what-to-never-log) above. The exception is values
wrapped in `SensitiveString` - those may appear in the message because `Debug`/`Display` redacts them
before they reach a log sink.

When a URL is part of an error message, apply the same rule as [HTTP traffic](#http-traffic): log **host and path**, strip query strings and fragments.

### Rules

- `.cursor/rules/rust-error-handling-rules.mdc` - enforces constructing core errors with `snafu`.
- `.ai/review/universal-driver-logging.yaml` (`ud-log-underlying-error-cause-type-only`) - logs only the type
  name of underlying/foreign error causes at the default level; raw message only at DEBUG.
- `.ai/review/universal-driver-logging.yaml` (`ud-log-stack-trace-unhandled-only`) - stack traces only for
  unhandled errors, frames only (no locals/args), at the exception's own level.
- `.ai/review/universal-driver-logging.yaml` (`ud-log-url-in-error-host-and-path`) - error messages containing a
  URL must include the host and path only - query strings and fragments stripped.

---

## HTTP traffic

Every HTTP call made by the driver should be logged on the INFO level.
Log the URL **host and path**. Strip **query strings and fragments** before logging - they can carry tokens, passcodes, and other identifiers.

Every HTTP response code must be logged at the appropriate level, regardless of whether the response is further handled (e.g. retried).

> [TODO(SNOW-3725854)]: To view the HTTP traffic we should expose proper integration with some tools (e.g. mitmproxy).

### Rules

- `.ai/review/universal-driver-logging.yaml` (`ud-log-every-http-call-at-info`) - every outbound HTTP call must
  be logged at INFO (host and path, query strings and fragments stripped); every response code must be logged at the
  appropriate level.

---
