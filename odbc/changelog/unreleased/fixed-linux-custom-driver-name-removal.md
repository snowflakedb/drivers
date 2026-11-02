---
category: bugfix
---

- Fixed the Linux RPM and deb packages so removing a package unregisters the driver name recorded at install, including a custom `SF_DRIVER_NAME`, and an upgrade keeps that registration unless `SF_DRIVER_NAME` names a different driver, which replaces the old registration. (snowflakedb/drivers#2964)
- Fixed the Linux RPM upgrade so the driver and the `snowflake` DSN stay registered, while an upgrade from a driver version released before this fix registers `SF_DRIVER_NAME` or the default `Snowflake ODBC` name and leaves a different custom name from that version registered. (snowflakedb/drivers#2964)
