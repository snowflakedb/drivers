---
category: bugfix
---

- Added the `X-Snowflake-Service` request header, echoing the `SERVICE_NAME` session parameter on queries so the proxy routes requests to the correct HTAP cluster. (snowflakedb/drivers#3068)
