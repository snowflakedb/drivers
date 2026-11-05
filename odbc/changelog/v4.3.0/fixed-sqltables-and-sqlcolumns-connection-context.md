---
category: bugfix
---

- Fixed `SQLTables` and `SQLColumns` so that, with `CLIENT_METADATA_REQUEST_USE_CONNECTION_CTX` enabled, a non-NULL catalog argument such as `%` is no longer limited to the current schema, and an unset session database or schema no longer makes the call return no rows. (snowflakedb/drivers#2862)
