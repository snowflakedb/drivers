---
category: bugfix
---
- Fixed an issue where `write_pandas` accepted column names that are not valid unquoted identifiers when `quote_identifiers=False`. (snowflakedb/drivers#2977)
