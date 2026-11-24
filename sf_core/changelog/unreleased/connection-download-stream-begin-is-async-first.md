---
category: feature
---

- Changed `ConnectionDownloadStreamBegin` to `async_first`, so cancelling it from another thread aborts the in-flight GET on the server, or drops the cloud download it is opening without an abort-request, instead of blocking until the server answers, and opens no download session (snowflakedb/drivers#3526).
