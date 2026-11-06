---
category: feature
---

- Changed `ConnectionSetAutocommit`, `ConnectionUseDatabase`, and `ConnectionUseSchema` to `async_first`, so cancelling one while its statement is in flight aborts that statement instead of blocking until the server answers, and leaves the session cache unchanged (snowflakedb/drivers#3186).
