---
category: feature
---

- Added the JDBC properties `HTTP_CLIENT_CONNECTION_TIMEOUT`, `HTTP_CLIENT_SOCKET_TIMEOUT`, and `networkTimeout`, in milliseconds, as whole-second aliases of the connect timeout, the per-attempt timeout, and the non-query request timeout. (snowflakedb/drivers#2936)
