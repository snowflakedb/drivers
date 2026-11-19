---
category: bugfix
---

- Fixed a synchronous query that is still running at its second or later session-token expiry failing with `390112 Session expired - reauthentication required`; the driver now refreshes the session at every expiry while it waits for the result. (snowflakedb/drivers#3315)
- Fixed a session-token expiry while waiting on an async-execution-mode query or a metadata (`SHOW`) query re-sending the statement; the driver now retries only the request that hit the expiry and keeps waiting on the same query. (snowflakedb/drivers#3315)
