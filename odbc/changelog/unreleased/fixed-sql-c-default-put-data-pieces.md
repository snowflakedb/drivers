---
category: bugfix
---

- Fixed `SQLPutData` rejecting character and binary data bound as `SQL_C_DEFAULT` with `HY019` when it was sent in more than one piece. (snowflakedb/drivers#3027)
