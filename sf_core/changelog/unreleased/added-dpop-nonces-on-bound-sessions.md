---
category: internal
---

- Added DPoP nonce handling to DPoP-bound sessions: every proof carries the latest `DPoP-Nonce` Snowflake issued, starting from the login response, and a request answered with `use_dpop_nonce` is sent once more with the new nonce. (snowflakedb/drivers#3370)
