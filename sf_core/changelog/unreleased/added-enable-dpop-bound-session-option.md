---
category: feature
---

- Added the `enable_dpop_bound_session` connection option, which, on accounts where the server supports DPoP-bound sessions, binds key-pair and workload identity sessions to a key generated at login so that a copied session token cannot be used without that key. (snowflakedb/drivers#3357)
