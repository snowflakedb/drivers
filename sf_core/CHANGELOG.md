# Changelog

## Upcoming Release

Bug fixes:

- Fixed an issue where cached OAuth tokens could be incorrectly shared across different Snowflake accounts or roles that used the same identity provider, and where tokens stored by one driver could not be read by another. The token cache key now includes the IdP URL, Snowflake account URL, username, role, and token type, all normalized and hashed uniformly across drivers (`SnowflakeTokenCache.v2.<sha256>`). Existing v1 cache entries are orphaned; the driver re-authenticates transparently on the next connection and writes a v2 entry. (snowflakedb/universal-driver#TBD)

Test improvements:

- `SnowflakeTestClient` now automatically releases statement handles via Drop, eliminating manual `release_statement()` calls in tests and preventing resource leaks when tests panic.
