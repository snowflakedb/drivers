---
category: bugfix
---

- Fixed `SQLColumns` and `SQLGetTypeInfo` `NUM_PREC_RADIX` reporting INTEGER instead of the SMALLINT type the ODBC specification defines. (snowflakedb/drivers#2760)
