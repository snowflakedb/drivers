---
category: bugfix
---

- Fixed an issue where `send_http` accepted relative paths that resolve to a different host than the connection's server; such paths are now rejected. (snowflakedb/drivers#3501)
