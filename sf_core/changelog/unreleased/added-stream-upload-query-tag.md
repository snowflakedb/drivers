---
category: feature
---

- Added `connection_upload_stream_begin_with_query_tag` so Rust callers can attach a per-upload query tag to a streamed PUT and any stage-info refresh it triggers (expired credentials or a per-file presigned URL) without changing connection state (snowflakedb/drivers#1397).
