---
category: bugfix
---

- Fixed `SQLBindParameter` with `SQL_C_DEFAULT` and `SQL_REAL` reading the bound value as an 8-byte double instead of a 4-byte `SQL_C_FLOAT`, which inserted wrong values and misread parameter arrays. (snowflakedb/drivers#3010)
