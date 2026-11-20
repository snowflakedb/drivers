---
category: internal
---

- Changed `crypto::dpop` and `DPoPKey` back to crate-private, since `DPoPSessionKey` is now the only DPoP type the connection's public fields expose. (snowflakedb/drivers#3474)
