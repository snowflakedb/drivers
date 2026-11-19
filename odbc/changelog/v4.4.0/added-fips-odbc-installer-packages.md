---
category: feature
---

- Added FIPS ODBC installer packages (`-fips-` in the filename) as a separate FIPS build for Linux x64, Linux ARM64, macOS, and Windows x64. (snowflakedb/drivers#3528)
  - A FIPS build is not FIPS 140-3 validated; Snowflake does not claim that this driver is FIPS compliant.
  - The FIPS build links AWS-LC FIPS 3.6.0, a bugfix of AWS-LC FIPS 3.1.0. CMVP certificates cover 3.1.0; they do not cover this module. See [FIPS 140-3 considerations for drivers built on the Universal Core](https://docs.snowflake.com/en/developer-guide/universal-core/universal-core-fips).
  - Windows 32-bit and Windows ARM64 FIPS packages are not included.
