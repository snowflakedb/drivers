---
category: change
---

- Improved multi-file PUT and GET throughput by starting the next file as soon as any transfer finishes, instead of waiting for the oldest one. (snowflakedb/drivers#3205)
