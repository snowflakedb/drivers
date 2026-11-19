---
category: change
---

- Improved fetch latency for small results by reading result batches from the core directly instead of through an Arrow C stream. (snowflakedb/drivers#3271)
