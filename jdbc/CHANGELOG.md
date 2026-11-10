# Changelog

## Upcoming Release

In-progress notes are files under `changelog/unreleased/`.

## 5.0.0-RC2 (November 10, 2026)

New features:

- Added the JDBC properties `HTTP_CLIENT_CONNECTION_TIMEOUT`, `HTTP_CLIENT_SOCKET_TIMEOUT`, and `networkTimeout`, in milliseconds, as whole-second aliases of the connect timeout, the per-attempt timeout, and the non-query request timeout. (snowflakedb/drivers#2936)
- Added the `platformDetectionTimeoutMs` and `disablePlatformDetection` connection properties and the matching `net.snowflake.jdbc.platformDetectionTimeoutMs` and `net.snowflake.jdbc.disablePlatformDetection` JVM system properties, which set the per-probe platform-detection timeout in milliseconds and turn platform detection off. (snowflakedb/drivers#3034)

Changes:

- Changed `SNOWFLAKE_DISABLE_PLATFORM_DETECTION` to disable platform detection only for the literal value `true`, so `1` no longer disables it. (snowflakedb/drivers#3034)
- Improved driver performance by building the native library optimized for speed, at the cost of a larger package size. (snowflakedb/drivers#3175)

Bug fixes:

- Fixed result set fetches hanging indefinitely when a chunk download stalls; downloads now time out after 30 seconds without data and are retried, configurable with `chunkDownloadReadTimeout`. (snowflakedb/drivers#3202)
- Fixed `Statement.executeLargeUpdate`, `Statement.getLargeUpdateCount()`, and `PreparedStatement.executeLargeUpdate()` throwing `UnsupportedOperationException`: they now return the affected-row count as a long, and `Statement.executeLargeUpdate` with generated-keys flags, column indexes, or column names runs only with `NO_GENERATED_KEYS` and throws `SQLFeatureNotSupportedException` for the rest, as in JDBC 4.x. (snowflakedb/drivers#3225)
- Fixed `Statement.getGeneratedKeys()` throwing `SQLFeatureNotSupportedException`. It returns an empty result set whose getters return type defaults (`getLong` is 0, `getString` is `""`), as in JDBC 4.x. `Statement.execute` and `Statement.executeUpdate` with `Statement.RETURN_GENERATED_KEYS`, column indexes, or column names throw `SQLFeatureNotSupportedException`, as in JDBC 4.x. (snowflakedb/drivers#3167)
- Fixed `Connection.prepareStatement` with `Statement.RETURN_GENERATED_KEYS`, column indexes, or column names reporting SQLState `0A000` and vendor code `200035`, which made connection pools such as HikariCP evict the connection. It now reports no SQLState and vendor code 0, as in JDBC 4.x. The same SQLState change applies to `setTypeMap`, `setHoldability`, savepoints, `createBlob`, `createNClob`, `createSQLXML`, and `createStruct`. Transaction isolation and unsupported ResultSet type, concurrency, or holdability still report `0A000` / `200035`, as in JDBC 4.x. (snowflakedb/drivers#3167)
- Fixed JDBC fetch and bind loops recording an api-usage telemetry event on every `ResultSetMetaData` accessor, `ResultSet.getClob`, VECTOR `getArray`, and `PreparedStatement` setter call. (snowflakedb/drivers#3234)
