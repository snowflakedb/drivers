---
category: change
---

- Marked `SnowflakeDbConnectionPool.GetPool` and `SnowflakeDbSessionPool` as obsolete in favor of `SnowflakeDbConnectionPool.GetPoolPartition`, so calls to them raise compiler warning CS0618. (snowflakedb/drivers#3595)
