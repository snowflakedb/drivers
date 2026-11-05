---
category: feature
---

- Added the `platformDetectionTimeoutMs` and `disablePlatformDetection` connection properties and the matching `net.snowflake.jdbc.platformDetectionTimeoutMs` and `net.snowflake.jdbc.disablePlatformDetection` JVM system properties, which set the per-probe platform-detection timeout in milliseconds and turn platform detection off. (snowflakedb/drivers#3034)
