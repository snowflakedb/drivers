---
category: internal
---

- Changed JDBC result sets to decode their first Arrow batch while the result set is built, so that cost is part of execute rather than fetch. (snowflakedb/drivers#3363)
