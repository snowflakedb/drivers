---
category: bugfix
---

- Fixed native Arrow text fetches so a value that is not valid UTF-8 raises InterfaceError instead of aborting the iterator. (snowflakedb/drivers#3241)
