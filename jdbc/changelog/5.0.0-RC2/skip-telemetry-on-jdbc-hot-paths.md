---
category: bugfix
---

- Fixed JDBC fetch and bind loops recording an api-usage telemetry event on every `ResultSetMetaData` accessor, `ResultSet.getClob`, VECTOR `getArray`, and `PreparedStatement` setter call. (snowflakedb/drivers#3234)
