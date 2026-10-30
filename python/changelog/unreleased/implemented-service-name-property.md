---
category: bugfix
---

- Implemented the `service_name` connection property, reading the live `SERVICE_NAME` session parameter from the core driver so the value tracks cluster reassignments made by query responses. (snowflakedb/drivers#3068)
