# Changelog

## Upcoming Release

New features:

- Added server-side query cancel: capture the in-flight `requestId`/`sqlText` on the statement at submit time and add a `statement_cancel` driver API (plus `StatementCancel` RPC) that aborts the running query via `POST /queries/v1/abort-request`, so a cross-thread `SQLCancel` can stop a query on the server. (snowflakedb/universal-driver#628)

Bug fixes:

- Fixed an issue where cached OAuth tokens could be incorrectly shared across different Snowflake accounts or roles that used the same identity provider, and where tokens stored by one driver could not be read by another. The token cache key is now a versioned, uniformly hashed value (`SnowflakeTokenCache.v2.<token_type>.<sha256>`) computed identically across drivers. OAuth entries are keyed by IdP URL, Snowflake account URL, username, and role; MFA and ID-token entries are keyed only by Snowflake account URL and username (their flows carry no IdP or role). Existing v1 cache entries are orphaned; the driver re-authenticates transparently on the next connection and writes a v2 entry. (snowflakedb/universal-driver#735)
- Fixed `ConnectionAbortQuery` silently collapsing genuine errors (invalid connection handle, transport failures) into a declined-abort outcome; these now surface as proper errors instead. The response also now reports a typed `AbortQueryOutcome` (`ABORTED` / `NOT_RUNNING`) instead of a bare `success` bool. (snowflakedb/universal-driver#TBD)

Internal improvements:

- Replaced the three rarely-varied trailing parameters of `snowflake_query` and `snowflake_query_with_client` (retry policy, execution mode, request id) with a single `QueryOptions` struct that defaults to the common case (default retry policy, blocking mode, freshly-minted requestId), so most callers pass `QueryOptions::default()`.

Test improvements:

- `SnowflakeTestClient` now automatically releases statement handles and result set handles via Drop, eliminating manual `release_statement()` and `result_set_release()` calls in tests and preventing resource leaks when tests panic.
