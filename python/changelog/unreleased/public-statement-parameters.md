---
category: feature
---

- Added a public `statement_parameters` argument on `execute` and `execute_async` for per-query parameters such as `CLIENT_RESULT_CHUNK_SIZE`, with `_statement_params` still working and warning to use `statement_parameters`. (snowflakedb/drivers#3094)
