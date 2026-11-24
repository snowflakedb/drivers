---
category: bugfix
---
- Fixed the 4.x `CERT_REVOCATION_CHECK_MODE` and `ALLOW_CERTIFICATES_WITHOUT_CRL_URL` connection properties being ignored, and accepted the CRL cache settings `ENABLE_CRL_IN_MEMORY_CACHING`, `ENABLE_CRL_DISK_CACHING`, `CRL_CACHE_VALIDITY_TIME`, and `CRL_RESPONSE_CACHE_DIR` as connection properties. (snowflakedb/drivers#3463)
