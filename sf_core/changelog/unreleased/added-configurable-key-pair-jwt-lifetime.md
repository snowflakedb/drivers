---
category: feature
---

- Added a `jwt_timeout` connection parameter, exposed in Node.js as `jwtTimeout`, to configure the lifetime in seconds of each key-pair authentication JWT, including tokens re-signed on login retry. The default remains 120 seconds; `0` selects the default and negative or non-integer values are rejected (snowflakedb/drivers#3141).
