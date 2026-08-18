# Changelog

## Upcoming Release

New features:

- Added VECTOR type fetch support: VECTOR columns are now returned as compact JSON array strings (e.g. `[1,2,3]`) via `SQL_C_CHAR`, `SQL_C_WCHAR`, and `SQL_C_BINARY`, matching the ARRAY/VARIANT pattern. `SQLDescribeCol` reports `SQL_VARCHAR`; `SQL_DESC_TYPE_NAME` returns `"VECTOR"`. (snowflake-eng/universal-driver#1010)

Changes:

- Renamed the default ODBC driver registration name from `Snowflake ODBC UD` to `Snowflake ODBC` (custom name still supported via `DRIVER_NAME=` / `SF_DRIVER_NAME`). (snowflake-eng/drivers#1143)
- Renamed Linux/macOS packages from `snowflake-odbc-ud` to `snowflake-odbc` and the macOS install path from `/opt/snowflake/snowflakeodbcud` to `/opt/snowflake/snowflakeodbc`. (snowflake-eng/drivers#1143)
- Dropped `PrPr`/`Private Preview` labels from the Windows MSI product name, DLL version resource, and setup dialog title. (snowflake-eng/drivers#1143)
- Changed `SQLGetInfo(SQL_DRIVER_VER)` to return the zero-padded fixed-width `MM.mm.bbbb` string (e.g. `04.00.0000`) defined by the ODBC spec (`##.##.####`), instead of the unpadded Cargo semver (e.g. `4.0.0`). (snowflakedb/drivers#1074)
- Changed distributable ODBC package filenames to use a consistent `<version>.<architecture>.<extension>` pattern across Linux, macOS, and Windows. (snowflake-eng/drivers#TBD)

Bug fixes:

- Fixed `SQLFreeHandle(SQL_HANDLE_DESC)` on an implicitly allocated descriptor to return SQLSTATE HY017 with the handle left valid, matching the ODBC specification (previously returned `SQL_INVALID_HANDLE` with no diagnostic). (snowflakedb/drivers#1165)
- Fixed `SQLColumns` `DATA_TYPE` / `SQL_DATA_TYPE` to fall back to `SQL_VARCHAR` for unmapped catalog types (e.g. GEOGRAPHY, GEOMETRY) instead of returning NULL. (snowflake-eng/drivers#1156)
- Fixed `SQLColumns` `NUM_PREC_RADIX` for FLOAT/DOUBLE/REAL to return `10`, matching the reference driver and decimal `COLUMN_SIZE` (query-result `SQLColAttribute` radix for DOUBLE remains `2`). (snowflake-eng/drivers#1146)
- Fixed `SQLGetInfo(SQL_DRIVER_NAME)` to return the driver library file name (e.g. `libsfodbc.so`) instead of the full on-disk path, matching the ODBC specification. (snowflake-eng/drivers#1076)
- Fixed `SQLColumns` `USER_DATA_TYPE` (column 19) to always return `0` (`UDT_STANDARD_SQL_TYPE`), matching the reference driver (previously mirrored `DATA_TYPE`). (snowflakedb/drivers#1099)
- Fixed `SQLTables` / `SQLColumns` catalog result IRDs so string columns report `SQL_WVARCHAR` and numeric `SQLColumns` columns report `SQL_SMALLINT` / `SQL_INTEGER`, matching the reference driver (previously all catalog result columns were labeled `SQL_VARCHAR`). (snowflakedb/drivers#1085)
- Fixed mismatched ODBC handle types (e.g. a statement handle passed where a connection handle is expected) to correctly return `SQL_INVALID_HANDLE`. (snowflakedb/drivers#1040)
- Fixed `SQLCancelHandle(SQL_HANDLE_DBC)` to return SQLSTATE HY010 when an associated statement is asynchronously executing or mid data-at-execution, matching the ODBC 3.8 Diagnostics table (previously a no-op like the reference driver). (snowflakedb/drivers#871)
- Fixed the `SecondaryRoles` connection attribute being silently dropped: the connection-string parser uppercases keys with no separator preserved, so `SecondaryRoles=None;` arrived as `SECONDARYROLES` and was never mapped to the shared `secondary_roles` parameter. It is now mapped correctly, restoring parity with legacy ODBC. (snowflakedb/drivers#954)

## v0.0.8

New features:

- Implemented `SQLSetCursorName` and `SQLSetCursorNameW`: assigns a client-side cursor-name label to a statement handle. (snowflake-eng/drivers#758)
- Implemented `SQLGetCursorName` and `SQLGetCursorNameW`: returns the cursor name assigned via `SQLSetCursorName`, or a driver-generated `SQL_CUR`-prefixed name if none was set. (snowflake-eng/drivers#759)
- Added cross-thread `SQLCancel` / `SQLCancelHandle` support: both now fire a server-side abort for the statement's in-flight query (via the core `StatementCancel` RPC) before signalling the local cancellation token, so a query executing on another thread is actually stopped on the server. The canceled query surfaces `HY008` whether the server abort or the local token wins the race. `SQLCancelHandle(SQL_HANDLE_STMT)` delegates to the same path as `SQLCancel`. (snowflakedb/drivers#629)
- Implemented `SQLTransact` (ODBC 2.x) mapping to `SQLEndTran` for direct-link and ODBC 2.x applications that bypass the Driver Manager. (snowflakedb/drivers#TBD)

Bug fixes:

- Fixed server-side SQL errors (e.g. Snowflake code 002003 "object does not exist") surfacing as the opaque "Received core protobuf error" with native code 0. The driver now forwards the server's original message, numeric error code (`vendor_code`), and SQLSTATE to `SQLGetDiagRec`. (snowflake-eng/drivers#773)
- Fixed a priority inversion where API-usage telemetry could block on the connection mutex for the full duration of a synchronous query; the connection handle is now read from a lock-free, best-effort atomic telemetry cache so telemetry (and cross-thread `SQLCancel`) never waits on an in-flight query. (snowflakedb/drivers#631)
- Fixed `SQLDisconnect` to return SQLSTATE 25000 (invalid transaction state) and keep the connection open when a manual-commit transaction is still in process, matching the ODBC specification and the reference driver. (snowflakedb/drivers#754)
