---
category: bugfix
---
- Fixed result set fetches hanging indefinitely when a chunk download stalls; downloads now time out after 30 seconds without data and are retried, configurable with `chunk_download_read_timeout`. (snowflakedb/drivers#3515)
