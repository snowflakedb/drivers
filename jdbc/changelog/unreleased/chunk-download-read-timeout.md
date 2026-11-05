---
category: bugfix
---

- Fixed result set fetches hanging indefinitely when a chunk download stalls; downloads now time out after 30 seconds without data and are retried, configurable with `chunkDownloadReadTimeout`. (snowflakedb/drivers#3202)
