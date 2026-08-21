# Logging & diagnostics (deep-dive notes)

This page holds the **deep-dive** logging facts that aren't in the reference
layer. It does **not** duplicate how to turn logging on — that lives in the
Runbook:

- **Enable driver logs per wrapper** (Python, ODBC, JDBC) →
  [Runbook §1 Troubleshooting logs](../../troubleshooting-runbook.md#1-troubleshooting-logs)
- **Secret redaction** →
  [Runbook §1.4](../../troubleshooting-runbook.md#14-secret-redaction)
- **Connection-diagnosis probe** (DNS/TLS/CRL/proxy/allowlist report) →
  [Runbook §2](../../troubleshooting-runbook.md#2-connection-diagnosis)
- **Route driver logs into the app's own framework** →
  [Runbook §3 Native logging integration](../../troubleshooting-runbook.md#3-native-logging-integration)
- **Logging architecture & guidelines** →
  [docs/logging/logging-architecture.md](../../logging/logging-architecture.md),
  [logging-guidelines.md](../../logging/logging-guidelines.md)

Part of the [troubleshooting deep-dive](../index.md).

---

## `log_path` gates the core file layer (TOML / ODBC)

The core file-logging layer emits nothing unless a log directory is configured.
`build_core_layer()` (`sf_core/src/logging/log_manager.rs`) returns an **empty,
event-discarding layer** when `log_path` is unset **or** logging is disabled —
regardless of the configured level. So on the TOML/ODBC path, a missing
`LogPath`/`path` means **silent no output**, not "default output somewhere".

This applies to the **file** layer only. Python and JDBC do not use it — they
bridge core events into the host logging framework (Python `logging`, JDBC
SLF4J), so their verbosity is controlled there (Runbook §1.1 / §1.3), and the
`log_path` rule does not apply to them.

**If you get no logs:** confirm `LogPath`/`path` is set to an existing, writable
directory before anything else (Runbook §1.2 for the ODBC INI keys).

---

## Grep by target

Core events are tagged with **module-path targets** (`tracing` convention), so you
can slice a captured log by subsystem:

```sh
grep 'sf_core::crl'  sf_driver.log    # CRL fetch / revocation / cache
grep 'sf_core::tls'  sf_driver.log    # handshake, chain, hostname
grep 'sf_core::rest' sf_driver.log    # login / query HTTP calls
grep -E ' (ERROR|WARN) ' sf_driver.log   # quick health scan
```

The CRL/TLS pages lean on `sf_core::crl` in particular — see
[crl-revocation.md](crl-tls/crl-revocation.md).

---

## Correlating a query across log lines

Each query submission carries a stable **request identifier** (a UUID reused
across all its retries) and, once the server assigns it, a **query identifier**.
Grep a single operation by that identifier to see submit → retries → response as
one thread. The identifiers and how they map to the request/response cycle are
documented where the query path is:
[query-execution.md → Request / query correlation](query-execution.md#request--query-correlation).

Capture the query identifier first for any "query failed / wrong result"
investigation, then pivot to the server side with it.

---

## What is never in the logs

- **Credentials** — password, token, private key, and proxy password are wrapped
  in `SensitiveString` (`sf_core/src/sensitive/`) and render as `[REDACTED]` at
  **every** level, including TRACE. Read them from config directly; they will
  never appear in a log (Runbook §1.4).
- **SQL text** — off unless `log_query_text=true`. Opt-in and security-sensitive;
  enabling it writes full SQL to the query log lines. Don't enable in production
  unless the log file is access-controlled.
- **Bind parameter values** — off unless `log_query_parameters=true` (which
  itself requires `log_query_text`). Same caution.

What **is** visible (audit before sharing a log): account name, username, login
URL, Snowflake error codes/messages, and file paths (including
`custom_root_store_path`).

---

## Forward-looking / not-yet-wired

- **TOML `[log]` section** — the keys are parsed by sf_core, but no wrapper calls
  the TOML logging initializer at connect time today (ODBC uses its INI path;
  Python/JDBC use the host-framework sink). Treat `[log]` as forward-compatible,
  not active.
- **`LogMaxSize` / `max_size`** — accepted for forward-compatibility but **not
  enforced**; the driver prints a one-line stderr warning at init and ignores it.
  Use `LogRotation` + `LogMaxCount` for pruning.
- **OpenTelemetry** — **not supported yet.** The TOML key (`opentelemetry = true`)
  and the standard `OTEL_EXPORTER_OTLP_ENDPOINT` / `OTEL_SERVICE_NAME` env vars are
  recognized for forward-compatibility, but tracing is not active and nothing is
  exported yet.

---

Related: [query-execution.md](query-execution.md) ·
[crl-revocation.md](crl-tls/crl-revocation.md) · up to the
[deep-dive index](../index.md) ·
[Runbook §1–§3](../../troubleshooting-runbook.md#1-troubleshooting-logs)
