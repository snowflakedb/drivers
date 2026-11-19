---
category: change
---

- Stopped reporting `SQL_NO_DATA`, `SQL_NEED_DATA` and `SQL_STILL_EXECUTING` as wrapper errors in telemetry, which removes a runtime round trip from the end of every result set and from every async poll. (snowflakedb/drivers#3362)
