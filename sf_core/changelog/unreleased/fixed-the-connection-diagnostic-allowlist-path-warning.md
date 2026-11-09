---
category: bugfix
---

- Fixed the connection diagnostic warning for an invalid `connection_diag_allowlist_path`, which claimed the driver would fetch the allowlist from Snowflake; it now says only the account host is probed and how to supply the allowlist file. The report no longer says the driver could not connect to Snowflake to get the allowlist. (snowflakedb/drivers#3140)
