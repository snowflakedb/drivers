# Node.js concurrency

Node.js builds on the [core contract](core.md), `serialize_session_operations` is `false` by default. Concurrent
statements on one connection run overlapped.

That matches the async API: several `connection.execute` calls may be in flight on the same connection.

## What this means

- `Promise.all` of several executes on one connection is supported for independent, read-only queries.
- A single `Statement` is not safe to fetch from concurrently. `streamRows()`, `fetchRows()`, and the default
  (buffered) `complete` callback share one row cursor on that instance. Do not start two fetches on the same
  statement. TODO: a second fetch throws an error with a dedicated message.
- The `Readable` from `streamRows()` is a single-consumer stream. Two streams from two statements on the same
  connection may be consumed in parallel.
- Session-mutating SQL (`ALTER SESSION`, `USE`, `COMMIT`, ...) on a shared connection is racy. See
  [core - session mutations](core.md#session-mutations).
- Cancel is safe from another call stack (see [core](core.md#cancel)).
- Independent connections are still the right tool when you need isolation - transactions and session state are per
  connection, not per statement.
