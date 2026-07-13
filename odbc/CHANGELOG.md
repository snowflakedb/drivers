# Changelog

## Upcoming Release

New features:

- Added `SQLColumns` and `SQLColumnsW` support for querying column metadata via ODBC catalog functions. (snowflakedb/universal-driver#369)
- Implemented `SQLSpecialColumns`, `SQLColumnPrivileges`, `SQLTablePrivileges`, and `SQLStatistics` catalog functions. Snowflake does not expose row identifiers, version columns, column/table-level privilege metadata, or index statistics over ODBC, so all four return a correctly-structured empty result set matching the reference driver's behavior. (snowflakedb/universal-driver#386)
- Implemented `SQLPrimaryKeys` and `SQLPrimaryKeysW` catalog functions using `SHOW PRIMARY KEYS`. (snowflakedb/universal-driver#455)
- Implemented `SQLForeignKeys` and `SQLForeignKeysW` catalog functions using `SHOW IMPORTED KEYS` / `SHOW EXPORTED KEYS`. (snowflakedb/universal-driver#456)
- Implemented `SQLProcedures` and `SQLProceduresW` catalog functions using `information_schema.procedures`. (snowflakedb/universal-driver#TBD)
