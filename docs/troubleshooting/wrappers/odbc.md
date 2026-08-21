# ODBC wrapper

ODBC-specific troubleshooting for the sf_core-based Snowflake ODBC driver: the
Driver Manager and installation layer (where most ODBC-only failures live), how
logging differs from the other wrappers, and pulling the real error out via
`SQLGetDiagRec`. For auth, TLS/CRL, and query issues that aren't ODBC-specific,
follow the links into the [core pages](../index.md).

Source tree: `odbc/` — a Rust `cdylib` (`libsfodbc.so` / `.dylib` /
`sfodbc.dll`) that links `sf_core` directly.

---

## Architecture

```
ODBC application
        │  ODBC C ABI (SQLDriverConnect, SQLExecDirect, …)
        ▼
Driver Manager   (unixODBC libodbc / iODBC / Windows DM)
        │  loads the driver .so/.dylib/.dll named in odbcinst.ini
        ▼
libsfodbc  (cdylib, ODBC C ABI in odbc/src/c_api.rs)
        ▼
sf_core  (protobuf transport, Rust-native HTTP + TLS)
```

Unlike Python (pyo3), Node.js (napi), and JDBC (JNI), the ODBC driver links
sf_core **in-process**: the connection/query path runs over the protobuf transport
(the same one the .NET driver uses), while config, logging, and helper calls reach
sf_core directly — `sf_core::config` (INI loading, param registry),
`LogManager::for_odbc`, and helpers like `query_types` / `SensitiveString`. There is
no separate runtime to load; the driver *is* the native library.

---

## Symptom: "Data source name not found" / driver library not loaded

This is a Driver-Manager/installation problem, before any Snowflake logic runs.

1. **Which DM are you on?** unixODBC (`isql`, `libodbc.so`) and iODBC
   (`iodbctest`, `libiodbc`) read the same INI files but ship separately. Mixing a
   driver registered for one DM with the other is a common miss.
2. **`odbcinst.ini`** must have a section for the driver whose `Driver=` points at
   the actual `libsfodbc` path. A DSN in **`odbc.ini`** then references that driver
   by section name. The DM resolves the driver path from `odbcinst.ini`
   (`odbc/src/api/odbc_installer.rs`).
3. **`Can't open lib … / file not found`** means the `Driver=` path is wrong or
   the library is for the wrong architecture — check it directly:
   ```sh
   ldd /path/to/libsfodbc.so     # Linux: unresolved deps show here
   otool -L /path/to/libsfodbc.dylib   # macOS
   ```
4. The driver-managed INI for driver settings is **`sf.odbc.ini`** — see
   [Runbook §4.2](../../troubleshooting-runbook.md).

---

## The driver's own settings file: `sf.odbc.ini`

`sf.odbc.ini` holds the **driver's own** settings and is separate from the Driver
Manager's `odbcinst.ini` (driver registration) and `odbc.ini` (DSN definitions).
On UNIX the driver searches for it in this order and uses the **first** file that
exists (`odbc/src/api/ini_paths.rs`):

1. `$SF_ODBC_INI` — explicit path override; set this to force a specific file.
2. `<config-dir>/snowflake/sf.odbc.ini` — `~/Library/Application Support/snowflake/sf.odbc.ini`
   on macOS, `~/.config/snowflake/sf.odbc.ini` on Linux.
3. `~/.snowflake/sf.odbc.ini`.
4. **macOS only:** the `.pkg` installer default — the driver looks here via the
   `MACOS_INSTALLER_INI` constant (`odbc/src/api/ini_paths.rs`), which resolves to
   `sf.odbc.ini` under the installer's `INSTALL_DIR` (`odbc/installer/mac/package.sh`).
   Both must agree; currently `/opt/snowflake/snowflakeodbcud/sf.odbc.ini` (verify
   against those symbols rather than trusting this literal — it can move between
   releases).

The shipped macOS default is a single line, and a minimal file rarely needs more:

```ini
DriverManagerEncoding=UTF-32
```

`UTF-32` matches iODBC, the DM macOS ships by default; the driver otherwise defaults
to `UTF-16` (what the other DMs use). A wrong value garbles wide-string calls — see
[wide-string calls return garbled or truncated text](#symptom-wide-string-calls-return-garbled-or-truncated-text)
below.

Driver logging keys (log path, level, rotation) also live here — see
[Runbook §1.2 ODBC](../../troubleshooting-runbook.md#12-odbc) for the exact keys.

For contrast, the **Driver-Manager** files that `sf.odbc.ini` sits alongside — a
minimal registration plus one DSN — look like:

```ini
# odbcinst.ini — registers the driver library (referenced by section name below)
[Snowflake]
Driver = /path/to/libsfodbc.so

# odbc.ini — a DSN that points at that driver and carries the connection keys
[MySnowflakeDSN]
Driver    = Snowflake
SERVER    = myaccount.snowflakecomputing.com
UID       = me
DATABASE  = db
SCHEMA    = sc
WAREHOUSE = wh
```

DSN keys are matched case-insensitively; use the exact spellings shown (see
[Auth & connection keys](#auth--connection-keys)).
The example above has no `PWD` line, but a `PWD` **is** honored if you add one: at
connect time the driver reads every key from the DSN section (`read_dsn_config` /
`merge_dsn_config` in `odbc/src/api/connection.rs`), stripping only `Driver`,
`Description`, and `DSN`. **Precedence:** a password supplied at connect time — in
the connection string, or as the `PWD` argument to `SQLConnect` — overrides one
stored in the DSN. Snowflake's Windows setup dialog never writes a password for you
(see [Configuring a DSN on Windows](#configuring-a-dsn-on-windows)), and keeping a
plaintext password in a DSN is discouraged.

---

## Symptom: wide-string calls return garbled or truncated text

On UNIX, the two Driver Managers disagree on the width of a wide character
(`SQLWCHAR`): **unixODBC uses 2-byte UTF-16**, **iODBC uses 4-byte UTF-32**. The
driver must decode wide (`…W`) calls with the same width the DM used to encode
them — DSN names, SQL text, and fetched wide strings all cross this boundary — so a
width mismatch turns wide text into mojibake, truncation, or strings with extra NUL
bytes.

The width is selected by the **`DriverManagerEncoding`** key in `sf.odbc.ini`
(`odbc/src/api/encoding.rs`):

| Driver Manager | `DriverManagerEncoding` |
|---|---|
| unixODBC | `UTF-16` (the driver's default) |
| iODBC — the DM macOS ships by default | `UTF-32` |

The driver negotiates the encoding **once** per process and does **not** transcode
on your behalf: on a suspected mismatch it logs a one-time warning naming this INI
key, then continues with the configured width. If wide text is garbled, set
`DriverManagerEncoding` to match your DM and restart the application.

> The macOS `.pkg` ships `DriverManagerEncoding=UTF-32` for iODBC. If you instead
> run **unixODBC on macOS**, override it to `UTF-16` (via `$SF_ODBC_INI` or a
> per-user `sf.odbc.ini`), or every wide-string call is mis-decoded. On Windows the
> default (`UTF-16`) already matches the Windows Driver Manager — leave this key
> unset there. The one-time mismatch warning above is **UNIX-only**, so a stray
> `UTF-32` on Windows silently mis-decodes every wide call with no diagnostic.

---

## Logging

ODBC is the wrapper that uses the **core file-logging layer** directly (Python and
JDBC bridge into their host frameworks instead). Two independent layers matter:

### Driver logging (the driver's own events)

Enabled through the ODBC INI keys — see
[Runbook §1.2 ODBC](../../troubleshooting-runbook.md#12-odbc) for the exact keys.

> **The log path gates everything.** If the log directory is unset, the core file
> layer discards every event **silently** — not "logs to a default location." A
> missing path is the first thing to check when "logging is on but there's no
> file." See
> [logging-diagnostics.md](../core/logging-diagnostics.md#log_path-gates-the-core-file-layer-toml--odbc).

### Driver-Manager tracing (the ODBC call sequence)

Separate from the driver's own logs: unixODBC can trace every ODBC call the
application makes, with return codes, at the DM layer. Enable it in the
`odbcinst.ini` `[ODBC]` section:

```ini
[ODBC]
Trace = Yes
TraceFile = /tmp/odbc_dm_trace.log
```

Use DM tracing when you suspect the **application ↔ DM** interaction (wrong call
order, unexpected return codes); use driver logging for what happens **inside** the
driver and on the wire to Snowflake.

---

## Symptom: a call fails but you only see the return code (`SQL_ERROR`)

`SQL_ERROR` / `SQL_SUCCESS_WITH_INFO` is just a status. The actual SQLSTATE,
Snowflake error code, and message are in the **diagnostic record** — always pull it
with `SQLGetDiagRec` (or `SQLGetDiagField`) on the same handle before doing
anything else. In `isql`, the error text is printed for you; in application code,
loop `SQLGetDiagRec` over record numbers until it returns `SQL_NO_DATA`.

---

## Symptom: TLS/certificate errors

Same rustls-based core stack as every other wrapper (not a system ODBC TLS layer).
Supply a trust anchor with the `custom_root_store_path` connection setting, which
**replaces** the system root store (not additive; there is no `use_system_roots`
toggle) — put both public and private CAs into one PEM bundle if you need both. See
[cert-chain.md §B2](../core/crl-tls/cert-chain.md#b2-custom-trust-store-empty-or-malformed).
To reproduce a TLS failure without the DM/driver at all, use
[tls-client-tool.md](../core/tls-client-tool.md).

---

## Auth & connection keys

ODBC connection-string / DSN keys are matched **case-insensitively** against the
core parameter registry and resolve to their canonical names
(`odbc/src/api/oauth.rs`, `sf_core/src/config/param_registry.rs`) — the parser
uppercases the key you supply and looks it up, rather than applying a mechanical
`SCREAMING_SNAKE` transform, so use the exact spellings shown here and in the
examples above. Standard ODBC keys `UID` / `PWD` / `SERVER` / `DSN` are recognized
directly. The login flows
(key-pair, PAT, OAuth, MFA, external-browser/Okta) are core behavior —
[authentication.md](../core/authentication.md). A persistent `390114` after a
working connect means the master token expired (reconnect); see
[authentication.md § Session expiry & renewal](../core/authentication.md#session-expiry--renewal).

---

## Quoting connection-string values (passwords with `;`)

In a DSN-less connection string, a value containing a semicolon — most often a
password — must be **brace-quoted**, or the parser splits it at the `;` and you get
a wrong/short password (auth fails) or an invalid-connection-string error
(`odbc/src/api/connection.rs`):

```
PWD={p@ss;word}
```

- Wrap the whole value in `{ }`; everything up to the closing brace is literal,
  including `;` and interior spaces.
- A literal `}` inside the value is **doubled**: `{ab}}cd}` yields `ab}cd`.
- Keys are case-insensitive. A **duplicate key** and an **unbalanced brace** are
  each rejected up front — the diagnostic names the cause (`duplicate key`,
  `unterminated brace`).

Braces are a *connection-string* convention only. Values in a DSN's `odbc.ini`
section are plain `KEY = value` lines read verbatim by the Driver Manager — do not
brace-quote there.

---

## Configuring a DSN on Windows

On Windows the driver is configured through the **ODBC Data Source Administrator**
(`odbcad32.exe`), which stores DSNs in the registry rather than in `odbc.ini`
files. Add a *User* or *System* DSN under the Snowflake driver; the setup dialog
persists the same connection keys used everywhere else — `SERVER`, `UID`,
`DATABASE`, `SCHEMA`, `WAREHOUSE`, `ROLE`, `AUTHENTICATOR`, `PROXY`, `NO_PROXY`,
`PRIV_KEY_FILE`, and the OAuth client fields (`odbc/src/setup_dialog.rs`). The
dialog also writes a `TRACING` field, but the current core parameter registry does
**not** consume it — it does not enable logging; use the ODBC INI logging keys
([Runbook §1.2](../../troubleshooting-runbook.md#12-odbc)) instead.

Two things are deliberately **not** written to the stored DSN
(`odbc/src/setup_common.rs`):

- **`PWD`** — the setup dialog never writes the password to the DSN; supply it at
  connect time (in the connection string, or when the DM prompts). A DSN written by
  the dialog that "forgets" the password is behaving as designed. (A `PWD` you add to
  a DSN by hand *is* still read at connect time — see the
  [`sf.odbc.ini` / DSN note above](#the-drivers-own-settings-file-sfodbcini).)
- **OAuth secrets** (`OAUTH_CLIENT_SECRET`, `TOKEN`) — read from the dialog only for
  the in-memory **Test** connection, never saved to the registry.

---

## Connection setup

```sh
# DSN-less connection string (isql -v with a full connection string):
isql -v -k "Driver=Snowflake;SERVER=myaccount.snowflakecomputing.com;UID=me;PWD=...;WAREHOUSE=wh;DATABASE=db;SCHEMA=sc"

# Or via a DSN defined in odbc.ini:
isql -v MySnowflakeDSN me '...'
```

---

Related: [authentication.md](../core/authentication.md) ·
[cert-chain.md](../core/crl-tls/cert-chain.md) ·
[query-execution.md](../core/query-execution.md) ·
[logging-diagnostics.md](../core/logging-diagnostics.md) ·
[tls-client-tool.md](../core/tls-client-tool.md) · up to the
[deep-dive index](../index.md) ·
[Runbook §1.2](../../troubleshooting-runbook.md#12-odbc) /
[§2.2](../../troubleshooting-runbook.md#22-odbc)
