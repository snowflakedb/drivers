# Python concurrency

Python builds on the [core contract](core.md), `serialize_session_operations` is `false` by default. Concurrent
statements on one connection run overlapped.

The module advertises DB-API `threadsafety = 2`: threads may share the module and connections, but not cursors.

## What this means

- Several cursors on the same connection may execute at the same time. That is the intended path for the async API.
- A single cursor is not thread-safe. Do not share a cursor across threads.
- Only `Connection` is designed to be shared across threads. Other objects, like a `ResultBatch`, a fetch iterator, or a
  file-transfer stream are each single-owner: do not use the same instance from two threads. Different `ResultBatch`
  objects from one `get_result_batches()` may be processed in parallel.
- Session-mutating SQL (`ALTER SESSION`, `USE`, `COMMIT`, ...) on a shared connection is racy: the client cache can
  disagree with the backend. See [core - session mutations](core.md#session-mutations).
- Cancel is safe from another thread (see [core](core.md#cancel)).
- Independent connections (a pool) are still the right tool when you need isolation - transactions and session state are
  per connection, not per cursor.
