---
category: feature
---

- Added `SnowflakeDbConnectionPool.GetPoolPartition`, which returns the pool for a connection string with its settings as properties (`MaxPoolSize`, `MinPoolSize`, `CurrentPoolSize`, `PoolingEnabled`, `ExpirationTimeout`, `ConnectionTimeout`, `WaitingForIdleSessionTimeout`, `ChangedSession`) and `Clear()`. (snowflakedb/drivers#3595)
