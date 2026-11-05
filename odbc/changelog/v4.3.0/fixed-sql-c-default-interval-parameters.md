---
category: bugfix
---

- Fixed `SQLBindParameter` with `SQL_C_DEFAULT` and a `SQL_INTERVAL_*` parameter type reading the bound `SQL_INTERVAL_STRUCT` as a character string instead of as the matching `SQL_C_INTERVAL_*` type. (snowflakedb/drivers#3020)
