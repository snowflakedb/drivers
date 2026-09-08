# Core API concurrency guarantees

This document defines the **core contract** for all database operations - *wrappers may be stricter, never weaker.*

> Direct core operations stay safe even under concurrent misuse of a handle: no panic, no deadlock, cancel is not
> blocked.

---

## Core contract

Handles are the isolation unit:

| handle                               | thread-safe       | notes on concurrent use of the same handle                                                                                                                                                                                  |
|--------------------------------------|-------------------|-----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------|
| Database (no session)                | yes               | -                                                                                                                                                                                                                           |
| Connection                           | yes*              | Concurrent statements on the same connection are allowed. Whether they run overlapped or queued is the `serialize_session_operations` connection parameter, which defaults to the wrapper preset when unset.                |
| Statement                            | yes* (serialized) | Core serializes statement operations.                                                                                                                                                                                       |
| ResultSet                            | partially         | Concurrent `get_stream` / `get_chunks` are safe (independent reader per call). <br> Each reader is single-consumer - one thread owns iteration. It's exposed via FFI *Arrow C Stream Interface* that follows the same rule. |
| UploadStreamSession / DownloadStream | no                | Single-threaded sequential use only. Concurrent use is not supported.                                                                                                                                                       |

> \* *session mutations are not thread safe!* See details below.

### `serialize_session_operations`

Core is permissive by default at the handle-safety level (no panic, no deadlock). Each wrapper chooses whether core
serializes session operations on one connection by default.

When the preset is true, core holds a per-connection gate across every backend session operation: backend request
submit, wait for the response, and session-cache merge. This setting can be also configured by the user.

The gate is **not** held for:

- cancel
- result-chunk download / `get_stream` / `get_chunks`
- PUT/GET body transfer after backend's response
- heartbeat, login, logout, and local-only option/parameter writes

> Note: when the wrapper is stricter (and e.g. serializes connection operations in wrapper) then this setting is not
> applicable.

---

## Cancel

Cancel applies to non-blocking API operations (those marked `async_first` in the proto).

- Cancel is non-blocking: the caller returns immediately after signaling.
- Cancel is safe to call from any thread (e.g. cross-thread statement execution cancel).
- Cancel is idempotent: issuing it again is harmless.
- The canceled operation sees a `Cancelled` result.
- Cancel does not guarantee the backend job is aborted.

> **WARN**: Session state and cancel - aborting an operation can leave driver state and backend session state out of
> sync (e.g. cancel `COMMIT`/`ROLLBACK`, `ALTER SESSION`, or a statement involving hybrid tables).

---

## Session mutations

Session-mutating operations on the same connection are not thread-safe (unless `serialize_session_operations` applies).

- Some operations change session state in the backend **and** in the client cache. It includes operations like
  `ALTER SESSION`, `USE`, `connection_set_autocommit`, etc. It also applies to the corresponding queries sent by a
  statement execute.

- Only one session mutation in flight per connection. Overlapping mutations can finish in either order. The cache
  reflects whichever response is merged last and may disagree with the backend.

- Read-only queries (`SELECT`, etc.) are **not** session mutations. When `serialize_session_operations` is false they
  may overlap other queries on the same connection. When it is true, statement executes are queued even when the SQL is
  read-only.

- Hybrid-table queries return a per-connection context that core caches and sends with the next statement. Overlapping
  statements (only when serialization is off) can send an older snapshot, then the cache reflects whichever response is
  merged last.

---

## Backend behavior

Snowflake does not serialize statements submitted concurrently on one session. If two statements are in flight, they may
execute concurrently server-side (capped by warehouse concurrency).

- Concurrent statements share session state (role, database, schema, warehouse, variables, parameters, transaction
  state). Concurrent `USE`, `ALTER SESSION`, `SET`, `COMMIT`, etc. are unsafe unless externally synchronized.

- Transactions are session-scoped. Concurrent statements on one session do not form independent transactions.

- Ordering is not guaranteed. If statement B depends on statement A, their order should be enforced externally, or they
  should be submitted as a multi-statement request (which executes in order).
