---
category: bugfix
---

- Fixed an issue where HTTP requests retried up to 5 times after a TLS handshake or CRL certificate revocation failure instead of failing immediately. (snowflakedb/drivers#3499)
