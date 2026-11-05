---
category: bugfix
---

- Fixed column-wise `SQLBindCol` arrays bound as `SQL_C_DEFAULT` with a zero `BufferLength` writing every fetched row into the first array element instead of advancing by the size of the default C type. (snowflakedb/drivers#3027)
