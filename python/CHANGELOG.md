# Changelog

## Upcoming Release

New features:

- Changed `execute_string` and `execute_stream` to keep Snowflake Scripting blocks, such as `CREATE PROCEDURE ... AS BEGIN ... END;` and anonymous `DECLARE ... BEGIN ... END;` blocks, together as one statement instead of splitting them on each inner semicolon.

## 5.0.0 (October 15, 2026)

Breaking changes:

- Changed an empty `connections.toml` so connections defined in `config.toml` are kept.
- Changed `ConfigManager` with no `file_path` so reading an option raises `MissingConfigOptionError` instead of `ConfigManagerError`.
- Changed a bare `connect()` so a detected application name — from the `SF_PARTNER` environment variable, or from an imported Streamlit, Jupyter, or `snowbooks` module — takes precedence over `application` in the default connection profile.
- Changed `client_session_keep_alive` and `client_session_keep_alive_heartbeat_frequency` to read-only properties; assigning them raises `AttributeError`.
- Removed `snowflake.connector.config_manager.CONFIG_PARSER`; use `CONFIG_MANAGER`.
- Removed `ConfigManager.add_subparser()` and `ConfigManager._sub_parsers`; use `add_submanager()` and `_sub_managers`.
- Removed the public `Connection.connect()` method; the session is opened when the connection is constructed.
- Removed the `region` connection parameter and `Connection.region`; pass a fully qualified `account` or set `host` instead.
- Removed the `backoff_policy` callable; retry delays are set with `retry_backoff_base_ms`, `retry_backoff_cap_ms`, `retry_backoff_factor`, and `retry_backoff_jitter`, the default initial delay is 250 ms (was 1 s), and the default jitter is decorrelated (was full).
- Removed `Connection.disable_request_pooling`; passing `disable_request_pooling` to `connect()` warns and has no effect.
- Removed the `converter_class` connection parameter and `DefaultConverterClass`; passing `converter_class` raises `ProgrammingError`.
- Changed OAuth login failures for a bad client secret to include the identity provider's error name, such as `invalid_client`, in the exception message.
- Changed session-token login so `master_token` is required and invalid tokens fail in `connect()` instead of on a later request.
- Changed OAuth token and authorization endpoint URL validation to require HTTPS; `http://` is allowed only for loopback addresses.
- Changed external-browser login so a mid-session master-token expiry raises `ReauthenticationRequest` instead of opening the browser again and retrying the query.
- Changed EXTERNALBROWSER login so `disable_console_login` warns and has no effect: the parameter defaults to false, and the driver always requests the SSO URL from Snowflake instead of opening `/console/login`. (snowflakedb/drivers#2723)
- Removed the `password_callback` connection parameter; passing it raises `ProgrammingError`, and an expired password must be changed outside the driver.
- Changed cursor session parameters so a change on one cursor is visible to other cursors on the same connection without those cursors running a query.
- Changed methods on a closed cursor to raise `InterfaceError`; `fetchone`, `fetchmany`, `fetchall`, and `reset` still read rows already buffered on an open cursor after `connection.close()`.
- Changed `fetchone`, `fetchmany`, `fetchall`, and cursor iteration before any query to raise `ProgrammingError` instead of `TypeError`.
- Changed `cursor.execute` so passing `file_stream` on a statement that is not a PUT raises `ProgrammingError`.
- Changed `fetch_pandas_all`, `fetch_pandas_batches`, and the `fetch_arrow_*` methods so a JSON result, such as `SHOW` or `DESCRIBE`, returns a DataFrame or Arrow table instead of raising `NotSupportedError`.
- Changed `executemany()` on an `INSERT` without a `VALUES` clause to raise `ProgrammingError` instead of `InterfaceError`.
- Removed the `create_temp_table` argument from `write_pandas`; pass `table_type='temp'` instead.
- Removed `ResultBatch.session_manager` and `ResultBatch.http_config`.
- Changed result-chunk fetching to always use a thread pool: `client_fetch_use_mp` warns and no longer selects a process pool, and `client_fetch_threads` is a deprecated alias of `client_prefetch_threads`.
- Changed PUT so symlinks in a source path are resolved before upload: a symlinked file is stored under the target file's name, and a relative path that walks through a symlink and `..` can select a different file.
- Removed `iobound_tpe_limit`; passing it to `connect()` warns and has no effect, and `Connection.iobound_tpe_limit` does not exist.
- Changed proxy selection so an explicit `proxy_host`/`proxy_port` or `proxy` URL wins when `HTTP_PROXY`, `HTTPS_PROXY`, or `NO_PROXY` is also set.
- Changed CRL caching so `enable_crl_cache=False` warns and no longer disables it; set both `enable_crl_memory_cache` and `enable_crl_file_cache` to `False` to turn caching off.
- Changed FIPS support so it is available only when the Python connector is installed from source with `SNOWFLAKE_FIPS=1`; see the installation instructions for details. (snowflakedb/drivers#2489)
- Removed support of OCSP certificate validation in favor of CRL validation.
- Removed YAML easy-logging configuration; troubleshooting logs are enabled with the `SNOWFLAKE_TROUBLESHOOTING_ENABLED` environment variable.
- Removed the `snowflake.connector.options` module; importing it raises `ModuleNotFoundError`.
- Removed `snowflake.connector.dbapi` and the authenticator classes previously exported from `snowflake.connector.auth`.
- Removed file-transfer and type-helper exports from `snowflake.connector.constants`, including `FileTransferType`, `ResultStatus`, cloud filesystem labels, part-size constants, the `get_*_types` and `is_*_type_name` helpers, and the Arrow type helpers; `QueryStatus` remains.
- Removed `DownloadMetrics`, `RemoteChunkInfo`, and the chunk-download constants `DOWNLOAD_TIMEOUT`, `MAX_DOWNLOAD_RETRY`, `SSE_C_AES`, `SSE_C_ALGORITHM`, and `SSE_C_KEY` from `snowflake.connector.result_batch`.

New features:

- Added an asyncio-compatible API under `snowflake.connector.aio`, including nonblocking connection, query, fetch, transaction, Arrow, pandas, and async context-manager operations. (snowflakedb/drivers#216)
- Added the `use_core_arrow` connection parameter to opt into native Rust conversion for row, Arrow, and pandas result fetching in the synchronous and asyncio APIs. (snowflakedb/drivers#2523)

Bug fixes:

- Fixed PUT `SOURCE_COMPRESSION=AUTO_DETECT` so a `.deflate` file is recognized as `DEFLATE`.
- Fixed GET so a failed download removes the temporary `.part` file.
- Fixed Azure client-side encryption metadata so `EncryptionAgent.EncryptionAlgorithm` is `AES_CBC_256`.
- Fixed the GET result `file` column so a stage path with more than one directory level reports only the file name.
- Fixed `Connection.is_valid()` so a live `PAT_WITH_EXTERNAL_SESSION` connection returns true.

Changes:

- Changed gzip-compressed PUT uploads to omit the original filename from the gzip header.
- Added support for PUT `SOURCE_COMPRESSION=BROTLI`, including auto-detection of `.br` files.
- Changed the private-key password parameter to `private_key_password`; `private_key_file_pwd` is still accepted and warns.
- Changed the MFA token-caching parameter to `client_store_temporary_credential`; `client_request_mfa_token` is still accepted and warns.
- Changed `connection.rest.request()` to raise `ValueError` for an HTTP method other than `get` or `post`.
- Changed `JSONResultBatch` and `ArrowResultBatch` so `to_arrow()`, `to_pandas()`, and table iteration work on every result batch.
- Changed S3 uploads and downloads in China regions (`cn-*`) to use the AWS SDK's default endpoint when no regional URL and no custom endpoint are set.
- Changed `oauth_enable_refresh_tokens` so it warns and has no effect; a refresh token is used when the identity provider returns one, and `client_store_temporary_credential` controls whether that token is cached on disk.
- Changed `oauth_credentials_in_body=True` so the client id and secret are sent only in the token-request body.
- Changed the connection parameter for S3 regional URLs to `use_s3_regional_url`; `enable_stage_s3_privatelink_for_us_east_1` is still accepted and warns.
- Changed Azure Blob PUT to omit the `Content-Encoding` and `x-ms-blob-content-encoding` headers.
- Changed concurrent interactive logins for the same user and host to share one prompt when `disable_parallel_user_prompt` and `client_store_temporary_credential` are left at their defaults.
- Changed `executemany` to follow the server's array-binding flag instead of detecting `INSERT` in the SQL text; a statement the server does not mark as array-bindable still runs one row at a time.
- Changed Azure PUT and GET so any HTTP 403 refreshes the SAS token and retries.
- Changed a failed Azure SAS refresh so it raises a terminal error instead of retrying without a limit; a rejected token is logged as a warning and a failed refresh as an error.
- Removed the `snowflake.connector.telemetry_oob` module.
- Changed client-side encryption metadata so `KeyWrappingMetadata.EncryptionLibrary` is `Rust(OpenSSL)`.
- Added `parameter` and `validation_code` on errors from connection-configuration validation.
- Changed `fetch_pandas_all` and `fetch_pandas_batches` so extra keyword arguments are passed to Arrow's `to_pandas`; arguments that only `pandas.concat` accepts, such as `sort`, `keys`, and `copy`, raise `TypeError`.
- Changed `login_timeout`, `network_timeout`, and `socket_timeout` so `login_timeout` returns the login budget (120 seconds when unset, previously `None`), `network_timeout` returns `request_timeout` (120 seconds when unset, previously `None`), and `socket_timeout` returns `retry_timeout`; passing `network_timeout` warns and also sets `query_timeout`, passing `socket_timeout` warns and also sets `connect_timeout`, and `retry_timeout` is a total time limit for the attempt rather than a per-read inactivity timeout.
- Changed `cursor._request_id` to warn on use; it returns the same `uuid.UUID` as `cursor.request_id`.
- Changed non-string session parameter values so they are sent as JSON strings.
- Changed binding of an unsupported Python type to raise `ProgrammingError` with errno 255001.
- Added `display_size` for text columns, set to the column's character length.
- Changed `put_get_max_attempts` so any positive integer is used, with no upper limit.
- Changed the default of `client_store_temporary_credential` to true on every platform; an explicit false now disables token caching on macOS and Windows as well.
- Changed the unused connection arguments `dsn`, `session_id`, `support_negative_year`, `json_result_force_utf8_decoding`, `inject_client_pause`, `probe_connection`, `debug_arrow_chunk`, `snowflake_server_dop_cap_for_file_transfer`, and `reraise_error_in_file_transfer_work_function` so they warn and have no effect; `connection.session_id` after login still returns the server-assigned id, and `log_imported_packages` and `log_imported_packages_in_telemetry` are ignored without a warning.
- Changed legacy HTTP error classes so they remain importable from `snowflake.connector.errors`, warn on first use, and are no longer raised: `HttpError`, `InternalServerError`, `ServiceUnavailableError`, `GatewayTimeoutError`, `ForbiddenError`, `RequestTimeoutError`, `BadRequest`, `BadGatewayError`, `MethodNotAllowed`, `TooManyRequests`, `OtherHTTPRetryableError`, `RevocationCheckError`, `RefreshTokenError`, `BindUploadError`, `RequestExceedMaxRetryError`, `TokenExpiredError`, and `PresignedUrlExpiredError`.
