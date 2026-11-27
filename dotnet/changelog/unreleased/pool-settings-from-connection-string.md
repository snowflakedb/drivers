---
category: feature
---

- Added reading and validation of the pool settings `MinPoolSize`, `MaxPoolSize`, `ChangedSession`, `WaitingForIdleSessionTimeout`, `ExpirationTimeout`, `PoolingEnabled`, and `connection_timeout` from the connection string, so `SnowflakeDbConnectionPool` reports them per pool and rejects invalid values. (snowflakedb/drivers#3596)
