---
category: feature
---

- Changed `ConnectionUploadStreamFinish` to `async_first`, so cancelling it from another thread aborts the in-flight PUT on the server, or aborts an S3 multipart upload so no parts are left on the stage, instead of blocking until the upload finishes. (snowflakedb/drivers#3541)
