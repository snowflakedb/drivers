---
category: feature
---

- Changed `DatabaseFetchChunk` to `async_first`, so cancelling it from another thread stops in-flight chunk downloads and an inline decode still waiting behind a memory-budget wait, instead of blocking until the server answers, and issues no abort-request because a chunk fetch submits no query (snowflakedb/drivers#3158)
