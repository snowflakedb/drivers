---
category: internal
---

- Changed `client_crl_stats` from a shared REST client to be reported through the session of the latest telemetry-enabled connection that got the client, instead of the connection whose handshake triggered the CRL download. (snowflakedb/drivers#3644)
