---
category: bugfix
---
- Fixed result chunk download retries ignoring `maxHttpRetries` (`retry_max_attempts`) and `retry_backoff_*` connection settings. (snowflakedb/drivers#3514)
