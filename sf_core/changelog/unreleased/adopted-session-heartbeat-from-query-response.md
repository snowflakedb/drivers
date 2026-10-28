---
category: feature
---

- Added heartbeats for a connection that adopts an existing session without the `CLIENT_SESSION_KEEP_ALIVE` option: they start after the first query whose response reports keep-alive on for the session. (snowflakedb/drivers#2984)
