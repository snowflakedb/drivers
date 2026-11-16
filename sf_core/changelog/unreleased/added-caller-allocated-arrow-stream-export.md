---
category: feature
---

- Added the `ResultSetExportStream` and `ArrowStreamGetNextBatch` RPCs, which export a result set into an Arrow C stream the caller allocates and read its next batch on a blocking thread, so a wrapper that does not share sf_core's allocator owns and frees the stream itself. (snowflakedb/drivers#3376)
