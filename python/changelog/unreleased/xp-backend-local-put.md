---
category: feature
---

- Added support for uploading files to stages inside Python stored procedures through `session.file.put`, `session.file.put_stream`, and `cursor.upload_stream`, sending the bytes through the host instead of cloud storage. (snowflakedb/drivers#3410)
