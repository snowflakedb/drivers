---
category: change
---

- Arrow result chunks are decoded without copying: the downloaded body is handed to the IPC decoder as-is and record batches reference it directly, and gzip-encoded chunks are inflated into a buffer sized from the server-reported uncompressed size. Peak memory while fetching large results drops by roughly the size of the chunks in flight (for example about 250 MiB on a 6 million row TPC-H `LINEITEM` fetch).
