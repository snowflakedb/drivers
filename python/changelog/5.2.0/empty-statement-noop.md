---
category: bugfix
---
- Fixed `cursor.execute()` raising `000006: Invalid Snowflake response` for an empty or whitespace-only statement; it now skips the statement with a warning and returns the cursor, as the 4.x connector did. (snowflakedb/drivers#2970)
