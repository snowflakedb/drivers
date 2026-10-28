---
category: change
---

- Changed query-result `SQL_DESC_TYPE_NAME` for FILE columns to report `FILE`. `SQLDescribeCol` stays `SQL_VARCHAR`, including when `ODBC_USE_CUSTOM_SQL_DATA_TYPES` is enabled. (snowflakedb/drivers#2833)
