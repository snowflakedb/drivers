---
category: feature
behavior_differences: [BD#db-exception-sql-state-overrides-base]
---

- Added a working `SnowflakeDbException` that carries the vendor code, SQL state and query ID of driver and server failures, and returns its SQL state through `DbException.SqlState` on .NET 5 and later. (snowflakedb/drivers#3599)
