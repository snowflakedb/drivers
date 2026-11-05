---
category: bugfix
---

- Fixed Windows wide-string calls being mis-decoded, which failed or crashed the connection, when `sf.odbc.ini` sets `DriverManagerEncoding=UTF-32`; the driver now uses UTF-16 and logs a warning. (snowflakedb/drivers#2961)
