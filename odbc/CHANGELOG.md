# Changelog

## Upcoming Release

New features:

- Added `SQLColumns` and `SQLColumnsW` support for querying column metadata via ODBC catalog functions. (snowflakedb/universal-driver#369)
- Implemented `SQLSpecialColumns`, `SQLColumnPrivileges`, `SQLTablePrivileges`, and `SQLStatistics` catalog functions. Snowflake does not expose row identifiers, version columns, column/table-level privilege metadata, or index statistics over ODBC, so all four return a correctly-structured empty result set matching the reference driver's behavior. (snowflakedb/universal-driver#386)
