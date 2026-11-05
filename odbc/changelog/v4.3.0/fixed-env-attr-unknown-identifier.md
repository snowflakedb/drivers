---
category: bugfix
---

- Fixed `SQLGetEnvAttr` and `SQLSetEnvAttr` of an unrecognized environment attribute returning HYC00 instead of HY092. (snowflakedb/drivers#3179)
