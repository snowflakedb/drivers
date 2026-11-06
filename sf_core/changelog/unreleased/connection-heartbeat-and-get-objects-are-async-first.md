---
category: feature
---

- Changed `ConnectionHeartbeat` and `ConnectionGetObjects` to `async_first`, so cancelling either from another thread stops waiting for the server, a cancelled heartbeat issues no abort-request, and a cancelled `ConnectionGetObjects` aborts each in-flight metadata `SHOW` by the request id it was submitted with. (snowflakedb/drivers#3154)
