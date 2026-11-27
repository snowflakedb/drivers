---
category: bugfix
---
- Fixed Windows Credential Manager rejecting ASCII OAuth tokens over about 1280 characters by storing them as UTF-8 bytes. (snowflakedb/drivers#3352)
