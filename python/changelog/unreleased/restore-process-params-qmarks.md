---
category: bugfix
---

- Restored `Connection._process_params_qmarks`, which Python stored procedures call to build bindings for `_snowflake.execute_sql`. It returns the same `{"1": {"type": ..., "value": ...}}` dict as the reference connector, or `None` when there are no parameters. (snowflakedb/drivers#3561)
