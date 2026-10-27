---
category: bugfix
---

- Fixed catalog function SMALLINT and INTEGER result columns reporting NUMBER display size, octet length, and type names. `SQL_DESC_LOCAL_TYPE_NAME` on those columns now matches the 3.x driver (`SSMALLINT`, `USMALLINT`, `SINTEGER`), and unsigned `USER_DATA_TYPE` reports display size 5. Catalog wide-character columns report octet length as four bytes per character, and `SQLSpecialColumns` `COLUMN_NAME` reports character length 255. (snowflakedb/drivers#2760)
