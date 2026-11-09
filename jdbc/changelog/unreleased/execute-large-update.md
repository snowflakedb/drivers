---
category: bugfix
---

- Fixed `Statement.executeLargeUpdate`, `Statement.getLargeUpdateCount()`, and `PreparedStatement.executeLargeUpdate()` throwing `UnsupportedOperationException`: they now return the affected-row count as a long, and `Statement.executeLargeUpdate` with generated-keys flags, column indexes, or column names runs only with `NO_GENERATED_KEYS` and throws `SQLFeatureNotSupportedException` for the rest, as in JDBC 4.x. (snowflakedb/drivers#3225)
