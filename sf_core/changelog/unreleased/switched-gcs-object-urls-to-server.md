---
category: change
---

- Changed GCS PUT/GET to use the `endPoint` and `useVirtualUrl` the server returns in stage info when present, and declared the `GCS_ENDPOINT_IN_STAGE_LOCATIONINFO` login capability so the server can send them. (snowflakedb/drivers#3454)
