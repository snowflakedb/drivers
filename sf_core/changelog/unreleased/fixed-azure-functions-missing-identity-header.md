---
category: bugfix
---

- Fixed Azure Functions workload identity so a missing `IDENTITY_HEADER` returns an error instead of requesting a token from the instance metadata service. (snowflakedb/drivers#2966)
