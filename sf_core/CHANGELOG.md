# Changelog

## Upcoming Release

New features:

- Added server-side query cancel: capture the in-flight `requestId`/`sqlText` on the statement at submit time and add a `statement_cancel` driver API (plus `StatementCancel` RPC) that aborts the running query via `POST /queries/v1/abort-request`, so a cross-thread `SQLCancel` can stop a query on the server. (snowflakedb/universal-driver#628)

Bug fixes:

- Fixed an issue where cached OAuth tokens could be incorrectly shared across different Snowflake accounts or roles that used the same identity provider, and where tokens stored by one driver could not be read by another. The token cache key now includes the IdP URL, Snowflake account URL, username, role, and token type, all normalized and hashed uniformly across drivers (`SnowflakeTokenCache.v2.<sha256>`). Existing v1 cache entries are orphaned; the driver re-authenticates transparently on the next connection and writes a v2 entry. (snowflakedb/universal-driver#TBD)
- Fixed `ConnectionAbortQuery` silently collapsing genuine errors (invalid connection handle, transport failures) into a declined-abort outcome; these now surface as proper errors instead. The response also now reports a typed `AbortQueryOutcome` (`ABORTED` / `NOT_RUNNING`) instead of a bare `success` bool. (snowflakedb/universal-driver#TBD)

Test improvements:

- `SnowflakeTestClient` now automatically releases statement handles and result set handles via Drop, eliminating manual `release_statement()` and `result_set_release()` calls in tests and preventing resource leaks when tests panic.
