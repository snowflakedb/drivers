# Changelog

## Upcoming Release

New features:

Bug fixes:

- Fixed `SQLCancelHandle(SQL_HANDLE_DBC)` to return SQLSTATE HY010 when an associated statement is asynchronously executing or mid data-at-execution, matching the ODBC 3.8 Diagnostics table (previously a no-op like the reference driver). (snowflakedb/universal-driver#871)
- Fixed the `SecondaryRoles` connection attribute being silently dropped: the connection-string parser uppercases keys with no separator preserved, so `SecondaryRoles=None;` arrived as `SECONDARYROLES` and was never mapped to the shared `secondary_roles` parameter. It is now mapped correctly, restoring parity with legacy ODBC. (snowflakedb/universal-driver#954)

## v0.0.8

New features:

- Implemented `SQLSetCursorName` and `SQLSetCursorNameW`: assigns a client-side cursor-name label to a statement handle. (snowflake-eng/universal-driver#758)
- Implemented `SQLGetCursorName` and `SQLGetCursorNameW`: returns the cursor name assigned via `SQLSetCursorName`, or a driver-generated `SQL_CUR`-prefixed name if none was set. (snowflake-eng/universal-driver#759)
- Added cross-thread `SQLCancel` / `SQLCancelHandle` support: both now fire a server-side abort for the statement's in-flight query (via the core `StatementCancel` RPC) before signalling the local cancellation token, so a query executing on another thread is actually stopped on the server. The canceled query surfaces `HY008` whether the server abort or the local token wins the race. `SQLCancelHandle(SQL_HANDLE_STMT)` delegates to the same path as `SQLCancel`. (snowflakedb/universal-driver#629)
- Implemented `SQLTransact` (ODBC 2.x) mapping to `SQLEndTran` for direct-link and ODBC 2.x applications that bypass the Driver Manager. (snowflakedb/universal-driver#TBD)

Bug fixes:

- Fixed server-side SQL errors (e.g. Snowflake code 002003 "object does not exist") surfacing as the opaque "Received core protobuf error" with native code 0. The driver now forwards the server's original message, numeric error code (`vendor_code`), and SQLSTATE to `SQLGetDiagRec`. (snowflake-eng/universal-driver#773)
- Fixed a priority inversion where API-usage telemetry could block on the connection mutex for the full duration of a synchronous query; the connection handle is now read from a lock-free, best-effort atomic telemetry cache so telemetry (and cross-thread `SQLCancel`) never waits on an in-flight query. (snowflakedb/universal-driver#631)
- Fixed `SQLDisconnect` to return SQLSTATE 25000 (invalid transaction state) and keep the connection open when a manual-commit transaction is still in process, matching the ODBC specification and the reference driver. (snowflakedb/universal-driver#754)
