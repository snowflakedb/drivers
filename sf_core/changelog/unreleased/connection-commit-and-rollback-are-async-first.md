---
category: feature
---

- Changed `ConnectionCommit` and `ConnectionRollback` to `async_first`, so cancelling either from another thread aborts the in-flight statement instead of blocking until the server answers, and leaves the session cache unchanged (snowflakedb/drivers#3186).
