---
category: bugfix
---

- Fixed JSON chunks silently dropping truncated rows or accepting garbage, missing row separators, leading commas, or repeated commas between rows. Invalid UTF-8 values now use replacement characters, and downloaded JSON chunks reject mismatched row counts. (snowflakedb/drivers#1398)
