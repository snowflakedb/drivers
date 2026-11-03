---
category: internal
---

- Changed `SQL_C_DEFAULT` handling to resolve the C type once from a shared SQL-type table before parameter binding and result conversion, replacing the per-converter `SQL_C_DEFAULT` arms. (snowflakedb/drivers#3027)
