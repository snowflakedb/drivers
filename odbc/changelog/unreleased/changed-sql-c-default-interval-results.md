---
category: breaking
---

- Changed `SQLBindCol` and `SQLGetData` with `SQL_C_DEFAULT` on `INTERVAL` result columns to return a `SQL_INTERVAL_STRUCT` instead of the ANSI interval literal string, failing with `22015` when the leading field exceeds the interval leading precision (2 digits by default) and with `HY090` when a nonzero buffer length is smaller than the struct; binding `SQL_C_CHAR` still returns the string. (snowflakedb/drivers#3020)
