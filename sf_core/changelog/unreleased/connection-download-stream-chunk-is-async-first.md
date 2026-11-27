---
category: feature
---

- Changed `ConnectionDownloadStreamChunk` to `async_first`, so cancelling a chunk read from another thread returns CANCELLED instead of waiting for the cloud download, aborts the download, and makes later chunk reads on that stream fail with an invalid-argument error until it is closed. (snowflakedb/drivers#3533)
