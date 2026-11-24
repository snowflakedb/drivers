---
category: bugfix
---

- Fixed stored procedures that use the XP backend: failed queries now raise the server's error instead of returning an empty result, array binds work in Snowpark procedures because the scoped bind stage is named `SNOWPARK_TEMP_STAGE_BIND`, and `write_pandas` creates scoped temporary objects (new `use_scoped_temp_object` option) so it works in owner's-rights procedures. (snowflakedb/drivers#3569)
