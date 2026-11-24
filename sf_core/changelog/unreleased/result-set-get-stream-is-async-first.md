---
category: feature
---

- Changed `ResultSetGetStream` to `async_first`, so cancelling it from another thread stops waiting on the result's first remote chunk download instead of blocking until the server answers, and issues no abort-request because opening a stream submits no query (snowflakedb/drivers#3512).
