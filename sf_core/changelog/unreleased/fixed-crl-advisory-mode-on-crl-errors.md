---
category: bugfix
---

- Fixed `ADVISORY` certificate revocation checking (`crl_check_mode=ADVISORY`) so that CRL download, verification, and missing-distribution-point errors log a warning and allow the connection instead of failing with `CRL validation failed: chain revoked`. (snowflakedb/drivers#3566)
- Fixed `ADVISORY` certificate revocation checking so that when the server presents several candidate certificate chains, a revoked chain combined with a chain whose CRL check could not be completed allows the connection instead of failing. (snowflakedb/drivers#3566)
