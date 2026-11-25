---
category: breaking
---

- Removed the public parameterless constructor of `SnowflakeDbConnectionPool`, so `new SnowflakeDbConnectionPool()` and classes deriving from it no longer compile; use its static members instead. (snowflakedb/drivers#3595)
