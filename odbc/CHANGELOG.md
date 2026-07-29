# Changelog

## Upcoming Release

Bug fixes:

- Fixed server-side SQL errors (e.g. Snowflake code 002003 "object does not exist") surfacing as the opaque "Received core protobuf error" with native code 0. The driver now forwards the server's original message, numeric error code (`vendor_code`), and SQLSTATE to `SQLGetDiagRec`. (snowflake-eng/universal-driver#773)

New features:

- Added cross-thread `SQLCancel` / `SQLCancelHandle` support: both now fire a server-side abort for the statement's in-flight query (via the core `StatementCancel` RPC) before signalling the local cancellation token, so a query executing on another thread is actually stopped on the server. The canceled query surfaces `HY008` whether the server abort or the local token wins the race. `SQLCancelHandle(SQL_HANDLE_STMT)` delegates to the same path as `SQLCancel`. (snowflakedb/universal-driver#629)
- Added `SQLColumns` and `SQLColumnsW` support for querying column metadata via ODBC catalog functions. (snowflakedb/universal-driver#369)
- Implemented `SQLSpecialColumns`, `SQLColumnPrivileges`, `SQLTablePrivileges`, and `SQLStatistics` catalog functions. Snowflake does not expose row identifiers, version columns, column/table-level privilege metadata, or index statistics over ODBC, so all four return a correctly-structured empty result set matching the reference driver's behavior. (snowflakedb/universal-driver#386)
- Implemented `SQLPrimaryKeys` and `SQLPrimaryKeysW` catalog functions using `SHOW PRIMARY KEYS`. (snowflakedb/universal-driver#455)
- Implemented `SQLForeignKeys` and `SQLForeignKeysW` catalog functions using `SHOW IMPORTED KEYS` / `SHOW EXPORTED KEYS`. (snowflakedb/universal-driver#456)
- Implemented `SQLProcedures` and `SQLProceduresW` catalog functions using `information_schema.procedures`. (snowflakedb/universal-driver#534)
- Implemented `SQLProcedureColumns` and `SQLProcedureColumnsW` catalog functions using `information_schema.procedures`, parsing the argument signature and return type into per-column metadata. (snowflakedb/universal-driver#535)

Bug fixes:

- Fixed a priority inversion where API-usage telemetry could block on the connection mutex for the full duration of a synchronous query; the connection handle is now read from a lock-free, best-effort atomic telemetry cache so telemetry (and cross-thread `SQLCancel`) never waits on an in-flight query. (snowflakedb/universal-driver#631)
- Fixed `SQLDisconnect` to return SQLSTATE 25000 (invalid transaction state) and keep the connection open when a manual-commit transaction is still in process, matching the ODBC specification and the reference driver. (snowflakedb/universal-driver#754)
