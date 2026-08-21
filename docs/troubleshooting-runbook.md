# Snowflake's Universal Core Architecture — Troubleshooting Runbook

How to diagnose a problem with Snowflake's Universal Core Architecture across
the Python, ODBC and JDBC wrappers and the shared Rust core (`sf_core`).

| Section | Use it for |
|---------|-----------|
| [1. Troubleshooting logs](#1-troubleshooting-logs) | Capture all-level driver logs to a file, no code change. **Start here.** |
| [2. Connection diagnosis](#2-connection-diagnosis) | DNS / TLS / CRL / proxy / allowlist probe report for connectivity issues. |
| [3. Native logging integration](#3-native-logging-integration) | Route driver logs into the app's own logging framework. |
| [4. Configurations supported](#4-configurations-supported) | Where each setting can be set: `.toml`, `.ini`, env vars, connection params. |
| [Appendix A](#appendix-a--symptom--action) | Symptom → page lookup — pointer to the keyword index. |
| [Deep-dive pages ↓](troubleshooting/index.md) | Subsystem internals below this runbook: auth, TLS/CRL, query execution, stage transfers, PrivateLink, and per-wrapper (Python / JDBC / ODBC / Node.js / .NET) pages. |
| [Keyword → page index ↓](troubleshooting/keyword-index.md) | **Agents:** match an exact error string, code, or symptom keyword to the owning deep-dive page. |

---

## Using this runbook with an AI agent

This runbook is written to be driven by an AI coding agent (e.g. Claude Code) with
the driver source checked out, though the same structure works for a human reading
top-down. A useful starting prompt:

> Using `docs/troubleshooting-runbook.md` and the deep-dive pages it links, help me
> diagnose a Universal Core issue. Wrapper + version: **snowflake-connector-python
> 4.x**. Symptom: on connect I get `<paste the exact error string / code>`, from
> behind a corporate proxy. First tell me which capture to collect (troubleshooting
> logs §1 and/or the connection diagnostic §2); then use the keyword index to find
> the owning deep-dive page and walk me through its symptom → cause → fix. Cite the
> source paths you rely on, and don't guess when a log or the diagnostic would
> settle it.

Give the agent three things up front: the **exact** error string or code, the
**wrapper and version**, and the **auth method** (plus whether a proxy or
PrivateLink is involved). It should start from the capture-first workflow
([§1](#1-troubleshooting-logs) / [§2](#2-connection-diagnosis)) and route via the
[keyword index](troubleshooting/keyword-index.md) rather than pattern-matching on
the symptom alone.

---

## 1. Troubleshooting logs

Writes **all** log events (core + wrapper, every level) to a file, independent of
the wrapper's own log configuration and with no application code change. This is
the recommended way to collect logs for a ticket.

Two environment variables, read **once** at driver init:

| Variable | Default | Notes |
|----------|---------|-------|
| `SNOWFLAKE_TROUBLESHOOTING_ENABLED` | off | `true` / `1` / `yes` / `on` enables. |
| `SNOWFLAKE_TROUBLESHOOTING_REPORT_PATH` | current working directory | Created if missing. |

Output: `<path>/sf_driver_troubleshooting.log` — single file, no rotation, mode
`0600` on Unix. If the file already exists, subsequent runs **append** to it
(it is not truncated or rotated). There is no connection parameter and no
runtime toggle; enabling it requires a process restart.

### 1.1 Python

```bash
export SNOWFLAKE_TROUBLESHOOTING_ENABLED=true
export SNOWFLAKE_TROUBLESHOOTING_REPORT_PATH=/tmp/sfdiag
python your_repro.py
```

Collect `/tmp/sfdiag/sf_driver_troubleshooting.log`.

### 1.2 ODBC

```bash
export SNOWFLAKE_TROUBLESHOOTING_ENABLED=true
export SNOWFLAKE_TROUBLESHOOTING_REPORT_PATH=/tmp/sfodbc
isql -v <dsn>          # or start the ODBC application
```

Collect `/tmp/sfodbc/sf_driver_troubleshooting.log`.

### 1.3 JDBC

Set the variables in the JVM's environment before launch (they are read by the
native core at init, so `-D` system properties do **not** work):

```bash
export SNOWFLAKE_TROUBLESHOOTING_ENABLED=true
export SNOWFLAKE_TROUBLESHOOTING_REPORT_PATH=/tmp/sfjdbc
java -jar your-app.jar
```

Collect `/tmp/sfjdbc/sf_driver_troubleshooting.log`.

### 1.4 Secret redaction

Secrets use a `SensitiveString` type that renders as `***` and is zeroized on
drop. Tokens, passwords, private keys, passcodes, `Authorization` /
`Proxy-Authorization` / `Set-Cookie` headers, presigned URLs, and URL query
strings are never logged. Account, host, role, database, schema, warehouse,
`queryId`, `requestId`, and `sessionId` are safe to log. SQL text and bound
parameters are off by default
([§3.5](#35-query-text--parameters-sensitive--off-by-default)). Still review
logs before sharing externally.

---

## 2. Connection diagnosis

A connectivity probe built into the core (`sf_core/src/diagnostic/mod.rs`). Run
it for connection issues (dropped connections, TLS/cert errors, timeouts,
proxy/PrivateLink). For query or stage issues it comes back clean — skip it and
rely on [§1](#1-troubleshooting-logs).

Three connection parameters, in every wrapper:

| Canonical name | Default | Purpose |
|----------------|---------|---------|
| `enable_connection_diag` | `false` | Master switch; runs the diagnostic during connect. |
| `connection_diag_log_path` | system temp dir | Directory for `SnowflakeConnectionTestReport.txt`. |
| `connection_diag_allowlist_path` | *(none — fetches live)* | Pre-fetched `system$allowlist()` JSON; lets you diagnose even when connect fails. |

`connection_diag_log_path` must be an **existing absolute** directory; otherwise
the core falls back to the system temp dir (`/tmp`, `%TEMP%`). The report
filename is always `SnowflakeConnectionTestReport.txt`. The same parameters can
be set in a `connections.toml` profile — see
[§4.1](#41-connectionstoml--configtoml).

**Trust store.** By default the driver uses the OS root certificate store. To
override it, set `custom_root_store_path` to a PEM file of trusted roots (also
accepted as `TLS_CUSTOM_ROOT_STORE_PATH` / `tls_custom_root_store_path`) — this
**replaces** the system roots for that connection. To verify what you trust:
inspect/list the PEM for a custom store, or use the OS trust-store tools for the
system store. The diagnosis report ([§2.5](#25-reading-the-report)) prints the
**server** certificate chain (subject, issuer, validity) so you can check it
against the expected CA.

### 2.1 Python

```python
conn = snowflake.connector.connect(
    account="myaccount", user="me", password="…",
    enable_connection_diag=True,
    connection_diag_log_path="/tmp/sfdiag",                    # optional
    connection_diag_allowlist_path="/tmp/allowlist.json",      # optional
)
```

### 2.2 ODBC

Uppercase the canonical names in the connection string:

```
Driver=<snowflake-ud-driver>;SERVER=…;UID=…;PWD=…;\
ENABLE_CONNECTION_DIAG=true;\
CONNECTION_DIAG_LOG_PATH=/tmp/sfdiag;\
CONNECTION_DIAG_ALLOWLIST_PATH=/tmp/allowlist.json
```

The keys have no explicit mapping in `normalize_connection_string_options`, so
they take the generic uppercase passthrough and are resolved to their canonical
lowercase names by the core param registry.

### 2.3 JDBC

Pass the canonical names as connection properties or JDBC-URL parameters — both
are merged by `ConnectionOptionsResolver` and then normalized:

```java
Properties props = new Properties();
props.put("user", "me");
props.put("password", "…");
props.put("enable_connection_diag", "true");
props.put("connection_diag_log_path", "/tmp/sfdiag");
DriverManager.getConnection("jdbc:snowflake://myaccount.snowflakecomputing.com", props);
```

Legacy old-driver aliases are accepted (`ParameterKeyNormalizer`):
`enableDiagnostics` → `enable_connection_diag`, `diagnosticsAllowlistFile` →
`connection_diag_allowlist_path`.

### 2.4 What it checks

For the account host and **every entry** returned by `system$allowlist()`
(STAGE, GATEWAYS, HIVE, PROXY, OCSP cache, …):

| Check | Detail |
|-------|--------|
| **DNS resolution** | Resolves the host; classifies each IP public or private. |
| **PrivateLink sanity** | Flags a `*.privatelink.*` host resolving to a **public** IP. |
| **Connected peer IP** | The IP actually connected to after TCP connect, not just the DNS answer. |
| **TLS certificate chain** | Per cert: serial (hex), subject, issuer, validity window, `crt.sh` link. |
| **CRL distribution points** | Downloads and parses each cert's CRL; each CRL URL fetched at most once per run. |
| **Allowlist reachability** | Dispatches on port: **443** → TLS handshake, **80** → HTTP GET, other → raw TCP. |
| **Proxy honoring** | Proxy used on all ports (CONNECT tunnel for 443/other, absolute-form GET for 80). Reports system vs env-var proxies. |
| **HTTP status** | Success = `{200, 301, 302, 307, 308, 400, 403}`. Empty/unparseable bodies still count as success. |

> **Not checked: OCSP.** The core does revocation via **CRL**, not OCSP — an
> intentional divergence from the old drivers.

The diagnostic runs in two phases: **pre-connect** (proxy detection) and
**post-connect** (allowlist probing). If the connection itself fails it falls
back to `connection_diag_allowlist_path` when one was supplied.

### 2.5 Reading the report

```
=========Connectivity diagnostic report================================
INITIAL           → account, resolved host, allowlist-load errors
=========Proxy information==============================================
PROXY             → system proxies (env removed) vs env proxies
=========Snowflake URL information======================================
SNOWFLAKE_URL     → nslookup (public/private), connected peer IP,
                    certificate chain, CRL DP + fetch results
=========Snowflake Stage information====================================
STAGE / GATEWAYS / … → per allowlist entry: "URL Check: Connected
                    Successfully" or "Failed: <reason>", plus TLS + CRL
```

| Finding | Means |
|---------|-------|
| `Failed:` on a `SNOWFLAKE_URL` entry | Account host unreachable — DNS, firewall, or TLS interception. Compare the connected peer IP to what you expect. |
| `.privatelink.` host → public IP | PrivateLink DNS not in effect; the client is bypassing the private endpoint. |
| CRL fetch failures | CRL distribution point blocked by firewall/proxy; TLS validation fails under strict CRL checking. |
| `STAGE`/`GATEWAYS` fails, `SNOWFLAKE_URL` succeeds | Login works but cloud-storage traffic is blocked — partial allowlist. |

> **Read the report file, not the logs.** The diagnostic runs on a
> `spawn_blocking` thread that does not inherit the thread-local tracing
> subscriber the Python/JDBC FFI installs, so its `WARN` (path fallback) and
> `DEBUG` (full report) events never reach the wrapper logger. They *do* appear
> in the core's own file layer — ODBC logs and troubleshooting logs ([§1](#1-troubleshooting-logs)).

---

## 3. Native logging integration

Use this to route driver logs into the application's own logging framework. For
support-ticket log collection prefer troubleshooting mode ([§1](#1-troubleshooting-logs)) — it needs no
code change and cannot be suppressed by wrapper log configuration.

Core level vocabulary: `OFF`, `ERROR`, `WARN`, `INFO` (default), `DEBUG`,
`TRACE`.

### 3.1 Python

Two stdlib loggers, created at import with a `NullHandler` and propagation left
**on**:

| Logger | Carries |
|--------|---------|
| `snowflake.connector` | Wrapper-level events. |
| `snowflake.connector._core` | Events from the Rust core, delivered over the FFI callback. Child of the above; inherits its level. |

```python
import logging
logging.basicConfig(level=logging.DEBUG)
logging.getLogger("snowflake.connector").setLevel(logging.DEBUG)
```

The wrapper-side logger level is the real gate for core events — the core's own
`config.level` only filters the core *file* layer ([logging architecture](logging/logging-architecture.md#log-filtering)). `config.toml [log]`
does **not** apply to Python.

### 3.2 ODBC

The core writes its own log file, configured via `sf.odbc.ini` ([§4.2](#42-sfodbcini-odbc-only)).
Recognised keys (case-insensitive):

| Key | Values | Notes |
|-----|--------|-------|
| `LogEnabled` | bool | Master switch for the core file layer. |
| `LogLevel` | `OFF`…`TRACE` | Gates the core file layer. |
| `LogPath` | directory | Where the log file is written. |
| `LogFile` | filename | Log file name. |
| `LogRotation` | `NEVER`,`DAILY`,`HOURLY`,`MINUTELY` | Default `NEVER`. |
| `LogMaxCount` | integer | Rotated files to keep (forces `DAILY` if rotation is `NEVER`). |
| `LogMaxSize` | bytes | **Not yet enforced** — warns and is ignored. |
| `LogQueryText` | bool | Logs SQL text — see [§3.5](#35-query-text--parameters-sensitive--off-by-default). |
| `LogQueryParameters` | bool | Logs bound params — see [§3.5](#35-query-text--parameters-sensitive--off-by-default). |
| `ErrorTraceEnabled` | bool | Include error traces; default on. |

Keys outside this set are silently ignored by the logging loader; a recognised
key with an invalid value is a hard parse error.

### 3.3 JDBC

Delivery logger is selected by the `net.snowflake.jdbc.loggerImpl` system
property; default is JUL.

| Value | Backend |
|-------|---------|
| `net.snowflake.client.log.JDK14Logger` | `java.util.logging` (default) |
| `net.snowflake.client.log.SLF4JLogger` | SLF4J — binds to the app's Logback/Log4j |

**SLF4J:** set the property and configure the level on the
`net.snowflake.client` logger in the app's own SLF4J configuration.

**JUL:** either supply `-Djava.util.logging.config.file=…` (takes precedence and
disables the bootstrap below), or set the `TRACING` connection property to a JUL
level to get file logging at `$HOME/snowflake_jdbc%u.log`:

```
jdbc:snowflake://myaccount.snowflakecomputing.com/?TRACING=ALL
```

### 3.4 What each level buys you

Applies to the native per-wrapper configuration above. Troubleshooting mode
([§1](#1-troubleshooting-logs)) always captures every level regardless.

- `INFO` — connection lifecycle, each HTTP call (host + path only; query
  strings stripped), response codes, auth steps, retries.
- `DEBUG` — core API entry/exit, verbose diagnostics. The full connection
  diagnostic report is logged at `DEBUG` **only in the core file layer**, i.e.
  ODBC and troubleshooting logs ([§2.5](#25-reading-the-report)).
- `TRACE` — finest-grained internal tracing; core-file-layer vocabulary. Over
  the Python FFI bridge it is delivered as `DEBUG` (finest level the bridge
  encodes).

### 3.5 Query text & parameters (sensitive — off by default)

`log_query_text` / `log_query_parameters` (core config fields; `LogQueryText` /
`LogQueryParameters` as ODBC INI keys) log SQL and bound values. **Both default
to `false`** and emit a `WARN` when enabled. Turn them on only in a controlled
repro and scrub before attaching logs to a ticket.

---

## 4. Configurations supported

| Source | Scope | Wrappers |
|--------|-------|----------|
| Connection parameters (kwargs / connection string / DSN / `Properties`) | Per connection | all |
| `connections.toml` / `config.toml` | Per named profile | all ([§4.1](#41-connectionstoml--configtoml)) |
| `sf.odbc.ini` | Process-wide logging | ODBC only ([§4.2](#42-sfodbcini-odbc-only)) |
| Environment variables | Process-wide | all ([§4.3](#43-environment-variables)) |

**Precedence:** explicit connection parameters > `connections.toml [name]` >
`config.toml [connections.name]` > SPCS env vars > registry defaults.

### 4.1 `connections.toml` / `config.toml`

Any registered connection parameter can be set in a profile — the TOML loader
passes registered keys through without an allowlist.

```toml
# ~/.snowflake/connections.toml   (dir overridable via $SNOWFLAKE_HOME)
[production]
account = "myaccount"
user    = "myuser"
enable_connection_diag         = true
connection_diag_log_path       = "/var/log/sfdiag"
connection_diag_allowlist_path = "/var/snowflake/allowlist.json"   # optional
```

The profile is merged only when (`sf_core/src/config/resolver.rs`):

1. the app connects with `connection_name = "production"`, **or**
2. the app makes a **bare** connect (no connection parameters at all) — then the
   *default* profile is loaded, its name resolved from
   `SNOWFLAKE_DEFAULT_CONNECTION_NAME` → `config.toml`'s
   `default_connection_name` → literal `"default"`.

If the app passes explicit `account` / `user` / `password` **without**
`connection_name`, no profile is loaded and this route cannot inject a setting —
a small code change (add `connection_name`, or the parameter itself) is required.

On Unix the TOML files should be `chmod 600`. Group/other **writable** (`0o022`)
is a hard error; group/other **readable** (`0o044`) only warns — suppressible
via `SF_SKIP_WARNING_FOR_READ_PERMISSIONS_ON_CONFIG_FILE`, or bypass the whole
check with `unsafe_skip_file_permissions_check`.

### 4.2 `sf.odbc.ini` (ODBC only)

Discovery order — first existing file wins (`odbc/src/api/ini_paths.rs`):

1. `$SF_ODBC_INI` (explicit override)
2. `<config_dir>/snowflake/sf.odbc.ini` (`~/.config/snowflake/…` on Linux,
   `~/Library/Application Support/snowflake/…` on macOS)
3. `~/.snowflake/sf.odbc.ini`
4. *(macOS only)* the `.pkg` installer default — `MACOS_INSTALLER_INI`
   (`odbc/src/api/ini_paths.rs`), placed under the installer's `INSTALL_DIR`
   (`odbc/installer/mac/package.sh`); currently
   `/opt/snowflake/snowflakeodbcud/sf.odbc.ini`

Should be `chmod 600` on Unix; the driver warns on looser permissions. Keys are
listed in [§3.2](#32-odbc).

### 4.3 Environment variables

| Variable | Wrapper | Purpose |
|----------|---------|---------|
| `SNOWFLAKE_TROUBLESHOOTING_ENABLED` | all | Enables troubleshooting mode ([§1](#1-troubleshooting-logs)); read once at process start. |
| `SNOWFLAKE_TROUBLESHOOTING_REPORT_PATH` | all | Output dir for `sf_driver_troubleshooting.log`; defaults to CWD ([§1](#1-troubleshooting-logs)). |
| `SNOWFLAKE_HOME` | all | Base dir for `connections.toml` / `config.toml` ([§4.1](#41-connectionstoml--configtoml)). |
| `SNOWFLAKE_DEFAULT_CONNECTION_NAME` | all | Default profile a bare connect picks ([§4.1 case 2](#41-connectionstoml--configtoml)). |
| `SF_ODBC_INI` | ODBC | Explicit path to `sf.odbc.ini` (highest discovery priority). |
| `HTTP_PROXY` / `HTTPS_PROXY` (and lowercase) | all | Proxy; the diagnostic reports system-vs-env proxy differences. |

**Environment variables cannot set arbitrary connection parameters.** There is
no `SNOWFLAKE_<PARAM>` mechanism — `ParamDef` aliases are matched against
connection parameters and TOML keys only, never env vars. **`RUST_LOG` is not
read either**; use troubleshooting mode ([§1](#1-troubleshooting-logs)) to capture logs regardless of level.

---

## Appendix A — Symptom → action

Looking up a **symptom, error string, or error code**? Use the
**[Keyword → page index](troubleshooting/keyword-index.md)** — it maps exact error
strings, status codes, and symptom keywords to the owning deep-dive page and the
heading to jump to. It replaces the short symptom table that used to live here (that
table had drifted into a partial duplicate of the index).

The capture-first workflow is unchanged: grab **logs**
([§1](#1-troubleshooting-logs)) and/or run the **connection diagnostic**
([§2](#2-connection-diagnosis)) before you route — nearly every symptom is diagnosed
from one of those two artifacts, whichever the index points you at.
