# Changelog

## Upcoming Release

New features:

- Added a `secondary_roles` connection parameter that lets a client control secondary-role activation at login (e.g. `ALL` or `NONE`) without relying on the user's `DEFAULT_SECONDARY_ROLES` setting, ported from legacy snowflake-connector-python. Also restores parity with legacy ODBC's `SecondaryRoles` connection attribute, and is newly available (with no legacy equivalent) for JDBC. (snowflakedb/drivers#954)
- Added server-side query cancel: capture the in-flight `requestId`/`sqlText` on the statement at submit time and add a `statement_cancel` driver API (plus `StatementCancel` RPC) that aborts the running query via `POST /queries/v1/abort-request`, so a cross-thread `SQLCancel` can stop a query on the server. (snowflakedb/drivers#628)
- Added a shared operation-cancellation registry and `RustTransport::handle_message_cancellable`, letting a bridge cancel an in-flight RPC by handle from any thread and surface cancellation as `DriverException` with `STATUS_CODE_CANCELLED`; the async C API now cancels through this registry. (snowflakedb/drivers#TBD)
- Added async-first RPCs: an RPC marked `async_first` in the proto generates a `Future`-returning client method, and JDBC's `ConnectionInit` now uses it via new `nativeSubmitMessage`/`nativeAwaitMessage`/`nativeCancel` JNI entries. (snowflakedb/drivers#TBD)

Bug fixes:

- Restricted the WORKLOAD_IDENTITY authenticator to recognized Snowflake hosts (*.snowflakecomputing.com/.cn/.mil), normalizing the host before a suffix-anchored match. The SNOWFLAKE_WIF_ALLOWED_HOST_SUFFIXES environment variable additively extends the recognized-host list.
- Fixed an issue where cached OAuth tokens could be incorrectly shared across different Snowflake accounts or roles that used the same identity provider, and where tokens stored by one driver could not be read by another. The token cache key is now a versioned, uniformly hashed value (`SnowflakeTokenCache.v2.<token_type>.<sha256>`) computed identically across drivers. OAuth entries are keyed by IdP URL, Snowflake account URL, username, and role; MFA and ID-token entries are keyed only by Snowflake account URL and username (their flows carry no IdP or role). Existing v1 cache entries are orphaned; the driver re-authenticates transparently on the next connection and writes a v2 entry. (snowflakedb/drivers#735)
- Fixed `ConnectionAbortQuery` silently collapsing genuine errors (invalid connection handle, transport failures) into a declined-abort outcome; these now surface as proper errors instead. The response also now reports a typed `AbortQueryOutcome` (`ABORTED` / `NOT_RUNNING`) instead of a bare `success` bool. (snowflakedb/drivers#TBD)
- Fixed string `private_key` to accept plaintext PEM (as already documented), not only base64-encoded material. (snowflakedb/drivers#953)

Internal improvements:

- Cancellation is now observed inside the operation rather than raced at the protobuf transport, and the proto is the single source of truth for which operations are cancellable: an RPC marked `async_first` receives an `OperationCtx` through the generated dispatch, and its `DatabaseDriverV1` implementation reports cancellation as a typed `ApiError::Cancelled` (mapped to `STATUS_CODE_CANCELLED`). Unmarked RPCs keep the previous transport-level behaviour, so no wrapper changes semantics. (snowflakedb/drivers#TBD)
- Node's `Connection` now owns a cancellation context for `connect()` and exposes `cancelConnect()` to trigger it. (snowflakedb/drivers#TBD)
- Replaced the three rarely-varied trailing parameters of `snowflake_query` and `snowflake_query_with_client` (retry policy, execution mode, request id) with a single `QueryOptions` struct that defaults to the common case (default retry policy, blocking mode, freshly-minted requestId), so most callers pass `QueryOptions::default()`.
- Raised the default multipart block size for PUT uploads to internal Azure stages from 4 MiB to 8 MiB, matching the S3/GCS default and improving throughput for typical file sizes.
- Response-body reads for the OAuth token exchange and GCP metadata server now stream against a fixed size limit using a running byte count (shared `read_body_capped` helper) rather than relying on the advertised `Content-Length`. (snowflakedb/drivers#1053)
- Improved on-disk CRL cache file handling: cache files are written with owner-only permissions and each cache entry is read through a single file handle. (snowflakedb/drivers#1056)

Test improvements:

- `SnowflakeTestClient` now automatically releases statement handles and result set handles via Drop, eliminating manual `release_statement()` and `result_set_release()` calls in tests and preventing resource leaks when tests panic.
