# Changelog

## Upcoming Release

Changes:

- Reject `SQL_C_BIT` bound to any `SQL_INTERVAL_*` parameter with SQLSTATE 07006. (snowflakedb/universal-driver#80)
- Invalid `SQL_C_CHAR` / `SQL_C_WCHAR` literals bound to `DATE` / `TIME` / `TIMESTAMP` now return SQLSTATE 22018 instead of 07006. (snowflakedb/universal-driver#81)
- Cache `TIMESTAMP_TZ_OUTPUT_FORMAT` per connection and refresh it only after `ALTER SESSION`, removing a per-execute RPC roundtrip. A transient failure to read the parameter no longer reverts `TIMESTAMP_TZ` → CHAR/WCHAR rendering to bare UTC. (snowflakedb/universal-driver#74)
- Add iODBC support with UTF-32 encoding (snowflakedb/universal-driver#16)
- Added proxy DSN keys end-to-end (`PROXY_HOST`, `PROXY_PORT`, `PROXY_USER`, `PROXY_PASSWORD`, `NO_PROXY` and legacy aliases `PROXY`, `NOPROXY`, `PROXYWITHENV`, `ALLOWEMPTYPROXY`). (snowflakedb/universal-driver#29)
