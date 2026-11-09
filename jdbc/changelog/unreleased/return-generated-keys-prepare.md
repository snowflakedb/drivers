---
category: bugfix
---

- Fixed `Statement.getGeneratedKeys()` throwing `SQLFeatureNotSupportedException`. It returns an empty result set whose getters return type defaults (`getLong` is 0, `getString` is `""`), as in JDBC 4.x. `Statement.execute` and `Statement.executeUpdate` with `Statement.RETURN_GENERATED_KEYS`, column indexes, or column names throw `SQLFeatureNotSupportedException`, as in JDBC 4.x. (snowflakedb/drivers#3167)
- Fixed `Connection.prepareStatement` with `Statement.RETURN_GENERATED_KEYS`, column indexes, or column names reporting SQLState `0A000` and vendor code `200035`, which made connection pools such as HikariCP evict the connection. It now reports no SQLState and vendor code 0, as in JDBC 4.x. The same SQLState change applies to `setTypeMap`, `setHoldability`, savepoints, `createBlob`, `createNClob`, `createSQLXML`, and `createStruct`. Transaction isolation and unsupported ResultSet type, concurrency, or holdability still report `0A000` / `200035`, as in JDBC 4.x. (snowflakedb/drivers#3167)
