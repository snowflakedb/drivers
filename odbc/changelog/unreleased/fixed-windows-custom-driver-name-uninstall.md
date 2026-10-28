---
category: bugfix
---

- Fixed the Windows installer so uninstalling or upgrading a driver registered under a custom `DRIVER_NAME` removes that registration when this version or a later one installed it and leaves any other `Snowflake ODBC` registration in place, while an upgrade from an earlier installer still removes the default `Snowflake ODBC` registration unless that version is uninstalled with its original `DRIVER_NAME` first. (snowflakedb/drivers#2963)
