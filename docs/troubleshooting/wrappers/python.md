# Python wrapper

Python-specific troubleshooting for the sf_core-based `snowflake-connector-python`
(4.x): native-module loading, the Rust HTTP stack (not `requests`), the timeout
fan-out, and Arrow/pandas delivery. Part of the
[troubleshooting deep-dive](../index.md). For auth, TLS/CRL, and query issues that
aren't Python-specific, follow the links into the [core pages](../index.md).

Source tree: `python/` (build: `python/pyproject.toml`, `python/hatch_build.py`).

---

## Architecture

```
import snowflake.connector
        │  PEP 249 API (Connection / Cursor)
        ▼
snowflake.connector._core.sf_core_python   ← compiled pyo3 extension
        │  (python/src/snowflake/connector/_core/)
        ▼
sf_core  (protobuf dispatch, Rust-native HTTP + TLS)
```

The `_core` extension is built by Hatch at wheel-build time and loaded once at
import. The core HTTP transport is **Rust-native (`reqwest` + rustls)** — the
connector does **not** use the Python `requests` library. That single fact
explains several of the symptoms below.

---

## Symptom: `OperationalError: Couldn't load core driver dependency (sf_core_python)`

The compiled core extension is missing or was built for a different Python ABI.
The extension is the pyo3 module **`sf_core_python`** *inside* the
`snowflake.connector._core` package — not `_core` itself, which is an ordinary
(empty) package that always imports. The same root cause surfaces as **two**
different messages, depending on which import path trips first:

- **`ImportError: cannot import name 'sf_core_python' from
  'snowflake.connector._core'`** — the raw failure. A plain `import
  snowflake.connector` reaches an internal module that imports the binding at module
  top level, so on a broken install this is usually what you hit first.
- **`OperationalError: Couldn't load core driver dependency (sf_core_python).`** —
  the guarded, friendlier re-raise the connector emits when the binding is reached
  through one of its guarded call paths.

Either way the fix is the same.

1. Reproduce the import that actually fails — target the **`sf_core_python`**
   submodule, not `_core`:
   ```sh
   python -c "from snowflake.connector._core import sf_core_python"
   ```
   A broken install prints `ImportError: cannot import name 'sf_core_python' from
   'snowflake.connector._core'` (or a missing-shared-object error beneath it).
   `python -c "import snowflake.connector._core"` on its own **succeeds even when the
   driver is broken** — the empty package imports fine — so it is not a useful test.
2. **From wheel:** confirm the wheel's ABI tag matches your interpreter — e.g.
   `cp312-cp312-manylinux_2_28_x86_64` must match your Python minor version, libc,
   and architecture. A `cp311` wheel will not import under Python 3.12.
3. **From an editable / source build only (dev):** `SKIP_CORE_BUILD` is a Hatch
   dev-environment knob (it defaults to `1` there, to skip the Rust/cmake build) —
   it is *not* set by a normal `pip install`, so a user on a released wheel never
   hits this. If you built the package yourself in editable mode with it left on,
   the extension was never compiled; rebuild with it unset:
   ```sh
   SKIP_CORE_BUILD=0 pip install -e .
   ```

---

## Symptom: `OSError: libssl.so.X: cannot open shared object file`

Only occurs with **custom source builds**; manylinux wheels statically link their
crypto provider. Point the loader at your OpenSSL:

```sh
export LD_LIBRARY_PATH=/opt/custom/openssl/lib:$LD_LIBRARY_PATH   # macOS: DYLD_LIBRARY_PATH
```

---

## Symptom: TLS/certificate errors despite `REQUESTS_CA_BUNDLE` (or `SSL_CERT_FILE`) being set

`REQUESTS_CA_BUNDLE` is a `requests`-library variable. Because the core transport
is Rust-native and **does not use `requests`**, this variable has **no effect** on
connection or TLS behavior.

**Resolution:** point sf_core at your CA bundle with `custom_root_store_path`:

```python
con = snowflake.connector.connect(
    account="myaccount", user="myuser", password="...",
    custom_root_store_path="/etc/ssl/certs/corporate-ca.pem",
)
```

> `custom_root_store_path` **replaces** the system root store for that connection —
> it is not additive (there is no `use_system_roots` toggle). If you need to trust
> both the public web PKI and a private CA, put **both** into the one bundle. See
> [cert-chain.md §B2](../core/crl-tls/cert-chain.md#b2-custom-trust-store-empty-or-malformed).

---

## Symptom: a call hangs, or times out sooner/later than expected

Python exposes several **distinct** timeouts that fan out onto different phases —
there is no single master timeout. All the "wall-clock" ones include retry time
(`0` = no timeout):

| `connect()` kwarg | Default | Bounds |
|---|---|---|
| `login_timeout` | 120 s | The entire login operation, including retries |
| `query_timeout` | 0 (none) | Query execution, including retries |
| `request_timeout` | 120 s | All other operations (session close, heartbeat, …), including retries |
| `authentication_timeout` | 120 s | The interactive auth step (e.g. external-browser wait) |
| `connect_timeout` | system default | TCP connect only (per HTTP connection) |
| `retry_timeout` | none | Overall retry budget (see [query-execution.md](../core/query-execution.md#retries--503s)) |

So a query that "ignores" a short `login_timeout` is working as designed — set
`query_timeout` for query duration. A hang during external-browser login is bounded
by `authentication_timeout`, not `login_timeout`.

---

## Symptom: Arrow→pandas conversion raises `ArrowInvalid` / a type error

1. `TIMESTAMP_TZ` → needs recent `pyarrow`/`pandas` for tz-aware dtypes.
2. High-precision `NUMBER(38, n)` exceeds int64 — read as `Decimal` (or via
   `DictCursor`).
3. `pyarrow` version skew between build and runtime — check
   `python -c "import pyarrow; print(pyarrow.__version__)"`.

Arrow is the **wire transport for every fetch** — ordinary `fetchall()` /
`fetchone()` / `fetchmany()` decode it natively (through the compiled
`ArrowStreamIterator`) and need **no optional dependency**. The `[pyarrow]` /
`[pandas]` extras are required **only** to call the methods that hand you Arrow or
pandas objects:

- `[pyarrow]` → `cur.fetch_arrow_all()` / `cur.fetch_arrow_batches()`
- `[pandas]` → `cur.fetch_pandas_all()` / `cur.fetch_pandas_batches()` (also pulls in `pyarrow`)

```sh
pip install "snowflake-connector-python[pyarrow]"   # or [pandas]
```

---

## Symptom: `fetchall()` returns `[]`

**Almost always the query simply matched no rows** — `fetchall()` faithfully
returns an empty list. Confirm the result server-side (check `cursor.rowcount`, or
re-run the query in a worksheet) before suspecting the driver.

Rare edge cases, only once the above is ruled out:

- The cursor is already **exhausted** — a second `fetchall()` after the first
  drains the row iterator and returns `[]`. Fetch once per `execute()`.
- Mixing fetch APIs on one result — the row path (`fetchall`) and the Arrow path
  (`fetch_arrow_all`) consume the same underlying stream, so draining one leaves
  nothing for the other.

---

## Reading result metadata and the query ID

An executed cursor already carries the query's server-side identity and column
shape — you don't need to re-query for them:

- **`cursor.sfqid`** — the Snowflake-assigned query ID. This is the ID to quote
  when correlating with server-side history or with the driver logs (see
  [query-execution.md → Request / query correlation](../core/query-execution.md#request--query-correlation)).
- **`cursor.description`** — PEP 249 column metadata, populated after `execute()`.
  Each entry is a `ResultMetadata` named tuple
  (`name, type_code, display_size, internal_size, precision, scale, is_nullable`):
  tuple-compatible as the DB-API spec requires, but also attribute-addressable
  (`cur.description[0].name`). Import the type from `snowflake.connector.cursor`
  if you want to annotate it.
- **`cursor.rowcount`** — rows affected or returned, or `None` when the server
  reported no count.

To get the column shape **without executing** the statement, call
`cursor.describe(sql, params)`: it prepares the query server-side (`describeOnly`),
returns the `ResultMetadata` list — or `None` for DML/DDL that yields no result set
— and populates `cursor.description`. No rows are fetched.

### Attaching to a query by its ID (async / out-of-band)

`cursor.execute_async(sql)` returns `{"queryId": "..."}` immediately, without
waiting for the result. Fetch that result later — from the same cursor, or a
different cursor or process — by its ID:

```python
qid = cur.execute_async("call long_running_proc()")["queryId"]
# ... later, possibly on a fresh connection ...
cur.get_results_from_sfqid(qid)   # blocks until the query finishes, then wires up
rows = cur.fetchall()             # description / rowcount / sfqid now populated
```

`get_results_from_sfqid` waits for completion, then attaches the cursor to the
result set — the submit-and-poll shape from
[query-execution.md → Execution paths](../core/query-execution.md#execution-paths),
exposed as an explicit API. For a **non-blocking** status check, call
`connection.get_query_status(qid)` (returns a `QueryStatus`, importable from
`snowflake.connector`); `connection.get_query_status_throw_if_error(qid)` raises if
the query failed instead of returning a status.

---

## Symptom: `DatabaseError: Connection is closed`

Using the **connection** after it was closed — commonly by leaving a
`with snowflake.connector.connect(...)` block, calling `close()`, or letting it be
garbage-collected. The exact error is `Connection is closed.` with `errno=250002`
(`ER_CONNECTION_IS_CLOSED`), and it is a **`DatabaseError`, not an `OperationalError`** —
because `OperationalError` is a *subclass* of `DatabaseError`, an `except OperationalError`
will **not** catch it (catch `DatabaseError`). It is raised by connection methods such as
`cursor()`, `commit()`, `rollback()`, and `set_autocommit()`.

Already-buffered `fetchone()` / `fetchmany()` / `fetchall()` on a cursor whose
connection has since closed **still work**, as long as the cursor itself is still open —
the driver deliberately preserves this. But cursor operations that need a live
connection — `execute()`, `executemany()`, `nextset()`, and the `fetch_arrow_*` /
`fetch_pandas_*` methods — raise a *different* error, `InterfaceError: Cursor is closed.`
(`errno=252006`, `ER_CURSOR_IS_CLOSED`), because a cursor reports closed once its
connection is. For any of these, open a new connection.

---

## Auth, JWT, and session expiry

These are **not** Python-specific — the behavior lives in the core:

- Key-pair (JWT), PAT, OAuth, MFA, external-browser/Okta → [authentication.md](../core/authentication.md).
  Note the Python kwargs are `private_key_file` / `private_key` and
  `private_key_password` (the passphrase for an encrypted PEM — **not**
  `private_key_passphrase`).
- `390114` "session token has expired" is auto-recovered by the core; a persistent
  390114 means the **master** token expired — reconnect. See
  [authentication.md § Session expiry & renewal](../core/authentication.md#session-expiry--renewal).

---

## Connection setup

```python
import snowflake.connector
from snowflake.connector import ConnectionConfig

# Preferred: a typed ConnectionConfig. Because the fields are declared, your IDE
# autocompletes them and a type checker catches a misspelled parameter — the
# kwargs form below would silently ignore it. (connection_config.py is
# auto-generated from the core parameter registry.)
con = snowflake.connector.connect(config=ConnectionConfig(
    account="myaccount", user="me", password="...",
    warehouse="wh", database="db", schema="sc",
))

# Alternative: explicit kwargs (any sf_core setting is accepted):
con = snowflake.connector.connect(account="myaccount", user="me", password="...",
                                  warehouse="wh", database="db", schema="sc")

# Or a named connection from ~/.snowflake/connections.toml:
con = snowflake.connector.connect(connection_name="default")
```

> Pass **either** `config=` **or** keyword arguments, not both — combining them
> raises `ProgrammingError`.

---

Related: [authentication.md](../core/authentication.md) ·
[crl-tls-settings.md](../core/crl-tls/crl-tls-settings.md) ·
[query-execution.md](../core/query-execution.md) ·
[logging-diagnostics.md](../core/logging-diagnostics.md) · up to the
[deep-dive index](../index.md) · [Runbook](../../troubleshooting-runbook.md)
