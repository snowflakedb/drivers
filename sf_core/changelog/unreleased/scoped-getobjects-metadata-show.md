---
category: change
---

- Scoped GetObjects metadata SHOW queries to each matching database or schema for wildcard filters (including underscores), and to exact tables for column metadata, avoiding broad scans of unrelated metadata even when many schemas match. (snowflakedb/drivers#3067)
