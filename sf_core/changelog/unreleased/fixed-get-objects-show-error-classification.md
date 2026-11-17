---
category: bugfix
---

- Fixed GetObjects returning empty metadata for SHOW failures other than a missing, unauthorized, or no-longer-shared object, such as syntax errors. Only Snowflake error codes 2003, 2043 and 3030 now produce an empty catalog, schema, or table. (snowflakedb/drivers#1396)
