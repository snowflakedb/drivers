---
category: internal
---

- Moved the DPoP key, thumbprint and proof code into `crypto::dpop`, and the random-token helper into `crypto::random`, so authenticators other than OAuth can reuse them. (snowflakedb/drivers#3232)
