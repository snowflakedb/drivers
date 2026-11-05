---
category: bugfix
---

- Fixed `SQLGetEnvAttr(SQL_ATTR_DRIVER_UNICODE_TYPE)` returning an unsupported-attribute error, so iODBC 3.52.14 and later converts wide strings to the driver's UTF-16 or UTF-32 encoding. (snowflakedb/drivers#3179)
