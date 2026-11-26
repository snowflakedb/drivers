---
category: change
---
- Implemented `Statement.cancel()` and `Statement.getMaxFieldSize()`, and made `setMaxFieldSize`, `setPoolable(true)`, `closeOnCompletion` and `isCloseOnCompletion` throw `SQLFeatureNotSupportedException` as in snowflake-jdbc. (snowflakedb/drivers#3648)
