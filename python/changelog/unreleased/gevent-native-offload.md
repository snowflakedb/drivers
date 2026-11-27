---
category: bugfix
---

- Fixed queries and result fetches blocking every request in a gunicorn gevent worker; set `SNOWFLAKE_GEVENT_THREADPOOL_SIZE` (default 32) to at least the number of concurrent Snowflake calls per worker. (snowflakedb/drivers#3535)
