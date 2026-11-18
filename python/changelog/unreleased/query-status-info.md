---
category: feature
---

- Added `Connection.get_query_status_info` (sync and async) returning a `QueryStatusInfo` object with server-reported timings, warehouse details, and error fields, while `get_query_status` continues to return the `QueryStatus` enum. (snowflakedb/drivers#3377)
