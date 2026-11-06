---
category: feature
---

- Changed `ConnectionClose` and `ConnectionAbortQuery` to `async_first`, so cancelling either from another thread drops the in-flight logout or abort-request instead of blocking until the server answers. A cancelled close leaves the connection open. Cancelling an abort-request does not send a second one (snowflakedb/drivers#3165).
