---
category: bugfix
---

- Fixed the driver resubmitting a running query when the session token expired during a long-running server-side poll; the driver now refreshes the token and retries only the result poll. (snowflakedb/drivers#3127)
