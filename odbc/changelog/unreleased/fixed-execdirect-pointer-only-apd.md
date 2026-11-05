---
category: bugfix
---

- Fixed SQLExecDirect failing with a parameter-binding error for statements that have no parameters when only a deferred length or indicator pointer was set. (snowflakedb/drivers#3045)
- Fixed SQLExecDirect treating SQL_DESC_OCTET_LENGTH_PTR as a null indicator on non-character parameters. (snowflakedb/drivers#3045)
