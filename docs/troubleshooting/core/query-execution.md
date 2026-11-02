# Query execution

Sync and async query paths, result fetch, retries and 503s, and request/query
correlation. Part of the [troubleshooting deep-dive](../index.md); the
[Troubleshooting Runbook](../../troubleshooting-runbook.md) **Query fails / wrong
result** and **Slow / intermittent** rows route here.

---

## Execution paths

```
statement_execute_query()                 sf_core/src/apis/database_driver_v1/statement.rs
  ├─ sync  ── execute_sync_query()         sf_core/src/rest/snowflake/mod.rs
  │            POST /queries/v1/query-request (asyncExec=false), blocks for the body
  └─ async ── submit_statement_async()     sf_core/src/rest/snowflake/async_exec.rs
               POST /queries/v1/query-request (asyncExec=true)
               inline short-poll: 5ms, 10ms, 20ms, 40ms
               then exponential backoff on GET /queries/{query_id}/result
```

### Request / query correlation

Every submission emits an INFO log line tagged with a **`requestId`** (a UUID the
driver generates) and, once the server assigns it, the **`queryId`**. Both are
safe to log (router
[§1.4](../../troubleshooting-runbook.md#14-secret-redaction)) and are the primary
keys for correlating a driver-side failure with a server-side query. Capture the
`queryId` first for any "query failed / wrong result" investigation.

> `sequenceId` in the request body is a fixed constant, **not** a per-query
> counter or an idempotency key you can influence. Replay/dedup safety comes from
> the stable `requestId` (see [Retries](#retries--503s)), not from `sequenceId`.

---

## Result fetch

The result format is chosen by the **server response**, not a request parameter
(`sf_core/src/chunks/`):

| Response field | Format | Parser |
|---|---|---|
| `rowsetBase64` | Arrow IPC | `sf_core/src/chunks/arrow_parser.rs` |
| `rowset` | JSON (2-D string array) | `sf_core/src/chunks/json_parser.rs` |

Both produce an Arrow `RecordBatch` stream for the wrapper. The first chunk is
inline in the query response; additional chunks are presigned URLs downloaded in
parallel (up to `CLIENT_PREFETCH_THREADS`). Chunks are gzip-compressed
(`Content-Encoding`, else magic-byte `\x1f\x8b` detection).

| Setting | Default | Effect |
|---|---|---|
| `CLIENT_PREFETCH_THREADS` | 4 | Concurrent chunk downloads |
| `CLIENT_MEMORY_LIMIT` | 1536 (MB) | In-flight chunk memory budget; back-pressures prefetch when exceeded |
| `MULTI_STATEMENT_COUNT` | *unset* (statement parameter) | Expected statement count for multi-statement |

---

## Retries & 503s

**Entry: logs contain `HTTP status error: 503`, `max attempts (N) reached`,
`deadline exceeded`, or `retry-after … exceeds remaining budget`.**

The retried statuses are 408, 429, 307, 308, any 5xx, and any code listed in
`retry_extra_status_codes` (`sf_core/src/http/retry.rs`). The query
`POST /queries/v1/query-request` opts into POST retries, so 503s on submission are
retried.

**Dedup safety.** A query POST reuses one stable `requestId` UUID across all retry
attempts; attempts ≥ 2 carry `retryCount` and, by default, `retryReason` (the HTTP
status code that triggered the retry, e.g. `retryReason=503`). The server uses
`requestId` to detect replays — if the first attempt reached Snowflake before the
503, the retry joins the already-running query instead of launching a duplicate.
Set `include_retry_reason` to `false` to suppress the `retryReason` parameter;
`retryCount` is always sent regardless of this setting. Both parameters apply only
to query-request retries — login and other non-query endpoints carry neither.

**Default retry policy** (`sf_core/src/config/retry.rs`), configurable per
connection:

| Behavior | Default | Setting |
|---|---|---|
| Max attempts (incl. first) | 6 | `retry_max_attempts` |
| Backoff base | 250 ms | `retry_backoff_base_ms` |
| Backoff factor | 2× | `retry_backoff_factor` |
| Backoff cap | 16 000 ms | `retry_backoff_cap_ms` |
| Jitter | decorrelated | `retry_backoff_jitter` |
| Extra retried status codes | *none* | `retry_extra_status_codes` |
| Per-attempt HTTP timeout | 300 s (`0` = no timeout) | `retry_timeout` |
| Include retry reason | true | `include_retry_reason` |

`retry_timeout` bounds a **single HTTP attempt**, not the whole retry loop. The
overall wall clock comes from the operation's own timeout:

| Operation | Overall budget |
|---|---|
| Login | `login_timeout` (120 s) |
| Query | `query_timeout` (0 = unbounded) |
| Other HTTP calls (heartbeat, session close, …) | `request_timeout` (120 s) |
| PUT/GET file transfer | 600 s, and at most `put_get_max_attempts` (6) attempts |

Without a budget the loop stops after `retry_max_attempts` attempts with
decorrelated-jitter backoff.

**Exhaustion variants** (wrapped in `RestError::HttpRetry`): `max attempts (N)
reached; last status …` (all attempts used); `deadline exceeded after …` (the
operation budget ran out); `retry-after … exceeds remaining budget`.

**`Retry-After` back-pressure.** A 503 carrying `Retry-After: <seconds>` makes the
driver wait that long instead of the computed backoff. If the wait exceeds the
remaining budget it fails with `retry-after … exceeds remaining budget` — correct
behavior; the root cause is server-side throttling. Reduce submission rate at the
application layer.

**Retry-storm amplification.** The driver has no built-in concurrency limiter. N
concurrent queries each retrying up to `retry_max_attempts` can hit the service
with up to `N × retry_max_attempts` requests. `requestId` + `retryCount` prevents
double-execution but does not reduce request *volume*. Rate-limit or pool
connections at the application layer.

### RestError taxonomy (`sf_core/src/rest/snowflake/mod.rs`)

Query and session HTTP failures share one error type. Variants that matter
when diagnosing a failed statement:

| Variant | Meaning | Retryable? |
|---|---|---|
| `HttpRetry` / `Communication` | Retryable HTTP exhausted, or a single-attempt transport failure | Already retried — no |
| `QueryFailed` | Server error in the query-response body (GS code + SQLSTATE) | Depends on the Snowflake error code |
| `SessionExpired` | 390xxx session-token expiry | Yes — refresh then one retry ([authentication.md](authentication.md#session-expiry--renewal)) |
| `AsyncPollResultNotFound` | GS `612` on async poll (PUT/GET has no async result) | First poll: driver retries as sync. Later: no |
| `OperationTimeout` | Login or statement-poll deadline | No |
| `MasterTokenTerminal` | GS `390113` / `390114` / `390115`: the master token can never be renewed | No — reconnect ([authentication.md](authentication.md#session-expiry--renewal)) |

`QueryFailed` carries a Snowflake error code: `390xxx` auth/session; `612`
async-poll-not-found (handled automatically, below); `002xxx` SQL compilation
(**not** retryable — more attempts won't help).

**Idempotency:** poll `GET`, file `GET`/`PUT`, heartbeat, and token-refresh are
all safe to retry. `POST /queries/v1/query-request` is retry-safe via `requestId`
dedup. Do **not** blindly retry `POST /session/v1/login-request` on 401 — fix the
credentials first ([authentication.md](authentication.md)).

---

## Troubleshooting

### Symptom: query hangs, no response

1. Confirm which path (look for `asyncExec=true`); for async, watch the
   `GET /queries/{id}/result` poll loop.
2. Check timeouts — the CRL-fetch client has its own timeout (`crl_http_timeout`);
   query calls are bounded by `query_timeout` (0 = unbounded), not `request_timeout`.
3. The query may be queued server-side — check the warehouse state.
4. DEBUG logging shows poll intervals and response codes.

**Resolution:** cancel the statement through the wrapper's cancel API (the core sends
`POST /queries/v1/abort-request` for the `requestId`), cancel from a fresh session
with `SYSTEM$CANCEL_QUERY`, or drop the statement handle.

### Symptom: a legitimately long-running query (not hung)

A query that is genuinely doing work — not queued, not hung — is a different case
from the hang above.

- **Prefer submit-and-poll over one long blocking call.** A synchronous execute
  holds a single HTTP request open for the entire run, exposing it to the client
  request timeout and to any proxy / load-balancer idle timeout. The async path
  (submit with `asyncExec=true`, then poll `GET /queries/{query_id}/result` — see
  [Execution paths](#execution-paths)) returns a `queryId` up front and polls, so
  no single request is held open for the duration. Whether that path is automatic
  or an explicit call depends on the wrapper's API.
- **`retry_timeout` does not bound query runtime.** It bounds one HTTP attempt
  ([Retries & 503s](#retries--503s)), not how long a running query may take; setting
  it low will not "give up" on a slow query (the query is not failing, it is
  running) and does not cancel one. `query_timeout` bounds the client-side wait. To cap server-side runtime use
  `STATEMENT_TIMEOUT_IN_SECONDS` (see the keep-alive-vs-query-timeout note in
  [authentication.md](authentication.md)); to stop one in flight, cancel it (above).

### Symptom: error 612 (`AsyncPollResultNotFound`)

After an `asyncExec=true` submit, the first poll can return 612 before the result
is written; the driver falls back to the sync path automatically. If 612 reaches
the caller, the fallback itself failed — check the sync-path error and network
reliability in the logs.

### Symptom: multi-statement returns the wrong number of child results

Child query IDs come from the response `resultIds`
(`sf_core/src/apis/database_driver_v1/multistatement.rs`). Confirm
`MULTI_STATEMENT_COUNT` matches the number of `;`-separated statements, and that
the wrapper iterates every child ID (otherwise child results are silently
dropped).

### Symptom: bind-stage upload fails before the query runs

Large bindings are uploaded as CSV to an internal bind stage before the query is
sent (`sf_core/src/stage_binding.rs`); failures surface as **stage** errors — see
[stage-cloud-storage.md](stage-cloud-storage.md).

### Symptom: OOM on a large result set

Reduce `CLIENT_MEMORY_LIMIT` (earlier back-pressure) and/or
`CLIENT_PREFETCH_THREADS`; stream-consume rather than buffering the whole result.
If the result set actually *fits* in memory but is merely **slow**, that is a
throughput problem, not OOM — see the next symptom, where the same two knobs move
in the **opposite** direction.

### Symptom: slow (not memory-bound) large result set

A large result set that downloads slowly but does **not** exhaust memory is
throughput-bound on chunk fetch. Additional chunks are presigned URLs pulled in
parallel up to `CLIENT_PREFETCH_THREADS` (default 4), so the fix is the reverse of
the OOM case:

- **Raise `CLIENT_PREFETCH_THREADS`** so more chunks download concurrently.
- Give `CLIENT_MEMORY_LIMIT` **headroom** — it back-pressures prefetch when
  exceeded (see [Result fetch](#result-fetch)), so if it is set too low it can
  throttle downloads even after you raise the thread count; raising threads
  without memory headroom has little effect.
- Confirm the bottleneck is fetch, not the warehouse: a query still *running*
  server-side is a different problem (see "query hangs, no response" above).

### Symptom: chunk download fails mid-result

A presigned chunk URL expired before download (result fetch slower than the URL
lifetime) or a transient network error. Re-run the query; look for chunk-download
errors with the URL and HTTP status in DEBUG logs.

### Symptom: Arrow IPC parse error / result type mismatch

Two different failures wear this label:

- **Parse error** — the parser rejected a chunk as invalid Arrow IPC (or invalid
  gzip). This is almost always a **truncated or corrupted chunk**, the same class
  as "chunk download fails mid-result" above: a partial download, or a proxy /
  transcoding layer that rewrote the compressed stream (see
  [proxy-tls.md](crl-tls/proxy-tls.md)). Re-run; in DEBUG logs find the specific
  chunk URL, its HTTP status, and byte size.
- **Type mismatch** — parsing succeeded but a column's Arrow type is not what the
  application expected. Type mapping happens in the wrapper, not the core, so this
  is a wrapper-layer concern — e.g. Arrow→pandas in
  [python.md](../wrappers/python.md). The core delivers the `RecordBatch` stream
  described in [Result fetch](#result-fetch) unchanged.

If you encounter such an issue, report it to us please, as it might be a driver bug.

### Symptom: rows disappear after re-executing the same statement

A statement handle **can** be re-executed without closing it, but doing so while a
prior result set is only partially read **discards** the unread rows: the new
execution cancels the pending result set. Fully consume (or explicitly close) a
result set before calling execute again on the same handle. Using a handle after
it has been freed yields an `InvalidArgument` error ("Statement handle not found") — the wrapper owns handle
lifetime; there is no reference counting on the core side.

### Symptom: the same DML appears to run twice / duplicate side effects

The driver's own retries do **not** double-execute: every retry of a
`POST /queries/v1/query-request` reuses one stable `requestId`, and the server
joins the retry to the already-running query instead of launching a second one
(see [Retries & 503s](#retries--503s)). Duplicates almost always come from a retry
the driver did *not* make:

- **The application caught an error and re-submitted.** A fresh submission carries a
  new `requestId`, so the server does not dedup it against the first. If the first
  attempt had already committed server-side (for example the connection dropped
  *after* the commit but before the response arrived), the re-submit applies the
  DML a second time.
- **A statement handle was re-executed** after a transient failure, for the same
  reason.

Make the DML **idempotent** so a re-submit is harmless — for example `MERGE` keyed
on a business key (the canonical upsert), `INSERT … SELECT … WHERE NOT EXISTS (…)`,
`CREATE … IF NOT EXISTS`, or a conditional `UPDATE … WHERE`. Note Snowflake does
**not** enforce `PRIMARY KEY` / `UNIQUE` constraints, so they will not stop a
duplicate insert on their own. Correlate with the server using the `queryId` (see
[Request / query correlation](#request--query-correlation)): two distinct
`queryId`s for what should be one operation confirms a duplicate submission.
