---
category: bugfix
---

- Fixed type hints so `cursor(DictCursor)` remains a `DictCursor` through `execute` and `executemany`, its fetch results and `next()` are dictionaries, and the default `cursor()` is narrowed to a tuple cursor, for both the sync and async cursors. (snowflakedb/drivers#2872)
