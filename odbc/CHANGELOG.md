# Changelog

- [NEXT RELEASE]
    - Cache `TIMESTAMP_TZ_OUTPUT_FORMAT` per connection and refresh it only after `ALTER SESSION`, removing a per-execute RPC roundtrip. A transient failure to read the parameter no longer reverts `TIMESTAMP_TZ` → CHAR/WCHAR rendering to bare UTC.
    - Add iODBC support with UTF-32 encoding
