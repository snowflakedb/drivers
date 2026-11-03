---
category: bugfix
---

- Fixed `SQL_C_DEFAULT` on `SQL_WCHAR`, `SQL_WVARCHAR`, and `SQL_WLONGVARCHAR` treating data as narrow characters instead of `SQL_C_WCHAR`, which truncated or corrupted bound wide-string parameters and returned catalog function wide-character columns as narrow strings. (snowflakedb/drivers#3013)
