---
category: bugfix
---

- Fixed `SQLProcedureColumns` so a FILE argument reports the session VARCHAR maximum for `COLUMN_SIZE` and `BUFFER_LENGTH`, matching VARIANT, when `DEFAULT_VARCHAR_SIZE` is set. (snowflakedb/drivers#3008)
