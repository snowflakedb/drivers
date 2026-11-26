---
category: change
---

- Changed connecting to succeed when a method of a `customCredentialManager` throws, rejects or does not settle within 30 seconds. The failure is logged as a warning, and a failed `read` is treated as a cache miss. (snowflakedb/drivers#3626)
