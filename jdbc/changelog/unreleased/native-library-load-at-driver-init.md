---
category: change
---
- Changed the driver to load its native library when `SnowflakeDriver` is initialized, so the first `connect()` does not pay for extracting and loading it. (snowflakedb/drivers#3479)
