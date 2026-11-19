---
category: change
---

- Changed the macOS installer to write `~/Library/ODBC/odbcinst.ini` and `~/Library/ODBC/odbc.ini` as the installing user, and to print a warning instead of changing ownership when that directory is not writable by that user. (snowflakedb/drivers#3081)
