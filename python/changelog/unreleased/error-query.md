---
category: bugfix
---

- Fixed `ProgrammingError.query` being `None` for errors from `execute` and `describe`; it now holds the SQL that was sent, as in the old driver. (snowflakedb/drivers#3571)
