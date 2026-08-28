# Changelog

## Upcoming Release

New features:

- Added a `token_file_path` connection parameter that loads a PAT, legacy OAuth, or OIDC bearer token from a file (including from `connections.toml`), matching the legacy Python connector. If both `token` and `token_file_path` are set, the file contents are used. (snowflakedb/drivers#1445)
- Added a `secondary_roles` connection parameter that lets a client control secondary-role activation at login (e.g. `ALL` or `NONE`) without relying on the user's `DEFAULT_SECONDARY_ROLES` setting, ported from legacy snowflake-connector-python. Also restores parity with legacy ODBC's `SecondaryRoles` connection attribute, and is newly available (with no legacy equivalent) for JDBC. (snowflakedb/drivers#954)

- Added server-side query cancel: capture the in-flight `requestId`/`sqlText` on the statement at submit time and add a `statement_cancel` driver API (plus `StatementCancel` RPC) that aborts the running query via `POST /queries/v1/abort-request`, so a cross-thread `SQLCancel` can stop a query on the server. (snowflakedb/drivers#628)
- Added a shared operation-cancellation registry and `RustTransport::handle_message_cancellable`, letting a bridge cancel an in-flight RPC by handle from any thread and surface cancellation as `DriverException` with `ERROR_KIND_CANCELLED`; the async C API now cancels through this registry. (snowflakedb/drivers#TBD)
- Added async-first RPCs: an RPC marked `async_first` in the proto generates a `Future`-returning client method, and JDBC's `ConnectionInit` now uses it via new `nativeSubmitMessage`/`nativeAwaitMessage`/`nativeCancel` JNI entries. (snowflakedb/drivers#TBD)
- Added impersonation-chain support to the standalone `create_attestation` RPC, so callers can acquire a Workload Identity Federation attestation for an assumed AWS role, delegated GCP service account, or impersonated Azure service principal without an active connection. (snowflakedb/drivers#1027)
- `StatementExecuteQuery` is now `async_first`: cancelling a running query aborts it on the server via `POST /queries/v1/abort-request` instead of only dropping the in-flight request locally, so the query stops consuming credits. The abort is bounded and is awaited before cancellation is reported, so a returned cancellation implies the abort was issued. (snowflakedb/drivers#TBD)
- `StatementPrepare` is now `async_first` as well, so cancelling a prepare aborts its `describe_only` query on the server instead of only dropping the request locally — previously a cancelled prepare left the described query running. (snowflakedb/drivers#1463)

Bug fixes:

- Fixed cancelling a PUT abandoning the in-progress cloud upload instead of aborting it: an S3 multipart upload was left with its uploaded parts in place, which AWS bills until a lifecycle rule reaps them, and a GCS resumable session was left half-staged until Google expired it a week later. Both are now aborted when the transfer is cancelled, not only when it errors. (snowflakedb/drivers#TBD)
- Fixed cancelling a GET continuing to download the whole file in the background after the caller was told the operation was cancelled, and leaving a partial `.part` file beside the destination; the transfer is now stopped and the partial file removed. (snowflakedb/drivers#TBD)
- Fixed a client-side `QUERY_TIMEOUT` giving up locally without telling the server, leaving the query running and consuming credits; the timeout now also aborts the query. It still reports `QueryTimeout` rather than a cancellation, so the two remain distinguishable. (snowflakedb/drivers#TBD)
- Fixed a cancel arriving during a large bind-variable stage upload emitting an abort-request for a query that had never been submitted; the in-flight query identity is now published only once the query is about to be sent. (snowflakedb/drivers#TBD)
- Restricted the WORKLOAD_IDENTITY authenticator to recognized Snowflake hosts (*.snowflakecomputing.com/.cn/.mil), normalizing the host before a suffix-anchored match. The SNOWFLAKE_WIF_ALLOWED_HOST_SUFFIXES environment variable additively extends the recognized-host list.
- Fixed an issue where cached OAuth tokens could be incorrectly shared across different Snowflake accounts or roles that used the same identity provider, and where tokens stored by one driver could not be read by another. The token cache key is now a versioned, uniformly hashed value (`SnowflakeTokenCache.v2.<token_type>.<sha256>`) computed identically across drivers. OAuth entries are keyed by IdP URL, Snowflake account URL, username, and role; MFA and ID-token entries are keyed only by Snowflake account URL and username (their flows carry no IdP or role). Existing v1 cache entries are orphaned; the driver re-authenticates transparently on the next connection and writes a v2 entry. (snowflakedb/drivers#735)
- Fixed `ConnectionAbortQuery` silently collapsing genuine errors (invalid connection handle, transport failures) into a declined-abort outcome; these now surface as proper errors instead. The response also now reports a typed `AbortQueryOutcome` (`ABORTED` / `NOT_RUNNING`) instead of a bare `success` bool. (snowflakedb/drivers#TBD)
- Fixed string `private_key` to accept plaintext PEM (as already documented), not only base64-encoded material. (snowflakedb/drivers#953)
- Fixed Workload Identity Federation attestation failures reporting an internally inconsistent error type, which could have caused non-Python bindings to surface the wrong exception category. (snowflakedb/drivers#TBD)

Internal improvements:

- Cancellation is now observed inside the operation rather than raced at the protobuf transport, and the proto is the single source of truth for which operations are cancellable: an RPC marked `async_first` receives an `OperationCtx` through the generated dispatch, and its `DatabaseDriverV1` implementation reports cancellation as a typed `ApiError::Cancelled` (mapped to `ERROR_KIND_CANCELLED`). Unmarked RPCs keep the previous transport-level behaviour, so no wrapper changes semantics. (snowflakedb/drivers#TBD)
- `OperationCtx` can now carry cancellation cleanup: `arm_cleanup` registers work on a tracked task that survives the operation future being dropped (there is no async `Drop`, so a cancelled future cannot await its own cleanup), guarded by an RAII handle that suppresses it whenever the guarded work finishes on its own. This keeps the "race the token in exactly one place" invariant while still letting an inner layer clean up. (snowflakedb/drivers#TBD)
- Node's `Connection` now owns a cancellation context for `connect()` and exposes `cancelConnect()` to trigger it. (snowflakedb/drivers#TBD)
- Replaced the three rarely-varied trailing parameters of `snowflake_query` and `snowflake_query_with_client` (retry policy, execution mode, request id) with a single `QueryOptions` struct that defaults to the common case (default retry policy, blocking mode, freshly-minted requestId), so most callers pass `QueryOptions::default()`.
- Raised the default multipart block size for PUT uploads to internal Azure stages from 4 MiB to 8 MiB, matching the S3/GCS default and improving throughput for typical file sizes.
- Response-body reads for the OAuth token exchange and GCP metadata server now stream against a fixed size limit using a running byte count (shared `read_body_capped` helper) rather than relying on the advertised `Content-Length`. (snowflakedb/drivers#1053)
- Improved on-disk CRL cache file handling: cache files are written with owner-only permissions and each cache entry is read through a single file handle. (snowflakedb/drivers#1056)

Test improvements:

- `SnowflakeTestClient` now automatically releases statement handles and result set handles via Drop, eliminating manual `release_statement()` and `result_set_release()` calls in tests and preventing resource leaks when tests panic.
