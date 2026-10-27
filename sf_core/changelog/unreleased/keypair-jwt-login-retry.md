---
category: bugfix
---

- Fixed key-pair login retries sending an expired JWT by signing a new token for each login attempt. (snowflakedb/drivers#2946)
