---
category: internal
---

- Added a DPoP proof, with `ath` over the presented token, to every request of a DPoP-bound session, including retries, renewal, heartbeat, logout, telemetry and the `send_http` passthrough; the connection keeps the key its login generated. (snowflakedb/drivers#3356)
