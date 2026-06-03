# Changelog

- [NEXT RELEASE]
    - Invalid `SQL_C_CHAR` / `SQL_C_WCHAR` literals bound to `DATE` / `TIME` / `TIMESTAMP` now return SQLSTATE 22018 instead of 07006.
    - Cache `TIMESTAMP_TZ_OUTPUT_FORMAT` per connection and refresh it only after `ALTER SESSION`, removing a per-execute RPC roundtrip. A transient failure to read the parameter no longer reverts `TIMESTAMP_TZ` → CHAR/WCHAR rendering to bare UTC.
    - Add iODBC support with UTF-32 encoding
