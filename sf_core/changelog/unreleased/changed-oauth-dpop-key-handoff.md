---
category: internal
---

- Changed the OAuth DPoP flow to hand its key to the Snowflake login as a key object rather than JWK JSON, which is now produced only when the key is written to the token cache. (snowflakedb/drivers#3455)
