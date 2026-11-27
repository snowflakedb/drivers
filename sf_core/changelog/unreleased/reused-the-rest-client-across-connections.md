---
category: change
---

- Changed a new connection to reuse the driver's most recent REST HTTP client, and its idle sockets, when the TLS, proxy, and connect-timeout settings are unchanged, which makes reconnecting within 30 seconds faster; setting `SNOWFLAKE_DISABLE_HTTP_CLIENT_CACHE` to `true` or `1` builds a new client for every connection. A macOS or Windows system proxy change applies once a connection's settings differ from the cached client's or the process restarts. (snowflakedb/drivers#3644)
