---
category: bugfix
---

- Fixed `SQLPrepare`, `SQLColumns`, and `SQLProcedureColumns` ignoring `VARCHAR_AND_BINARY_MAX_SIZE_IN_RESULT`, `DEFAULT_VARCHAR_SIZE`, `DEFAULT_BINARY_SIZE`, and account- or user-level `ODBC_*` type-mapping parameters until the first statement executed on a connection. (snowflakedb/drivers#3487)
