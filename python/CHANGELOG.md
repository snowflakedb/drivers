# Changelog

## Upcoming Release

Changes:

- Added proxy connection parameters exposed through auto-generated `connection_config.py`. Explicit `proxy_host`/`proxy_port` now take precedence over `HTTP_PROXY`/`HTTPS_PROXY` env vars. See `BehaviorDifferences.yaml` entry 31. (snowflakedb/universal-driver#29)
