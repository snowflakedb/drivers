# JDBC concurrency

JDBC builds on the [core contract](core.md), `serialize_session_operations` is `false` by default. Concurrent statements
on one connection run overlapped.

Only `Connection` is meant to be shared across threads. A `Statement` and a `ResultSet` are each single-owner.

## What this means

- Two `Statement`s on the same `Connection` may execute at the same time.
- A single `Statement` is not thread-safe. Re-executing a statement while another thread still holds its `ResultSet` is
  unsafe.
- A `ResultSet` is not thread-safe, it's single-consumer: one thread owns iteration of that instance. Two `ResultSet`s
  from two `Statement`s on the same connection may be consumed in parallel.
- Parallel consumption of one query's rows is `SnowflakeResultSet.getResultSetSerializables()`: each partition may be
  deserialized and iterated on its own thread. Each reconstructed `ResultSet` is still single-owner.
- Session-mutating SQL (`ALTER SESSION`, `USE`, `COMMIT`, ...) and connection setters that change session state
  (`setAutoCommit`, `setCatalog`, ...) on a shared connection are racy. See
  [core - session mutations](core.md#session-mutations).
- Cancel is safe from another thread (see [core](core.md#cancel)). That is the only intended cross-thread use of a
  `Statement`.
- A pool is still the right tool for concurrent application work: transactions and session state are per connection.
