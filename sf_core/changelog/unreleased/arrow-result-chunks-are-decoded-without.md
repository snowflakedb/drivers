---
category: change
---

- Reduced peak memory while fetching Arrow results by decoding chunks in place instead of copying each batch out of the download buffer. (snowflakedb/drivers#3058)
