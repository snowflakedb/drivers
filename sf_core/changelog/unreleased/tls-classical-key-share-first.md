---
category: change
---
- Changed the TLS handshake to send a classical key share first so the ClientHello fits in one packet, with the new `tls_prefer_post_quantum` connection parameter restoring the post-quantum-first order. (snowflakedb/drivers#3366)
- Changed the connection diagnostic's TLS check to honor `min_tls_version` and `max_tls_version` when `tls_skip_verify` is set. (snowflakedb/drivers#3366)
