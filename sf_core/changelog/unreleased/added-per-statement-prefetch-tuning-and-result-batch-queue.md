---
category: feature
---

- Added client-only statement-level `CLIENT_PREFETCH_THREADS` overrides and an optional connection-, session-, or statement-level `CLIENT_RESULT_QUEUE_SIZE` limiting decoded batches handed to a reader but not yet read. A chunk with more batches is delivered alone; up to `CLIENT_PREFETCH_THREADS` further chunks may already be downloaded and decoded, bounded by `CLIENT_MEMORY_LIMIT`. Explicit login and post-connect session-parameter maps validate and retain client-only settings locally. Async prefetch snapshots include session parameters returned by submission. (snowflakedb/drivers#1379)
