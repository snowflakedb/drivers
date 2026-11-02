# Troubleshooting runbook for Snowflake drivers built on the Universal Core

How to diagnose a problem with the Snowflake Connector for Python, the Snowflake
ODBC Driver, and the Snowflake JDBC Driver that share the Universal Core (`sf_core`).

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
> diagnose a driver issue. Wrapper + version: **snowflake-connector-python
> 5.x**. Symptom: on connect I get `<paste the exact error string / code>`, from
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

Two environment variables. Python and JDBC read them **once** at driver init;
ODBC re-reads them (together with `sf.odbc.ini`) on every connect and applies a
change without a process restart (`odbc/src/api/log_reload.rs`):

| Variable | Default | Notes |
|----------|---------|-------|
| `SNOWFLAKE_TROUBLESHOOTING_ENABLED` | off | `true` / `1` / `yes` / `on` enables. |
| `SNOWFLAKE_TROUBLESHOOTING_REPORT_PATH` | current working directory | Created if missing. |

Output: `<path>/sf_driver_troubleshooting.log` — single file, no rotation, mode
`0600` on Unix. If the file already exists, subsequent runs **append** to it
(it is not truncated or rotated). There is no connection parameter; outside
ODBC, enabling it requires a process restart.

Troubleshooting mode also runs the [connection diagnostic](#2-connection-diagnosis)
on every connect and writes the report to the troubleshooting directory. To
choose the report directory or an allowlist file, also set
`enable_connection_diag=true`.

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

Secrets use a `SensitiveString` type (`sf_core/src/sensitive.rs`) that renders
as `****` and is zeroized on drop. Tokens, passwords, private keys, and passcodes
are never logged. The core does not log HTTP request or response headers, logs
URLs as host + path only with the query string stripped (`logging::url_for_log`),
and redacts Azure SAS signatures as `sig=REDACTED`. Account, host, role,
database, schema, warehouse, `queryId`, `requestId`, and `sessionId` are safe to
log. SQL text and bound parameters are off by default
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
| `connection_diag_allowlist_path` | *(none)* | Absolute path to the saved JSON output of `select system$allowlist();`. The only source of allowlist entries. |

`connection_diag_log_path` must be an **existing absolute** directory; otherwise
the core falls back to the system temp dir (`/tmp`, `%TEMP%`). The report
filename is always `SnowflakeConnectionTestReport.txt`. The same parameters can
be set in a `connections.toml` profile — see
[§4.1](#41-connectionstoml--configtoml).

**Allowlist.** The diagnostic does not run `system$allowlist()` itself. Without
`connection_diag_allowlist_path` it probes only the account host, and the report's
stage section reads `Snowflake Stage information - Unavailable`. To probe stage,
OCSP-cache and other allowlist hosts, run `select system$allowlist();` from any
working client, save the JSON, and point `connection_diag_allowlist_path` at it.
The file is used whether or not the login succeeds.

**Trust store.** The root store is loaded once per driver instance; OS trust
changes after the first connection apply on process restart. With CRL checking
disabled (the default) the trust set is the OS root store plus the bundled
Mozilla roots. With CRL checking `ENABLED` or `ADVISORY` only the OS store is
used, and a failure to load it fails the connection. On the default (non-custom)
path, `SSL_CERT_FILE` / `SSL_CERT_DIR` replace the OS store as the source of OS
roots; with CRL disabled the bundled Mozilla roots are still added. With CRL
enabled/advisory only the env-var roots are trusted. To override the trust set,
set `custom_root_store_path` to a PEM file of trusted roots — this **replaces**
all other roots for that connection. `extra_root_store_path` is not available in
Python, JDBC, or ODBC; to trust both public roots and a private CA, put both in
the one `custom_root_store_path` bundle. To verify what you trust: inspect/list
the PEM for a custom store, or use the OS trust-store tools for the system
store. The diagnosis report ([§2.5](#25-reading-the-report)) prints the
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

Pass the canonical names in the connection string (keys are case-insensitive):

```
Driver=<snowflake-ud-driver>;SERVER=…;UID=…;PWD=…;\
ENABLE_CONNECTION_DIAG=true;\
CONNECTION_DIAG_LOG_PATH=/tmp/sfdiag;\
CONNECTION_DIAG_ALLOWLIST_PATH=/tmp/allowlist.json
```

The ODBC driver resolves each key against the core param registry
(`normalize_connection_string_option` in `odbc/src/api/connection.rs`) and passes
it on under its canonical lowercase name. A key the registry does not recognize is
dropped, with SQLSTATE `01S00` (invalid connection string attribute).

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

For the account host and, when `connection_diag_allowlist_path` is supplied,
**every entry** in that allowlist file (STAGE, GATEWAYS, HIVE, PROXY, OCSP cache, …):

| Check | Detail |
|-------|--------|
| **DNS resolution** | Resolves the host; classifies each IP public or private. |
| **PrivateLink sanity** | Flags a `*.privatelink.*` host resolving to a **public** IP. |
| **Connected peer IP** | The IP actually connected to after TCP connect, not just the DNS answer. Through a proxy this is the proxy's IP, marked `(via proxy)`. |
| **TLS certificate chain** | Negotiated TLS version and cipher suite; per cert: serial (hex), subject, issuer, validity window, `crt.sh` link. |
| **CRL distribution points** | Downloads (plain HTTP, direct — not through the proxy) and parses each cert's CRL. A successfully fetched CRL URL is not fetched again in the same run; a failed fetch is retried for the next cert. `https://` distribution points are skipped. |
| **Allowlist reachability** | Dispatches on port: **443** → TLS handshake, **80** → HTTP GET, other → raw TCP. |
| **Proxy honoring** | An explicit `proxy_host` is used on all ports (CONNECT tunnel for 443/other, absolute-form GET for 80); the hop to the proxy is plain TCP even with `proxy_scheme=https`. Env-var proxies (`HTTP_PROXY` / `HTTPS_PROXY`) are reported but not used by the probes. |
| **HTTP status** | Port-80 check only: success = `{200, 301, 302, 307, 308, 400, 403}`; a response with no parseable status line also counts as success. Port 443 succeeds once the TLS handshake completes. |

> **Not checked: OCSP.** The core does revocation via **CRL**, not OCSP — an
> intentional divergence from the old drivers.

The diagnostic runs in two phases: **pre-connect** (environment and proxy
detection) and **post-connect** (account-host and allowlist probing). The report
is written whether or not the login succeeds.

### 2.5 Reading the report

```
=========Connectivity diagnostic report================================
INITIAL           → account, resolved host, sf_core + driver version, OS /
                    architecture, cert revocation check mode, detected
                    platforms, allowlist-load errors
=========Proxy information - These are best guesses, not guarantees====
PROXY             → system proxies (env removed) vs env proxies
=========Snowflake URL information=====================================
SNOWFLAKE_URL     → nslookup (public/private), connected peer IP,
                    negotiated TLS protocol + cipher suite,
                    certificate chain, CRL DP + fetch results
=========Snowflake Stage information===================================
STAGE / GATEWAYS / … → per allowlist entry: "URL Check: Connected
                    Successfully" or "Failed: <reason>", plus TLS + CRL
```

Without an allowlist file the last section is
`=========Snowflake Stage information - Unavailable=====================`
instead.

| Finding | Means |
|---------|-------|
| `Failed:` on a `SNOWFLAKE_URL` entry | Account host unreachable — DNS, firewall, or TLS interception. Compare the connected peer IP to what you expect. |
| `.privatelink.` host → public IP | PrivateLink DNS not in effect; the client is bypassing the private endpoint. |
| CRL fetch failures | CRL distribution point blocked by firewall/proxy; TLS validation fails under strict CRL checking. |
| `STAGE`/`GATEWAYS` fails, `SNOWFLAKE_URL` succeeds | Login works but cloud-storage traffic is blocked — partial allowlist. |

> **Read the report file, not the logs.** The diagnostic runs on a
> `spawn_blocking` thread that does not inherit the thread-local tracing
> dispatcher every wrapper installs, so its `WARN` (path fallback) and `DEBUG`
> (full report) events reach no log — not the wrapper logger, the ODBC log file,
> or the troubleshooting log ([§1](#1-troubleshooting-logs)). The report file is
> the only artifact.

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
| `LogFile` | filename | Log file name; default `sf_driver.log`. |
| `LogRotation` | `NEVER` (or `NONE`),`DAILY`,`HOURLY`,`MINUTELY` | Default `NEVER`. |
| `LogMaxCount` | integer | Rotated files to keep (forces `DAILY` if rotation is `NEVER`). |
| `LogMaxSize` | bytes | **Not yet enforced** — warns and is ignored. |
| `LogQueryText` | bool | Logs SQL text — see [§3.5](#35-query-text--parameters-sensitive--off-by-default). |
| `LogQueryParameters` | bool | Logs bound params — see [§3.5](#35-query-text--parameters-sensitive--off-by-default). |
| `ErrorTraceEnabled` | bool | Include error traces; default on. |

Keys outside this set are silently ignored by the logging loader. A recognised
key with an invalid value does not fail the driver: at startup it prints a stderr
line and logging uses the defaults; on a per-connect reload it logs a warning and
keeps the previous configuration. The file is re-read on every connect, so a
change applies without a restart. DSN-level `LogLevel` / `LogPath` / `TRACING`
are not consulted.

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
- `DEBUG` — core API entry/exit, verbose diagnostics. The connection
  diagnostic's own events reach no log at any level; read its report file
  ([§2.5](#25-reading-the-report)).
- `TRACE` — finest-grained internal tracing; core-file-layer vocabulary. Over
  the Python and JDBC bridges it is delivered as `DEBUG` (finest level the
  bridges encode).

### 3.5 Query text & parameters (sensitive — off by default)

`log_query_text` / `log_query_parameters` (connection parameters; `LogQueryText` /
`LogQueryParameters` as ODBC INI keys) log SQL and bound values on the `INFO`
query lines. **Both default to `false`** and emit a `WARN` when enabled.
`log_query_parameters` has no effect unless `log_query_text` is also on. The SQL
text and the bindings are each truncated to `log_max_query_length` characters
(default `80`). Turn them on only in a controlled repro and scrub before
attaching logs to a ticket.

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
# ~/.snowflake/connections.toml   (see the directory lookup below)
[production]
account = "myaccount"
user    = "myuser"
enable_connection_diag         = true
connection_diag_log_path       = "/var/log/sfdiag"
connection_diag_allowlist_path = "/var/snowflake/allowlist.json"   # optional
```

Directory lookup (`sf_core/src/config/path_resolver.rs`): `$SNOWFLAKE_HOME`
(default `~/.snowflake`) if that directory exists; otherwise the platform config
directory + `snowflake` — `$XDG_CONFIG_HOME/snowflake` or `~/.config/snowflake`
on Linux, `~/Library/Application Support/snowflake` on macOS,
`%LOCALAPPDATA%\snowflake` on Windows. Python can point at a specific
connections file with the `connections_file_path` argument.

The profile is merged only when (`sf_core/src/config/resolver.rs`):

1. the app connects with `connection_name = "production"`, **or**
2. the app makes a **bare** connect (no connection parameters at all) — then the
   *default* profile is loaded, its name resolved from
   `SNOWFLAKE_DEFAULT_CONNECTION_NAME` → `config.toml`'s
   `default_connection_name` → literal `"default"`. In JDBC the bare connect is
   the URL `jdbc:snowflake:auto`.

If the app passes explicit `account` / `user` / `password` **without**
`connection_name`, no profile is loaded and this route cannot inject a setting —
a small code change (add `connection_name`, or the parameter itself) is required.

On Unix the TOML files should be `chmod 600`. Group/other **writable** (`0o022`)
is a hard error; group/other **readable** (`0o044`) only warns — suppressible
via `SF_SKIP_WARNING_FOR_READ_PERMISSIONS_ON_CONFIG_FILE` (any value), or bypass
the whole check with the connection parameter
`unsafe_skip_config_file_permissions_check`. In Python,
`unsafe_skip_file_permissions_check` sets both this and
`crl_unsafe_skip_file_permissions_check`.

### 4.2 `sf.odbc.ini` (ODBC only)

Discovery order — first existing file wins (`odbc/src/api/ini_paths.rs`):

1. `$SF_ODBC_INI` (explicit override)
2. `<config_dir>/snowflake/sf.odbc.ini` (`~/.config/snowflake/…` on Linux,
   `~/Library/Application Support/snowflake/…` on macOS)
3. `~/.snowflake/sf.odbc.ini`
4. *(macOS only)* the `.pkg` installer default — `MACOS_INSTALLER_INI`
   (`odbc/src/api/ini_paths.rs`), placed under the installer's `INSTALL_DIR`
   (`odbc/installer/mac/package.sh`):
   `/opt/snowflake/snowflakeodbc/sf.odbc.ini`

Should be `chmod 600` on Unix. Group/other-readable only warns; a
group/other-**writable** file is rejected and handled like an invalid value
([§3.2](#32-odbc)). Keys are
listed in [§3.2](#32-odbc).

### 4.3 Environment variables

| Variable | Wrapper | Purpose |
|----------|---------|---------|
| `SNOWFLAKE_TROUBLESHOOTING_ENABLED` | all | Enables troubleshooting mode ([§1](#1-troubleshooting-logs)); read once at process start (ODBC: on every connect). |
| `SNOWFLAKE_TROUBLESHOOTING_REPORT_PATH` | all | Output dir for `sf_driver_troubleshooting.log`; defaults to CWD ([§1](#1-troubleshooting-logs)). |
| `SNOWFLAKE_HOME` | all | Base dir for `connections.toml` / `config.toml`, used only if it exists ([§4.1](#41-connectionstoml--configtoml)). |
| `SNOWFLAKE_DEFAULT_CONNECTION_NAME` | all | Default profile a bare connect picks ([§4.1 case 2](#41-connectionstoml--configtoml)). |
| `SF_SKIP_WARNING_FOR_READ_PERMISSIONS_ON_CONFIG_FILE` | all | Suppresses the group/other-readable warning for TOML files ([§4.1](#41-connectionstoml--configtoml)). |
| `SF_ODBC_INI` | ODBC | Explicit path to `sf.odbc.ini` (highest discovery priority). |
| `SF_GLOBAL_SSL_VERSION` | ODBC | Pins the TLS version (`TLSv1_2` / `TLSv1_3`; `DEFAULT` = no pin). A value below TLS 1.2 fails the connection. |
| `CORE_PATH` | JDBC | Explicit path to the native bridge library; checked before the library bundled in the JAR. |
| `HTTP_PROXY` / `HTTPS_PROXY` / `NO_PROXY` (and lowercase) | all | Used only when `use_proxy_env=true` (Python default `true`; elsewhere `false`, ODBC key `PROXYWITHENV`). The diagnostic reports system-vs-env proxy differences. |
| `SSL_CERT_FILE` / `SSL_CERT_DIR` | all | Replace the OS store as the source of OS roots on the default trust path ([§2](#2-connection-diagnosis) **Trust store**). |
| `SF_TEMPORARY_CREDENTIAL_CACHE_DIR` / `XDG_CACHE_HOME` | all | Location of the file-based token cache, used when no OS keyring is available. |
| `SNOWFLAKE_DISABLE_PLATFORM_DETECTION` | all | Turns off the cloud-platform detection probes run at connect. |
| `SNOWFLAKE_WIF_ALLOWED_HOST_SUFFIXES` | all | Extra hostname suffixes accepted for `WORKLOAD_IDENTITY` authentication, added to the built-in Snowflake suffixes. |
| `SNOWFLAKE_RUNNING_INSIDE_SPCS` | all | Set by Snowpark Container Services; enables reading `SNOWFLAKE_ACCOUNT` / `SNOWFLAKE_HOST` / `SNOWFLAKE_DATABASE` / `SNOWFLAKE_SCHEMA` as connection defaults. |

**Environment variables cannot set arbitrary connection parameters.** There is
no `SNOWFLAKE_<PARAM>` mechanism — `ParamDef` aliases are matched against
connection parameters and TOML keys only, never env vars. The only env vars that
feed connection settings are the specific ones in the table above. **`RUST_LOG`
is not read either**; use troubleshooting mode ([§1](#1-troubleshooting-logs)) to
capture logs regardless of level.

---

## Appendix A — Symptom → action

Looking up a **symptom, error string, or error code**? Use the
**[Keyword → page index](troubleshooting/keyword-index.md)** — it maps exact error
strings, status codes, and symptom keywords to the owning deep-dive page and the
heading to jump to.

Capture first: grab **logs**
([§1](#1-troubleshooting-logs)) and/or run the **connection diagnostic**
([§2](#2-connection-diagnosis)) before you route — nearly every symptom is diagnosed
from one of those two artifacts, whichever the index points you at.
