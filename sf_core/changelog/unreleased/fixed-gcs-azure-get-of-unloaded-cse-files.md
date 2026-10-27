---
category: bugfix
---

- Fixed GET downloads from GCS- and Azure-backed stages returning undecrypted, unreadable data for files written by a server-side unload (`COPY INTO @stage`) onto a client-side-encrypted stage, such as the Parquet files Snowpark `DataFrame.to_polars()` reads. (snowflakedb/drivers#2928)
