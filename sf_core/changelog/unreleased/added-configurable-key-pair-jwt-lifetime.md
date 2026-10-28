---
category: feature
---

- Added a `jwt_timeout` connection parameter to configure the lifetime in seconds of each key-pair authentication JWT, including tokens re-signed on login retry. The default remains 120 seconds; values must be positive integers.
