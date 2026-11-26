---
category: bugfix
---

- Fixed a possible crash when the application changes environment variables while the CRL background threads look up the default CRL cache directory; the directory is now chosen once, when the driver is initialized. (snowflakedb/drivers#3491)
