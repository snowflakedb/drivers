# ODBC concurrency

ODBC builds on the [core contract](core.md). The ODBC wrapper is **stricter**: one in-flight session operation per
connection.

ODBC 3.x requires thread-safe handles and allows (but does not require) the driver to serialize. This driver serializes.

## What this means

- Two statements on the same connection do not overlap. The second caller waits.
- Read-only `SELECT`s are queued the same way as `USE` / `COMMIT`.
- Cancel is still non-blocking and is not gated (see [core](core.md#cancel)).
- `SQL_ATTR_ASYNC_ENABLE` is the exception: the driver returns `SQL_STILL_EXECUTING` and drops the
  connection lock while the query is in flight, so a second statement on the same `SQLHDBC` can start.
- Concurrent work belongs on a **connection pool**, not on one shared `SQLHDBC`.
