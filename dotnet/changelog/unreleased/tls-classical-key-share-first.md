---
category: change
behavior_differences: [BD#tls-classical-key-share-first]
---
- Changed the TLS handshake to send a classical key share first so the ClientHello fits in one packet, with the new `tls_prefer_post_quantum` connection parameter restoring the post-quantum-first order. (snowflakedb/drivers#3366)
