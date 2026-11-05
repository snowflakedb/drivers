# Changelog

## Upcoming Release

In-progress notes are files under `changelog/unreleased/`.

## v4.3.0

Breaking changes:

- Changed `SQLBindCol` and `SQLGetData` with `SQL_C_DEFAULT` on `INTERVAL` result columns to return a `SQL_INTERVAL_STRUCT` instead of the ANSI interval literal string, failing with `22015` when the leading field exceeds the interval leading precision (2 digits by default) and with `HY090` when a nonzero buffer length is smaller than the struct; binding `SQL_C_CHAR` still returns the string. (snowflakedb/drivers#3020)

New features:

- Added support for a leading `~` in `token_file_path`, so a path such as `~/.snowflake/pat.token` is read from the home directory when loading a PAT, OAuth, or OIDC bearer token. (snowflakedb/drivers#3049)

Changes:

- Changed query-result `SQL_DESC_TYPE_NAME` for FILE columns to report `FILE`. `SQLDescribeCol` stays `SQL_VARCHAR`, including when `ODBC_USE_CUSTOM_SQL_DATA_TYPES` is enabled. (snowflakedb/drivers#2833)
- Changed the Windows installer to reject a driver name that contains `#`. (snowflakedb/drivers#2963)

Bug fixes:

- Added the `X-Snowflake-Service` request header, echoing the `SERVICE_NAME` session parameter on queries so the proxy routes requests to the correct HTAP cluster. (snowflakedb/drivers#3068)
- Fixed catalog function SMALLINT and INTEGER result columns reporting NUMBER display size, octet length, and type names. `SQL_DESC_LOCAL_TYPE_NAME` on those columns now matches the 3.x driver (`SSMALLINT`, `USMALLINT`, `SINTEGER`), and unsigned `USER_DATA_TYPE` reports display size 5. Catalog wide-character columns report octet length as four bytes per character, and `SQLSpecialColumns` `COLUMN_NAME` reports character length 255. (snowflakedb/drivers#2760)
- Fixed the driver resubmitting a running query when the session token expired during a long-running server-side poll; the driver now refreshes the token and retries only the result poll. (snowflakedb/drivers#3127)
- Fixed `SQLGetEnvAttr` and `SQLSetEnvAttr` of an unrecognized environment attribute returning HYC00 instead of HY092. (snowflakedb/drivers#3179)
- Fixed SQLExecDirect failing with a parameter-binding error for statements that have no parameters when only a deferred length or indicator pointer was set. (snowflakedb/drivers#3045)
- Fixed SQLExecDirect treating SQL_DESC_OCTET_LENGTH_PTR as a null indicator on non-character parameters. (snowflakedb/drivers#3045)
- Fixed the Linux RPM and deb packages so removing a package unregisters the driver name recorded at install, including a custom `SF_DRIVER_NAME`, and an upgrade keeps that registration unless `SF_DRIVER_NAME` names a different driver, which replaces the old registration. (snowflakedb/drivers#2964)
- Fixed the Linux RPM upgrade so the driver and the `snowflake` DSN stay registered, while an upgrade from a driver version released before this fix registers `SF_DRIVER_NAME` or the default `Snowflake ODBC` name and leaves a different custom name from that version registered. (snowflakedb/drivers#2964)
- Fixed the macOS ODBC driver library so its install name is `/opt/snowflake/snowflakeodbc/lib/libsfodbc.dylib`. (snowflakedb/drivers#3051)
- Fixed the macOS installer so installed driver files are owned by root. (snowflakedb/drivers#3046)
- Fixed the macOS installer so it registers the driver in the system-wide `/Library/ODBC/odbcinst.ini` as well as for the installing user. (snowflakedb/drivers#3055)
- Fixed `SQLGetEnvAttr(SQL_ATTR_DRIVER_UNICODE_TYPE)` returning an unsupported-attribute error, so iODBC 3.52.14 and later converts wide strings to the driver's UTF-16 or UTF-32 encoding. (snowflakedb/drivers#3179)
- Fixed column-wise `SQLBindCol` arrays bound as `SQL_C_DEFAULT` with a zero `BufferLength` writing every fetched row into the first array element instead of advancing by the size of the default C type. (snowflakedb/drivers#3027)
- Fixed `SQLBindParameter` with `SQL_C_DEFAULT` and a `SQL_INTERVAL_*` parameter type reading the bound `SQL_INTERVAL_STRUCT` as a character string instead of as the matching `SQL_C_INTERVAL_*` type. (snowflakedb/drivers#3020)
- Fixed `SQLPutData` rejecting character and binary data bound as `SQL_C_DEFAULT` with `HY019` when it was sent in more than one piece. (snowflakedb/drivers#3027)
- Fixed `SQLBindParameter` with `SQL_C_DEFAULT` and `SQL_REAL` reading the bound value as an 8-byte double instead of a 4-byte `SQL_C_FLOAT`, which inserted wrong values and misread parameter arrays. (snowflakedb/drivers#3010)
- Fixed `SQL_C_DEFAULT` on `SQL_WCHAR`, `SQL_WVARCHAR`, and `SQL_WLONGVARCHAR` treating data as narrow characters instead of `SQL_C_WCHAR`, which truncated or corrupted bound wide-string parameters and returned catalog function wide-character columns as narrow strings. (snowflakedb/drivers#3013)
- Fixed `SQLColumns` and `SQLGetTypeInfo` `NUM_PREC_RADIX` reporting INTEGER instead of the SMALLINT type the ODBC specification defines. (snowflakedb/drivers#2760)
- Fixed `SQLTables` and `SQLColumns` so that, with `CLIENT_METADATA_REQUEST_USE_CONNECTION_CTX` enabled, a non-NULL catalog argument such as `%` is no longer limited to the current schema, and an unset session database or schema no longer makes the call return no rows. (snowflakedb/drivers#2862)
- Fixed the Windows installer so uninstalling or upgrading a driver registered under a custom `DRIVER_NAME` removes that registration when this version or a later one installed it and leaves any other `Snowflake ODBC` registration in place, while an upgrade from an earlier installer still removes the default `Snowflake ODBC` registration unless that version is uninstalled with its original `DRIVER_NAME` first. (snowflakedb/drivers#2963)
- Fixed the Windows ODBC driver so applications can call `SFGetFipsStatus` before connecting. (snowflakedb/drivers#2837)
- Fixed the macOS ODBC installer so the driver-name dialog appears only during an interactive Installer.app install. (snowflakedb/drivers#3048)
- Fixed `validate_default_parameters=true` not failing login when the specified database, schema, or warehouse does not exist. (snowflakedb/drivers#2974)
- Fixed Windows wide-string calls being mis-decoded, which failed or crashed the connection, when `sf.odbc.ini` sets `DriverManagerEncoding=UTF-32`; the driver now uses UTF-16 and logs a warning. (snowflakedb/drivers#2961)

Internal changes:

- Changed `SQL_C_DEFAULT` handling to resolve the C type once from a shared SQL-type table before parameter binding and result conversion, replacing the per-converter `SQL_C_DEFAULT` arms. (snowflakedb/drivers#3027)

## v4.2.0

Changes:

- Changed `SQLDriverConnect` so unrecognized connection-string keywords are reported with a local `01S00` warning and are not forwarded to the server. Recognized session parameters are sent to the server. (snowflakedb/drivers#2712)
- Changed the leftover ODBC 3.x `TRACING` keyword in a DSN or connection string to post a SQLSTATE `01000` deprecation warning on connect that names `LogLevel` in `sf.odbc.ini`; the keyword still has no effect. (snowflakedb/drivers#2806)
- Changed login to omit `CLIENT_SESSION_KEEP_ALIVE` and `CLIENT_PREFETCH_THREADS` unless the connection sets them. (snowflakedb/drivers#2728)
- Changed login to send a set `validate_default_parameters` value, true or false, as `CLIENT_VALIDATE_DEFAULT_PARAMETERS`, and to leave that parameter off the login request when it is unset. (snowflakedb/drivers#2722)

Bug fixes:

- Fixed saving a Windows DSN through the setup dialog or `ConfigDSN` (including `Add-OdbcDsn`) failing with "Failed to save DSN configuration." or a `FALSE` return; saving now leaves a stored `TRACING` value in place. (snowflakedb/drivers#2806)
- Fixed `SQLDriverConnect` so `ODBC_QUERY_RESULT_FORMAT` is sent to the server and selects the session query result format. (snowflakedb/drivers#2753)
- Fixed statement-level `TIMEZONE` overrides so a `TIMEZONE` set on a statement is sent on the query request. (snowflakedb/drivers#2753)
- Fixed `SQLTables` so the `SQL_ALL_CATALOGS` and `SQL_ALL_SCHEMAS` listings return the account's catalogs or schemas when `TableType` is null or any other value, instead of an empty result. (snowflakedb/drivers#2731)
- Changed TLS certificate checks for RUSTSEC-2026-0382 by updating bundled root certificates and distrusting compromised intermediate Certificate Authority (CA) certificates. (snowflakedb/drivers#2859)

## v4.1.0

New features:

- Added recognition of the ODBC 3.x CRL connection-string keys `CRL_ALLOW_NO_CRL`, `CRL_DISK_CACHING`, `CRL_MEMORY_CACHING`, and `CRL_DOWNLOAD_TIMEOUT` as aliases for the matching 4.x CRL settings. `CRL_DOWNLOAD_TIMEOUT=0` applies the 4.x `crl_http_timeout` default of 10 seconds rather than 3.x's 120.
- Added support for the ODBC_USE_CUSTOM_SQL_DATA_TYPES session parameter so TIMESTAMP_LTZ, TIMESTAMP_TZ, and TIMESTAMP_NTZ report vendor type codes 2000/2001/2002, text GEOGRAPHY and GEOMETRY report 2004, and ARRAY, OBJECT, and VARIANT report 2003/2004/2005, when the parameter is true, while binary GEOGRAPHY and GEOMETRY stay SQL_BINARY and SQL_DESC_TYPE_NAME distinguishes the variants regardless of the parameter.
- Added `SFGetFipsStatus` so an application can check, before connecting, whether this process loaded the FIPS driver.
- Added the host application's executable path to login requests as `APPLICATION_PATH`, matching the 3.x driver.
- Added the Snowflake vendor SQL data type macros (`SQL_SF_TIMESTAMP_LTZ`, `SQL_SF_TIMESTAMP_TZ`, `SQL_SF_TIMESTAMP_NTZ`, `SQL_SF_ARRAY`, `SQL_SF_OBJECT`, `SQL_SF_VARIANT`) to the public `sf_odbc.h`, so applications written against the 3.x header compile without redefining them.
- Added the `MapToLongVarchar` connection parameter so CHAR and VARCHAR columns whose size exceeds the threshold are reported as `SQL_LONGVARCHAR`. Unset and negative values keep `SQL_VARCHAR`.
- Added the `DEFAULT_VARCHAR_SIZE` and `DEFAULT_BINARY_SIZE` connection parameters so max-length VARCHAR and BINARY columns report the configured sizes. Unset and negative values keep the session maximum.
- Added SQLGetInfo support for SQL_OUTER_JOINS (ODBC 1.0), returning Y to match the 3.x driver.
- Added the `PUT_COMPRESSLV` connection parameter so PUT AUTO_COMPRESS can select gzip compression levels 0–9. Unset and out-of-range values keep gzip level 6.
- Added the `PUT_TEMPDIR` connection parameter so PUT AUTO_COMPRESS can write gzip tempfiles to a caller-supplied directory. Unset and empty values keep the process temp directory. Nested directories are created.
- Added the Snowflake vendor SQL data type macros (`SQL_SF_TIMESTAMP_LTZ`, `SQL_SF_TIMESTAMP_TZ`, `SQL_SF_TIMESTAMP_NTZ`, `SQL_SF_ARRAY`, `SQL_SF_OBJECT`, `SQL_SF_VARIANT`) to the public `sf_odbc.h`, so applications written against the 3.x header compile without redefining them.
- Added `SQL_SF_VECTOR` (2006) to the public `sf_odbc.h` so applications can name the VECTOR type that `SQLGetTypeInfo` already reports. Binding VECTOR remains unsupported.
- Added per-connect re-reading of `sf.odbc.ini` logging configuration: each `SQLConnect`/`SQLDriverConnect`/`SQLBrowseConnect` now re-resolves `LogLevel`/`LogPath` (and the troubleshooting environment variables) and applies any change in place before the connection proceeds, so editing the ini or dropping in a new one takes effect on the next connect with no process restart. An unchanged configuration is detected by a cheap fingerprint and skipped without reading the file; `DriverManagerEncoding` stays negotiated once per process and DSN `LogLevel`/`LogPath`/`TRACING` remain ignored.
- Added the `wif_host` connection parameter so Workload Identity Federation can target a specific AWS STS endpoint; unset keeps the regional default.
- Added `PAT_WITH_EXTERNAL_SESSION` authentication (`authenticator=PAT_WITH_EXTERNAL_SESSION` with `external_session_id`) so a programmatic access token can reuse a server-side session.

Changes:

- Changed the leftover ODBC 3.x CRL connection-string keywords `CRL_CHECK` and `CRL_ADVISORY` to be accepted and ignored, posting SQLSTATE `01000` on connect. Set `CRL_MODE` to `ENABLED` for fail-close revocation checking or `ADVISORY` for fail-open.
- Changed TLS setup to load the operating-system root certificates once per driver environment instead of on every connection; trust-store changes made after the first connection take effect only after the environment is recreated.

Bug fixes:

- Fixed `SQLColAttribute` to report `ARRAY`/`STRUCT`/`VARIANT` for `SQL_DESC_TYPE_NAME` and `SQL_DESC_LOCAL_TYPE_NAME` instead of always returning `VARCHAR`.
- Fixed query-result `SQL_DESC_TYPE_NAME` so `TIMESTAMP_NTZ`, `TIMESTAMP_LTZ`, and `TIMESTAMP_TZ` are reported separately. `SQLDescribeCol` stays `SQL_TYPE_TIMESTAMP` (93).
- Fixed `SQLBindParameter` and IPD `SQLSetDescRec` to return SQLSTATE `HYC00` (optional feature not implemented) for the Snowflake vendor type codes that `SQLGetTypeInfo` reports but the driver cannot bind: `SQL_SF_ARRAY`, `SQL_SF_OBJECT`, `SQL_SF_VARIANT`, and `SQL_SF_VECTOR`. `SQLBindParameter` previously returned `HY004`; IPD `SQLSetDescRec` previously returned `HY021`. Bind semi-structured values as `SQL_VARCHAR` and coerce in the SQL with `PARSE_JSON(?)`, `TO_ARRAY(?)`, or `TO_OBJECT(?)`. A type code the catalog does not report still returns `HY004` from `SQLBindParameter`.
- Redacted credentials embedded in URL-typed connection parameters (proxy, server URL, OAuth/WIF endpoint URIs, and authenticator) in ODBC connection-parameter log output, including scheme-bearing values with a colon and no `@` unless the authority is a dotted host or bracketed IPv6 with a numeric port; query strings and fragments are also stripped from those parameters.
- Fixed `COPY INTO <location>` so `SQLNumResultCols` and `SQLFetch` expose the unload result rows instead of reporting no result set.
- Fixed `sf.odbc.ini` discovery so a candidate path that is a directory (or otherwise not a regular file) is skipped instead of aborting the search; a `SF_ODBC_INI` pointing at a directory no longer hides the standard per-user `sf.odbc.ini`.

## v4.0.0

Breaking changes:

- Changed catalog functions so a NULL `CatalogName` is no longer replaced with the current database by default. Set `UseCurrentCatalog=true` (or enable `CLIENT_METADATA_REQUEST_USE_CONNECTION_CTX`) to restore that substitution. Unconstrained NULL-catalog searches issue account-wide `SHOW` statements.
- Changed `SQLColumns` `BUFFER_LENGTH` for `DATE`/`TIME` from `COLUMN_SIZE` (`10` / `18` for `TIME(9)`) to `6` (`sizeof(SQL_DATE_STRUCT)` / `sizeof(SQL_TIME_STRUCT)`); query-result `SQLColAttribute` octet length for DATE/TIME remains 6.
- Changed `SQLColumns` `BUFFER_LENGTH` for `NUMBER`/`DECIMAL` to return precision + 2 (ODBC transfer octet length); query-result `SQLColAttribute` octet/display for NUMBER remains 136.
- Changed `SQLColumns` `SQL_DATA_TYPE` for DATE/TIME/TIMESTAMP to the verbose `SQL_DATETIME` (`9`) with the subtype in `SQL_DATETIME_SUB`; `DATA_TYPE` still returns the concise type (`91` / `92` / `93`).
- Changed `SQLColumns` `COLUMN_SIZE` for `TIMESTAMP*` from a fixed `35` to `20` + scale (`19` when scale is `0`).
- Changed `SQLColumns` `BUFFER_LENGTH` for `TIMESTAMP*` from `35` to `16` (`sizeof(SQL_TIMESTAMP_STRUCT)`).
- Changed `SQLColumns` `COLUMN_SIZE` and `BUFFER_LENGTH` for `VARIANT`/`OBJECT`/`ARRAY` to follow `VARCHAR_AND_BINARY_MAX_SIZE_IN_RESULT` instead of SHOW COLUMNS' 128 MB length.
- Changed `SQL_C_CHAR` conversion of DECIMAL/NUMERIC to return `SQL_ERROR` (`22003`) when whole digits do not fit in the buffer; previously returned `SQL_SUCCESS_WITH_INFO` with truncated digits.
- Changed `SQL_C_BINARY` conversion of DECIMAL/NUMERIC/DECFLOAT to return `SQL_ERROR` (`22003`) when the buffer is smaller than `sizeof(SQL_NUMERIC_STRUCT)`; previously `BufferLength` was ignored, risking buffer overflows.
- Changed conversion of NaN `FLOAT`/`DOUBLE` to integer and `SQL_C_BIT` targets to return `SQL_ERROR` (`22003`); previously returned `SQL_SUCCESS_WITH_INFO` with a silent `0`.
- Enforced interval leading-field precision for `SQL_C_INTERVAL_*` types: values that exceed the default precision of 2 now return `SQL_ERROR` (`22015`), and `SQL_DESC_DATETIME_INTERVAL_PRECISION` is respected; previously neither was enforced.
- Changed `SQLRowCount` with a `NULL` `RowCountPtr` to return `SQL_ERROR` (`HY009`); previously returned `SQL_SUCCESS`.
- Changed `SQLBindParameter` to return `SQL_ERROR` (`HY104`) for a negative `DecimalDigits` value; previously accepted negative scale silently.
- Changed conversion of DECFLOAT values with extreme exponents to `SQL_C_BINARY` to return `SQL_ERROR` (`22003`) instead of silently clamping.
- Changed `SQL_C_NUMERIC` to `VARCHAR` conversion to apply the scale from `SQL_NUMERIC_STRUCT` (e.g. magnitude `12345` with scale `2` produces `"123.45"`); previously scale was ignored.
- Changed binding of subnormal double values near `DBL_MIN` to preserve the value; previously stored as `0.0`.
- Changed `TIME` to `SQL_C_CHAR`/`SQL_C_WCHAR` buffer-too-small handling: a buffer that cannot hold the base time (`HH:MM:SS`) now returns `SQL_ERROR` (`22003`); a buffer that truncates only fractional seconds returns `SQL_SUCCESS_WITH_INFO` (`01004`).
- Changed `DATE` to `SQL_C_BINARY` conversion with an undersized buffer to return `SQL_ERROR` (`22003`); previously returned `SQL_SUCCESS` with truncated data.
- Changed `SQLGetFunctions` to report `SQL_API_SQLCANCELHANDLE` as supported; previously returned `SQL_FALSE` despite the function being exported.
- Tightened conversion from VARCHAR to `SQL_C_INTERVAL_*` types. Truncation returns `SQL_SUCCESS_WITH_INFO` (`01S07`) and interval-field overflow returns `SQL_ERROR` (`22015`).
- Changed `SQL_C_INTERVAL_SECOND` with fractional seconds bound to exact-numeric SQL types to truncate the fraction and succeed; previously returned `SQL_ERROR` (`22015`).
- Changed `SQLCancel` during data-at-execution to discard all accumulated `SQLPutData` data so a re-entered sequence starts fresh; previously accumulated data from an interrupted sequence could be concatenated into the next sequence.
- Changed `SQLForeignKeys` to resolve a `NULL` FK catalog and schema from the connection context; previously returned an empty result set unless `CLIENT_METADATA_REQUEST_USE_CONNECTION_CTX` was enabled.
- Changed `SQLSetConnectAttr(SQL_ATTR_LOGIN_TIMEOUT)` called after connect to return `SQL_ERROR` (`HY011`); previously returned `SQL_SUCCESS` as a silent no-op.
- Changed `SQLSetStmtAttr(SQL_ATTR_CURSOR_TYPE)` for unsupported cursor types to substitute `SQL_CURSOR_FORWARD_ONLY` and return `SQL_SUCCESS_WITH_INFO` (`01S02`); previously accepted any value silently.
- Changed the ODBC diagnostic vendor prefix from `[Snowflake][Support]` to `[Snowflake][Snowflake ODBC Driver]`, matching the ODBC spec format `[vendor][ODBC-component-identifier]`.
- Changed `SQLGetInfo(SQL_DRIVER_NAME)` to return the loaded driver library's file name (e.g. `libsfodbc.so`, `libsfodbc.dylib`, `sfodbc.dll`) as the ODBC specification prescribes; previously returned the fixed string `Snowflake`.
- Changed `SQLCancelHandle(SQL_HANDLE_DBC)` to return `SQL_ERROR` (`HY010`) when an associated statement is asynchronously executing or mid data-at-execution; previously returned `SQL_SUCCESS` as a no-op.
- Changed DECFLOAT fetched as `SQL_C_CHAR`/`SQL_C_WCHAR` to return normalized scientific notation (e.g. `1.2e200` instead of `12e199`).
- Changed stage array-binding threshold comparison from `>` to `>=` and added support for the `arrayBindSupported` and `CLIENT_STAGE_ARRAY_BINDING_THRESHOLD` session parameters; previously the server-provided `arrayBindSupported` flag was ignored.
- Removed support for the `SQL_SF_CONN_ATTR_PRIV_KEY` (raw `EVP_PKEY*`) connection attribute; use `PRIV_KEY_CONTENT`, `PRIV_KEY_BASE64`, or `PRIV_KEY_FILE` instead.
- Changed OAuth token and authorization endpoint URL validation to require HTTPS; `http://` is allowed only for loopback addresses.
- Changed the SQLSTATE for OAuth IdP token-exchange rejection from `HY000` to `28000` (invalid authorization specification).
- Changed connection setup to reject WIF-only parameters (e.g. `WORKLOAD_IDENTITY_PROVIDER`) unless `AUTHENTICATOR=WORKLOAD_IDENTITY`; previously silently ignored them with any other authenticator.
- Changed `WORKLOAD_IDENTITY_IMPERSONATION_PATH` with `IDENTITY_PROVIDER=OIDC` to return a connection error; previously silently ignored the combination.
- Changed catalog `REMARKS` and `SQLColumns` `COLUMN_DEF` to return `SQL_NULL_DATA` when absent instead of an empty string, and `SQLTables` `REMARKS` now surfaces a `SHOW OBJECTS` comment when present.
- Changed `SQL_SF_STMT_ATTR_LAST_QUERY_ID` to be read-only and fully populated after every statement execution; changed `SQL_SF_STMT_ATTR_MULTI_STATEMENT_COUNT` to support get/set with a default of `-1` (auto-detect); neither attribute was fully implemented in ODBC 3.x.
- Changed `SQLGetDiagRec` / `SQLGetDiagField` message text to append an internal error trace by default; set `ErrorTraceEnabled=false` in `sf.odbc.ini` to restore 3.x-shaped message text.
- Fixed return codes and diagnostics under iODBC to match the ODBC specification: zero-row DML returns `SQL_NO_DATA`, length-only queries return `SQL_SUCCESS`, and required `SUCCESS_WITH_INFO` / `HY012` diagnostics are posted correctly.
- Fixed orphaned child statement handles under iODBC: `SQLDisconnect` and `SQLFreeStmt(SQL_DROP)` now invalidate all associated handles through iODBC's alloc table so a subsequent `SQLFreeHandle` on an orphaned handle returns `SQL_INVALID_HANDLE` instead of `SQL_SUCCESS`.
- Fixed `SQLGetDescField` and `SQLSetDescField` called during `SQL_NEED_DATA` state under iODBC to return `SQL_ERROR` (`HY010`); previously returned `SQL_SUCCESS`.
- Fixed ODBC 3.x-standard SQLSTATEs under iODBC (e.g. `HY090`, `HY010`, `HY017`, `HY092`, `07009`, `HY008`); previously returned vendor `HY000` or ODBC 2.x aliases.
- Fixed `SQL_C_WCHAR` fetch encoding under iODBC to use a uniform `UTF-32` width driven by `DriverManagerEncoding` in `sf.odbc.ini`; previously the code-unit width varied by Snowflake source column type.
- Changed proxy resolution so `HTTP_PROXY` / `HTTPS_PROXY` / `NO_PROXY` environment variables are ignored unless `USE_PROXY_ENV=true` (alias `PROXYWITHENV`); explicit `PROXY=` / `NO_PROXY=` connection parameters still apply. ODBC 3.x fell back to those environment variables when no DSN/`simba.snowflake.ini` proxy was set. `PROXYWITHENV` no longer writes the connection's `PROXY` value into the process environment.
- Changed `SF_GLOBAL_SSL_VERSION` (and TLS settings) below TLS 1.2 to fail the connection; previously logged a deprecation warning and still negotiated.
- Removed support of OCSP certificate validation in favor of CRL validation.

New features:

- Added the `MapToLongVarchar` connection parameter so CHAR and VARCHAR columns whose size exceeds the threshold are reported as `SQL_LONGVARCHAR`. Unset and negative values keep `SQL_VARCHAR`.
- Added the `DEFAULT_VARCHAR_SIZE` and `DEFAULT_BINARY_SIZE` connection parameters so max-length VARCHAR and BINARY columns report the configured sizes. Unset and negative values keep the session maximum.
- Added SQLGetInfo support for SQL_OUTER_JOINS (ODBC 1.0), returning Y to match the 3.x driver.
- Added the `PUT_COMPRESSLV` connection parameter so PUT AUTO_COMPRESS can select gzip compression levels 0–9. Unset and out-of-range values keep gzip level 6.
- Added the `PUT_TEMPDIR` connection parameter so PUT AUTO_COMPRESS can write gzip tempfiles to a caller-supplied directory. Unset and empty values keep the process temp directory. Nested directories are created.
- Added recognition of the 3.x `MaxHttpRetries` connection-string key as an alias of `retry_max_attempts`.
- Added a `PUT_GET_MAX_ATTEMPTS` connection parameter for the shared PUT/GET attempt limit, and accepted the 3.x `PUT_MAXRETRIES` / `GET_MAXRETRIES` spellings as aliases that warn (`01000`) on use.
- Added the ODBC `UseCurrentCatalog` connection parameter (default false), matching the 3.x DSN key. When true, a NULL `CatalogName` on `SQLTables`, `SQLColumns`, `SQLPrimaryKeys`, `SQLForeignKeys`, `SQLProcedures`, and `SQLProcedureColumns` is the current database.
- Added TRACE-level entry and exit logs for all public ODBC functions.
- Added `INTERVAL YEAR TO MONTH` and `INTERVAL DAY TO SECOND` result support: `SQL_C_CHAR`/`SQL_C_WCHAR` fetch returns the canonical ANSI literal (`[-]Y-MM`, `[-]D HH:MM:SS[.f]`), same-family `SQL_C_INTERVAL_*` targets receive the parsed interval struct, and scalar numeric targets receive total months or total whole seconds (reporting `01S07` when sub-second precision is dropped).
- Added native AKS Workload Identity support for Azure: when the Azure Workload Identity webhook injects `AZURE_CLIENT_ID`, `AZURE_TENANT_ID`, and `AZURE_FEDERATED_TOKEN_FILE` into a pod and the projected token file exists on disk, `WORKLOAD_IDENTITY_PROVIDER=AZURE` exchanges that federated token for an Entra ID access token directly. `WORKLOAD_IDENTITY_IMPERSONATION_PATH` is not supported in this environment.
- Added a `WORKLOAD_IDENTITY_AWS_USE_OUTBOUND_TOKEN` connection parameter for AWS Workload Identity Federation. When set to true, attestation uses outbound STS `GetWebIdentityToken` instead of the default pre-signed `GetCallerIdentity` token; the connection parameter takes precedence over `SNOWFLAKE_ENABLE_AWS_WIF_OUTBOUND_TOKEN`.
- Added `INCLUDE_RETRY_REASON` (default true) so retried query requests send `retryReason` (the HTTP status that triggered the retry, or `0` for transport failures) alongside `retryCount`.
- Implemented `SQLFreeConnect` (ODBC 2.x) as a thin wrapper around `SQLFreeHandle(SQL_HANDLE_DBC, …)` for direct-link and ODBC 2.x applications that bypass the Driver Manager.
- Implemented `SQLFreeEnv` (ODBC 2.x) as a thin wrapper around `SQLFreeHandle(SQL_HANDLE_ENV, …)` for direct-link and ODBC 2.x applications that bypass the Driver Manager.
- Added a `TOKEN_FILE_PATH` connection parameter that loads a PAT, legacy OAuth, or OIDC bearer token from a file; when both `TOKEN` and `TOKEN_FILE_PATH` are set, the file contents are used.
- Added VECTOR column fetch: VECTOR columns are returned as compact JSON array strings (e.g. `[1,2,3]`) via `SQL_C_CHAR`, `SQL_C_WCHAR`, and `SQL_C_BINARY`; `SQLDescribeCol` reports `SQL_VARCHAR` and `SQL_DESC_TYPE_NAME` returns `VECTOR`.
- Added `ALTER USER ADD PROGRAMMATIC ACCESS TOKEN` result-set support: the generated token is returned as a result row; ODBC 3.x returned an invalid cursor state for this DDL.
- Implemented `SQLTransact` (ODBC 2.x) as a mapping to `SQLEndTran`, restoring compatibility with ODBC 2.x applications that bypass the Driver Manager.
- Added `TIME` to `SQL_C_BINARY` conversion support.
- Added `SQL_C_BINARY` as `SQLBindParameter` source type for `SQL_DECIMAL`, `SQL_NUMERIC`, `SQL_TIME`, and `SQL_TIMESTAMP` when the buffer length matches the SQL data length.
- Added PUT local-path tilde expansion: a leading `~/` or `~` in the source path is expanded to the user's home directory.
- Added automatic creation of a missing destination directory for `GET` operations.
- Added `SQLGetDiagField(SQL_DIAG_SERVER_NAME)` population: returns the connected server name instead of an empty string.
- Added `SQLGetDiagField(SQL_DIAG_DYNAMIC_FUNCTION)` / `SQL_DIAG_DYNAMIC_FUNCTION_CODE` population after statement execution (e.g. SELECT → `85` / `SQL_DIAG_SELECT_CURSOR`); previously returned empty / `0`.
- Added `SQL_C_BINARY` to `VARCHAR` binding: binary bytes are hex-encoded before send (e.g. `0xDEADBEEF` → `"deadbeef"`); ODBC 3.x forwarded raw bytes and the server rejected the request with HTTP 400.
- Added `SQL_SF_TIMESTAMP_TZ` parameter binding for `SQL_C_TYPE_TIMESTAMP` (stored at UTC) and `SQL_C_CHAR`/`SQL_C_WCHAR` strings with a `+/-HH:MM` offset.
- Added `SQLBindParameter` support for binding to `SQL_INTERVAL_*` parameters from same-family `SQL_C_INTERVAL_*`, `SQL_C_CHAR`/`SQL_C_WCHAR`, and exact-numeric C sources (approximate-numeric sources are rejected with `SQL_ERROR` `07006`).
- Added `SQL_SF_CONN_ATTR_APPLICATION` as a pre-connect get/set attribute that forwards the application name to the server.
- Added `SQLGetFunctions` reporting of `SQL_API_SQLSETSCROLLOPTIONS` and `SQL_API_SQLPARAMOPTIONS` as supported under iODBC.

Changes:

- Changed `SQLRowCount` after a SELECT capped by `ROWS_PER_RESULTSET` so it reports the number of rows in the result payload, matching ODBC 3.x.
- Changed leftover ODBC connection-string logging keywords such as `LogLevel` and `LogPath` to be accepted and ignored, posting SQLSTATE `01000` on connect. Each warning names the `sf.odbc.ini` key that configures the same behavior (for example `LogFileCount` points at `LogMaxCount`).
- Changed leftover ODBC test-only connection-string knobs (`INJECT_CURL_TIMEOUT`, `INJECT_INCIDENT1`, `CURL_NO_IDLE_CHECK`, `DisableStageBindFallback`, `StageBindMaxFileSize`, `StageBindThreshold`) to be accepted and ignored, posting SQLSTATE `01000` on connect. These knobs have no replacement; remove them from the DSN.
- Changed the ODBC Driver Manager `CPTimeout` connection-string keyword to be accepted and ignored instead of treated as an unknown session parameter.
- Changed leftover ODBC connection-string OCSP keywords `DisableOCSPCheck` and `OCSP_FAIL_OPEN` to be accepted and ignored, posting SQLSTATE `01000` on connect. Certificate revocation uses CRL; set `CRL_MODE` to `DISABLED`, `ENABLED`, or `ADVISORY`.
- Changed `CLIENT_STORE_TEMPORARY_CREDENTIAL` to default to true when the caller has not set it. An explicit value always wins.
- Changed leftover ODBC connection-string keyword `TRANSLATE` to be accepted and ignored, posting SQLSTATE `01000` on connect. Character-set translation DLLs are not supported.
- Changed `SQLGetInfo(SQL_ASYNC_DBC_FUNCTIONS)` to report `SQL_ASYNC_DBC_NOT_CAPABLE`, matching the driver's rejection of `SQL_ATTR_ASYNC_DBC_FUNCTIONS_ENABLE`; on Windows the Driver Manager now rejects that attribute with `HY114` before the driver sees it, instead of the driver's `HY092`.
- Changed `SQLSetConnectAttr(SQL_ATTR_ASYNC_DBC_FUNCTIONS_ENABLE)` so unixODBC and iODBC accept `SQL_ASYNC_DBC_ENABLE_OFF` and read it back, while enabling still fails with `HYC00`; on Windows the Driver Manager rejects both ON and OFF with `HY114` once GetInfo reports `NOT_CAPABLE`.
- Changed `SQLGetTypeInfo` `TIMESTAMP` `COLUMN_SIZE` from `35` to `29` (the ODBC-standard display length). `TIMESTAMP_LTZ`, `TIMESTAMP_NTZ`, and `TIMESTAMP_TZ` remain `35`. The 3.x `ODBC_USE_STANDARD_TIMESTAMP_COLUMNSIZE` connection parameter is not accepted.
- Changed `SQLDriverConnect` to reject connection-string keywords it does not recognize with a local `01S00` warning (native error 17, "N invalid keys are found in the connection string: <KEY>"); a keyword is recognized when the `sf_core` parameter registry resolves it or it names an ODBC structural keyword (`DSN`, `DRIVER`, `FILEDSN`, `SAVEFILE`). The connection still opens and the keyword is still forwarded to the server.
- Removed `Tracing(0-6)` field from the Windows ODBC DSN setup dialog; driver logging uses `sf.odbc.ini` (`LogLevel`, `LogPath`) instead. Legacy `TRACING` values in a DSN or connection string are ignored.
- Changed ODBC driver-manager connection-string keywords such as `DSN` and `DRIVER` to be accepted and ignored instead of forwarded as unknown session parameters.
- Changed client-local rejection of a non-credential connection parameter to report SQLSTATE `HY000` instead of the warning-class `01S00`; affected cases are an invalid `PORT`, an unparseable connection string, and any invalid or missing non-credential parameter. `01S00` is defined by the ODBC specification as a `SQL_SUCCESS_WITH_INFO` warning meaning the connection opened anyway, so returning it on `SQL_ERROR` led applications that branch on the SQLSTATE class to treat a failed connection as a warning. ODBC 3.x returned `28000` (native error `20032`) for these cases.
- Changed a missing key-pair credential (`PRIVATE_KEY` / `PRIVATE_KEY_FILE`) and a missing bearer token (`TOKEN` / `TOKEN_FILE_PATH`) to report SQLSTATE `28000` instead of `01S00`, matching ODBC 3.x and the SQLSTATE already reported when either parameter is named on its own.
- Changed rejection of a Workload Identity Federation parameter (`WORKLOAD_IDENTITY_PROVIDER`, `WORKLOAD_IDENTITY_ENTRA_RESOURCE`, `WORKLOAD_IDENTITY_IMPERSONATION_PATH`, `WORKLOAD_IDENTITY_AWS_USE_OUTBOUND_TOKEN`) to report SQLSTATE `28000` rather than a general error, so a missing or invalid one is reported as the authentication failure it is. ODBC 3.x reported `28000` for a missing `WORKLOAD_IDENTITY_PROVIDER`.
- Changed the diagnostic message on a failed login to lead with the server's own text: `SQLGetDiagRec` now reads `Failed to login: Login error: <server text>, code: <code>` on its first line, where the server sentence previously appeared only inside the error trace. SQLSTATE and native error code are unchanged.
- Improved log output to mask OAuth client IDs and AWS access-key IDs.
- Changed `SQLBrowseConnect` so an incomplete connection string returns `SQL_NEED_DATA` and keeps the handle available for further browse calls, matching the 3.x iterative protocol under iODBC.
- Changed OAuth Authorization Code connections to default `CLIENT_STORE_TEMPORARY_CREDENTIAL` to true when the caller has not set it, matching ODBC 3.x token caching.
- Changed `SQLForeignKeys` with `SQL_ATTR_METADATA_ID=TRUE` to return `SQL_ERROR` (`HY009`) for a `NULL` catalog, schema, or table pointer on either side.
- Changed `SQL_C_BINARY` fetch of `FLOAT`/`DOUBLE`/`REAL` to return the native 8-byte IEEE 754 value instead of a 19-byte `SQL_NUMERIC_STRUCT`.
- Changed `SQL_BIT` parameter binding so integer and `SQL_C_NUMERIC` sources accept only `0` and `1` (other magnitudes return `22003`).
- Changed `SQL_C_CHAR`/`SQL_C_WCHAR` binding of `"Infinity"`, `"-Infinity"`, and `"NaN"` to `SQL_FLOAT`/`SQL_REAL`/`SQL_DOUBLE` to forward the non-finite value instead of returning `22018`.
- Changed `SQL_C_CHAR`/`SQL_C_WCHAR` hex literals bound to `SQL_BINARY` so an odd-length hex string drops the leftover nibble and succeeds.
- Changed `SQLBindParameter` with Snowflake vendor TIMESTAMP codes (`2000` / `2001` / `2002`) to store those codes on the IPD so `SQLDescribeParam` returns them as bound.
- Changed `SQLSetStmtAttr(SQL_ROWSET_SIZE, 0)` to return `SQL_ERROR` (`HY024`) instead of storing `0` or coercing to `1`.
- Changed PUT result `source_compression` / `target_compression` tokens to lowercase (e.g. `gzip`), matching ODBC 3.x.
- Changed gzip-compressed PUT uploads to omit the original filename from the gzip `FNAME` header, matching ODBC 3.x.
- Changed PUT and GET to transfer several files at once, bounded by the statement `PARALLEL` value; result rows keep their original file order.
- Changed an unreadable or empty `TOKEN_FILE_PATH` to report SQLSTATE `28000` instead of `01S00`.
- Improved GET to warn when a downloaded batch contains multiple files that resolve to the same local filename.
- Improved external-browser callback handling to cap HTTP header size on the localhost listener.
- Changed `SQLCancel` to cancel through the statement's operation handle so the executing path aborts the query on the server; `SQLCancel` returns once the cancel is signaled, and the canceled call still reports `HY008` after the abort is issued.
- Changed query, cancel, and login timeouts to report SQLSTATE `HYT00` when no server SQLSTATE is present.
- Changed `SQL_C_CHAR`/`SQL_C_WCHAR` fetch of `+/-Infinity` `FLOAT`/`REAL` to return `INFINITY` / `-INFINITY` instead of `inf` / `-inf`.
- Changed PUT to resolve relative source paths to an absolute canonical form before upload.
- Changed `TIMESTAMP_TZ` Arrow fetch to reject a timezone offset outside the valid biased range 0 through 2880.
- Changed `TIMESTAMP_TZ` Arrow fetch to reject a flat Int64 physical type instead of decoding it as UTC with offset 0.
- Renamed the default driver registration name to `Snowflake ODBC`; update DSN `Driver=` entries that reference `SnowflakeDSIIDriver` (custom names still configurable via `DRIVER_NAME=` / `SF_DRIVER_NAME`).
- Changed distributable ODBC package filenames to `snowflake-odbc-<version>.<arch>.<extension>` with unified architectures `aarch64` / `x86_64` / `x86_32` / `universal`.
- Changed `SQLGetInfo(SQL_DRIVER_VER)` to return the zero-padded `MM.mm.bbbb` format (e.g. `04.00.0000`) per the ODBC specification instead of the unpadded semver string (e.g. `3.16.0`).
- Changed process-wide driver configuration to use `sf.odbc.ini` (searched in `$SF_ODBC_INI`, `~/.config/snowflake/sf.odbc.ini`, `~/.snowflake/sf.odbc.ini`, and `/opt/snowflake/snowflakeodbc/sf.odbc.ini`); update deployment scripts that write `simba.snowflake.ini`.
- Changed `PASSCODEINPASSWORD` connection parameter to accept `true` and `1` in addition to `on`.
- Changed `QUERY_TAG` connection parameter to be applied as a server session parameter at connection time.
- Changed the on-disk credential cache file from `credential_cache_v1.json` to `credential_cache_v2.json` (same lookup order: `$SF_TEMPORARY_CREDENTIAL_CACHE_DIR`, `$XDG_CACHE_HOME/snowflake`, `$HOME/.cache/snowflake`). Tokens cached by ODBC 3.x are not read by 4.x, so expect one extra authentication after upgrading.
- Changed GET downloads on Unix to create files with owner-only (`0600`) permissions by default; set `UNSAFE_FILE_WRITE=true` to use the process umask.
- Changed `PRIV_KEY_FILE` / `private_key_file` reads on Unix to require owner-only permissions (mode `0600`); overly permissive key files now fail unless `UNSAFE_SKIP_CONFIG_FILE_PERMISSIONS_CHECK=true`.
- Changed `AUTHENTICATOR=WORKLOAD_IDENTITY` to reject hosts outside the recognized Snowflake suffixes (`snowflakecomputing.com` / `.cn` / `.mil`) before fetching cloud credentials; extend the list only via the `SNOWFLAKE_WIF_ALLOWED_HOST_SUFFIXES` environment variable.
- Changed client-side-encryption `KeyWrappingMetadata.EncryptionLibrary` metadata to `"Rust(OpenSSL)"`.
- Changed client-side rejection of invalid `ACCOUNT` / `SERVER` / `PORT` characters to SQLSTATE `01S00` (native error `0`); ODBC 3.x returned `28000` (native error `20032`).
- Changed `CLIENT_SESSION_KEEP_ALIVE_HEARTBEAT_FREQUENCY` to be sent as a server session parameter during login (queryable via `SHOW PARAMETERS`) in addition to configuring the client heartbeat interval; ODBC 3.x used it client-side only.
- Changed control-plane response body reads with a cap of 20MB (OAuth, browser, GCP metadata, CRL)
- Changed strict owner-only permissions and single-open I/O in CRL cache handling

Bug fixes:

- Fixed stage-bind TIME cells in the CSV upload so they use `HH:MM:SS` instead of nanoseconds since midnight, matching ODBC 3.x bulk binding.
- Fixed `SQLPrepare` on statements the server declines to describe, such as `ALTER SESSION` and `COMMIT`, which previously failed with SQLSTATE `0A000` and native error `7` (statement not preparable); prepare now succeeds and the statement runs on `SQLExecute`, matching ODBC 3.x.
- Fixed query-result metadata so `SQL_DESC_TYPE_NAME` reports `GEOGRAPHY` or `GEOMETRY` while preserving the text or binary concise type selected by the output format.
- Fixed GET of a staged path that matches no object so it returns an empty result set, matching ODBC 3.x.
- Fixed `SQLExecute`/`SQLExecDirect` to return SQLSTATE `07S01` (Invalid use of default parameter) when any bound parameter has `StrLen_or_IndPtr = SQL_DEFAULT_PARAM (-5)`; previously the driver returned `HY000` where the ODBC spec requires `07S01`.
- Fixed `SQL_TINYINT` parameter binds so a value outside the `-128` to `255` range is rejected at execute with SQLSTATE `22003` (Numeric value out of range) instead of being sent to the server unchecked.
- Fixed a parameter bound with a null data pointer and no `SQL_NULL_DATA` indicator so execute reports SQLSTATE `HY009` (Invalid use of null pointer) instead of the general error `HY000`.
- Fixed `SQLExecDirect` so lowering the application parameter descriptor `SQL_DESC_COUNT` after extra parameters were bound no longer fails a statement that uses fewer placeholders.
- Fixed `SQLBindParameter` so an unsupported C-to-SQL conversion is rejected at bind with SQLSTATE `07006`, matching ODBC 3.x.
- Fixed `SQL_C_DEFAULT` parameter binds for DECIMAL, integer, DATE, TIME, and TIMESTAMP so execute infers the ODBC default C type instead of failing with SQLSTATE `07006`.
- Fixed `SQL_PARAM_OUTPUT` binds so they are omitted from execute-time JSON/CSV encoding instead of failing with SQLSTATE `HY009` when the output buffer pointer is null.
- Fixed integer parameters bound as `SQL_C_CHAR` or `SQL_C_WCHAR` so a non-numeric literal returns SQLSTATE `22018` instead of `07006`.
- Fixed `SQLPrepare` + `SQLExecute` for `PUT`/`GET` file-transfer commands, which previously failed with server error `000007` (statement not preparable) because prepare issued a `describeOnly` request the server rejects; prepare now skips the describe and the transfer runs on execute.
- Fixed `SQLGetTypeInfo` to return type information matching the application's configured ODBC version.
- Fixed catalog result-set string columns to report `SQL_WVARCHAR` metadata consistently, including after `SQLPrimaryKeys` and `SQLForeignKeys` results are installed on the statement; `SQLStatistics` `ASC_OR_DESC` reports `SQL_WCHAR`.
- Fixed intermittent key-pair `SQLDriverConnect` failures on Windows x64 Azure during JWT login.
- Fixed `SQL_C_DEFAULT` on catalog `SMALLINT` and `INTEGER` columns so `SQLGetTypeInfo` and `SQLColumns` return binary integers instead of failing with SQLSTATE 22003.
- Fixed INTERVAL DAY-TIME result fetch for subtypes whose Arrow scale is not TIME precision (`MINUTE TO SECOND`, `SECOND`, and truncated `DAY TO SECOND` fractions).
- Fixed `SQLColumns` sizes for `GEOGRAPHY` and `GEOMETRY` to follow the session VARCHAR maximum.
- Enforced restrictive file permissions when loading ODBC DSN files on Unix.
- Fixed row-wise fetches with unaligned `SQL_ATTR_ROW_BIND_TYPE` strides so length, indicator, character, wide-character, and fixed-width value writes do not crash.
- Fixed `SQL_C_CHAR`/`SQL_C_WCHAR` binds to `TIMESTAMP_TZ` columns rejecting ISO8601 strings with a `T` date-time separator (e.g. `"2017-11-30T18:17:05.123456789+08:00"`); the parser now accepts both the space and `T` separator variants.
- Fixed `SQLGetDiagField` return codes for three edge cases: a record field requested with `RecNumber=0` now returns `SQL_ERROR` instead of `SQL_NO_DATA`, a header field requested with a positive `RecNumber` now returns `SQL_SUCCESS` instead of `SQL_NO_DATA`, and a negative `BufferLength` for a string field now returns `SQL_ERROR`.
- Fixed array/batch parameter binding to retry the execute with inline JSON when the `SYSTEM$BIND` stage is disabled, instead of failing the statement.
- Fixed the file-based token cache changing the mode of a cache file that is not `0600` and then using it anyway; such a file is now reported and left unused.
- Fixed connections failing when `CLIENT_SESSION_KEEP_ALIVE_HEARTBEAT_FREQUENCY` falls outside the accepted range; the value is now clamped before login.
- Fixed a `Driver=`-only connect with no other connection-string attributes to load the default `connections.toml` profile.
- Fixed session-parameter reads used by `SQLGetConnectAttr` and decimal-as-int conversion to honor typed values returned by the server (e.g. `AUTOCOMMIT` after `ALTER SESSION`).
- Fixed queries returning a `FILE` column failing with `Unsupported column type`.
- Fixed queries returning a `MAP` column failing with `Unsupported column type`.
- Fixed `SQLColumns` and `SQLProcedureColumns` `CHAR_OCTET_LENGTH` for `VARIANT`/`OBJECT`/`ARRAY` to report the session VARCHAR max (equal to `BUFFER_LENGTH`) instead of NULL.
- Fixed `SQLColumns` `COLUMN_SIZE` and `BUFFER_LENGTH` for unrecognized Snowflake types such as `GEOGRAPHY`/`GEOMETRY` to report the varchar metrics implied by their `SQL_VARCHAR` `DATA_TYPE` instead of NULL.
- Fixed `SQLProcedureColumns` `TYPE_NAME` for unsupported types such as `GEOGRAPHY`/`GEOMETRY` to report the Snowflake type name while `DATA_TYPE` remains `SQL_VARCHAR`.
- Fixed `SQLColumns` and `SQLProcedureColumns` `CHAR_OCTET_LENGTH` for unsupported types such as `GEOGRAPHY`/`GEOMETRY` to report a byte length instead of NULL, matching the `SQL_VARCHAR` they report as `DATA_TYPE`.
- Fixed `SQLGetTypeInfo` string result columns (`TYPE_NAME`, `LITERAL_PREFIX`/`SUFFIX`, `CREATE_PARAMS`, `LOCAL_TYPE_NAME`) to report `SQL_WVARCHAR` as the IRD concise type, matching `SQLTables`/`SQLColumns`.
- Fixed `SQLGetTypeInfo` `INTERVAL_PRECISION` to report `SQL_SMALLINT` as the IRD concise type, matching the ODBC spec and the reference driver; `NUM_PREC_RADIX` remains `SQL_INTEGER`.
- Fixed reauth-required authentication failures to report SQLSTATE `08001` instead of `28000`.
- Fixed an empty `ACCOUNT` value hanging until login timed out; it is now rejected immediately.
- Fixed cancelling a PUT or GET to abort the in-flight cloud transfer and remove any partial `.part` download file.
- Fixed a client-side query timeout to abort the query on the server instead of giving up locally and leaving it running.
- Fixed JSON-format decode of timestamps just before the Unix epoch so the fractional second does not shift the instant forward by one second.
- Fixed PUT with overwrite disabled to skip an existing stage object instead of replacing it.
- Fixed `SQLColumns` and `SQLProcedureColumns` `BUFFER_LENGTH` and `CHAR_OCTET_LENGTH` for `VARCHAR`/`TEXT` to report Snowflake `byteLength`, falling back to 4× `COLUMN_SIZE` capped at the session VARCHAR byte maximum where no `byteLength` is available; query-result `SQLColAttribute` octet length is unchanged.
- Fixed failed `GET` downloads to leave no partial or corrupt file at the destination; the driver now writes to a `.part` temporary file and renames it on successful completion.
- Fixed concurrent `SQLDisconnect` to be thread-safe.
- Fixed `SQLDisconnect` to free child statement handles and explicitly allocated descriptors after a successful disconnect so a later `SQLFreeHandle` on those handles returns `SQL_INVALID_HANDLE`.
- Fixed `SQLDisconnect` to return `SQL_ERROR` (`HY010`) without disconnecting when a child statement is asynchronously executing or mid data-at-execution.
- Fixed `SQLCancel` on a statement with `SQL_ATTR_ASYNC_ENABLE` to actually interrupt the in-progress operation: subsequent polling returns `SQL_ERROR` (`HY008`) once the cancellation is acknowledged. Previously polling kept returning `SQL_STILL_EXECUTING` until the query completed naturally.
- Fixed cross-thread `SQLCancel` and `SQLCancelHandle` to always return `SQL_SUCCESS` and post no diagnostics of their own, per the ODBC specification; previously they intermittently returned `SQL_ERROR` (`HY008`, native error 604) when the server-side abort raced the query's completion. Only the canceled function returns `HY008`.
- Fixed FLOAT/DOUBLE boundary values (`FLT_MAX`, `DBL_MAX`) being incorrectly rejected with a numeric out-of-range error; they are now returned successfully.
- Fixed a crash during `TIMESTAMP` to `SQL_C_CHAR`/`SQL_C_WCHAR` conversion when the destination buffer was too small.
- Fixed `SQLNumResultCols` and `SQLDescribeCol` to return the correct column count and metadata after a prepared statement's cursor is closed - including the async `SQLPrepare` + `SQLExecute` + `SQLCloseCursor` path - instead of returning incorrect results or errors.
- Fixed `SQLDescribeCol` column size for `SQL_DOUBLE` / `SQL_FLOAT` to report `15` (decimal digit precision) instead of `53` (binary mantissa bits).
- Fixed `SQLColumns` `NUM_PREC_RADIX` for FLOAT/DOUBLE/REAL to return `10`, matching decimal `COLUMN_SIZE` (query-result `SQLColAttribute` radix for DOUBLE remains `2`).
- Fixed `SQLFreeHandle(SQL_HANDLE_DESC)` on an implicitly allocated descriptor to return `SQL_ERROR` (`HY017`) and leave the handle valid.
- Fixed external-browser SSO/OAuth on WSL/Linux to validate the browser URL before launching the browser.
- Fixed `SQL_C_CHAR` fetch of `FLOAT`/`DOUBLE`/`REAL` with a truncated buffer to post SQLSTATE `01004`.
- Fixed `SQL_C_WCHAR` chunked `SQLGetData` buffer capacity to use `sizeof(SQLWCHAR)` so an 8-byte buffer fits three data characters plus NUL.
- Fixed `SQL_C_WCHAR` decimal-string binding to `SQL_DECIMAL` to convert correctly on all platforms.
- Fixed crashes under iODBC during some `SQLFreeHandle` / `SQLDisconnect` / `SQLCopyDesc` handle-hierarchy sequences that aborted or segfaulted.
- Fixed connection-attribute state after `SQLDisconnect` under iODBC: `SQL_ATTR_CONNECTION_DEAD` reports `SQL_CD_TRUE` and reads of other connection attributes return `SQL_ERROR` instead of stale cached values.
- Fixed `SQLGetData` conversion from `DECFLOAT` to `SQL_C_WCHAR` under iODBC to convert the value instead of returning `SQL_SUCCESS` with `SQL_NULL_DATA` and an untouched buffer.
- Fixed `SQLColAttribute` to map ODBC 2.x field identifiers (`SQL_COLUMN_NAME`, `SQL_COLUMN_TYPE`, etc.) to their `SQL_DESC_*` equivalents as previous behavior was inconsistent (empty strings, wrong values, or `HY091`).
- Fixed `SQLPrimaryKeys` / `SQLForeignKeys` with `SQL_ATTR_METADATA_ID=TRUE` to case-fold unquoted identifiers to uppercase before matching instead of comparing case-sensitively and returning empty results for lowercase names.
- Fixed `SQLProcedures` / `SQLProcedureColumns` with `SQL_ATTR_METADATA_ID=TRUE` to case-fold unquoted identifiers to uppercase before matching instead of returning empty results for lowercase names.
- Fixed `SQLTables` with `SQL_ATTR_METADATA_ID=TRUE` to case-fold unquoted identifiers to uppercase before matching instead of returning empty results for lowercase names.
- Fixed `SQLGetData` with `SQL_C_NUMERIC` to honor `SQL_DESC_PRECISION` and `SQL_DESC_SCALE` set on the ARD via `SQLSetDescField`instead of hardcoded precision `38` / scale `0`.
- Fixed `DATE` to `SQL_C_CHAR`/`SQL_C_WCHAR` conversion with an undersized buffer to return `SQL_ERROR` (`22003`) instead of truncating.
- Fixed `SQLGetDescField` on an empty IPD (no parameters bound) to return `SQL_NO_DATA` instead of `SQL_ERROR`.
- Fixed `SQLGetStmtAttr` / `SQLGetConnectAttr` with a negative string `BufferLength` to return `SQL_ERROR` (`HY090`).
- Fixed `SQLGetConnectAttr` with an out-of-range attribute identifier to return `SQL_ERROR` (`HY092`).
- Fixed `SQLSetConnectAttr` with ODBC 2.x statement-level attribute IDs (`SQL_ATTR_MAX_ROWS`, `SQL_ATTR_QUERY_TIMEOUT`) to return `SQL_ERROR` (`HY092`) instead of `SQL_SUCCESS` as a silent no-op.
- Fixed fractional truncation on a numeric-to-character fetch to return `SQL_SUCCESS_WITH_INFO` with SQLSTATE `01S07`.
- Fixed `FLOAT`/`REAL` to single-field interval fetch with a nonzero fractional part to return `SQL_SUCCESS_WITH_INFO` (`01S07`).
- Fixed numeric-to-interval conversion so a value that truncates to zero always yields `+0` (no negative-zero interval sign).
- Fixed binary fetches so a negative buffer length or a null output pointer leaves the application buffer unchanged. (snowflakedb/drivers#2415)
- Fixed SQL_NTS character parameter binds so a value with no null terminator inside the buffer is read only up to its buffer length instead of past the end. (snowflakedb/drivers#2416)

Internal changes:

- Changed ODBC teardown so the last environment free joins `crl-refresh` by dropping the driver. A later environment starts a new in-memory CRL cache; a warm disk entry is still a hit. (snowflakedb/drivers#2429)

## v4.0.0-rc4

Breaking changes:

- Changed catalog functions so a NULL `CatalogName` is no longer replaced with the current database by default. Set `UseCurrentCatalog=true` (or enable `CLIENT_METADATA_REQUEST_USE_CONNECTION_CTX`) to restore that substitution. Unconstrained NULL-catalog searches issue account-wide `SHOW` statements. (snowflakedb/drivers#2005)

New features:

- Added recognition of the 3.x `MaxHttpRetries` connection-string key as an alias of `retry_max_attempts`.
- Added a `PUT_GET_MAX_ATTEMPTS` connection parameter for the shared PUT/GET attempt limit, and accepted the 3.x `PUT_MAXRETRIES` / `GET_MAXRETRIES` spellings as aliases that warn (`01000`) on use. (snowflakedb/drivers#1884)
- Added the ODBC `UseCurrentCatalog` connection parameter (default false), matching the 3.x DSN key. When true, a NULL `CatalogName` on `SQLTables`, `SQLColumns`, `SQLPrimaryKeys`, `SQLForeignKeys`, `SQLProcedures`, and `SQLProcedureColumns` is the current database. (snowflakedb/drivers#2005)
- Added TRACE-level entry and exit logs for all public ODBC functions. (snowflakedb/drivers#2018)

Changes:

- Changed `SQLGetTypeInfo` `TIMESTAMP` `COLUMN_SIZE` from `35` to `29` (the ODBC-standard display length). `TIMESTAMP_LTZ`, `TIMESTAMP_NTZ`, and `TIMESTAMP_TZ` remain `35`. The 3.x `ODBC_USE_STANDARD_TIMESTAMP_COLUMNSIZE` connection parameter is not accepted. (snowflakedb/drivers#2035)
- Changed `SQLDriverConnect` to reject connection-string keywords it does not recognize with a local `01S00` warning (native error 17, "N invalid keys are found in the connection string: <KEY>"); a keyword is recognized when the `sf_core` parameter registry resolves it or it names an ODBC structural keyword (`DSN`, `DRIVER`, `FILEDSN`, `SAVEFILE`). The connection still opens and the keyword is still forwarded to the server. (snowflakedb/drivers#1926)
- Removed `Tracing(0-6)` field from the Windows ODBC DSN setup dialog; driver logging uses `sf.odbc.ini` (`LogLevel`, `LogPath`) instead. Legacy `TRACING` values in a DSN or connection string are ignored. (snowflakedb/drivers#2022)
- Changed ODBC driver-manager connection-string keywords such as `DSN` and `DRIVER` to be accepted and ignored instead of forwarded as unknown session parameters. (snowflakedb/drivers#2033)
- Changed client-local rejection of a non-credential connection parameter to report SQLSTATE `HY000` instead of the warning-class `01S00`; affected cases are an invalid `PORT`, an unparseable connection string, and any invalid or missing non-credential parameter. `01S00` is defined by the ODBC specification as a `SQL_SUCCESS_WITH_INFO` warning meaning the connection opened anyway, so returning it on `SQL_ERROR` led applications that branch on the SQLSTATE class to treat a failed connection as a warning. ODBC 3.x returned `28000` (native error `20032`) for these cases. (snowflakedb/drivers#1883)
- Changed a missing key-pair credential (`PRIVATE_KEY` / `PRIVATE_KEY_FILE`) and a missing bearer token (`TOKEN` / `TOKEN_FILE_PATH`) to report SQLSTATE `28000` instead of `01S00`, matching ODBC 3.x and the SQLSTATE already reported when either parameter is named on its own. (snowflakedb/drivers#1883)
- Changed rejection of a Workload Identity Federation parameter (`WORKLOAD_IDENTITY_PROVIDER`, `WORKLOAD_IDENTITY_ENTRA_RESOURCE`, `WORKLOAD_IDENTITY_IMPERSONATION_PATH`, `WORKLOAD_IDENTITY_AWS_USE_OUTBOUND_TOKEN`) to report SQLSTATE `28000` rather than a general error, so a missing or invalid one is reported as the authentication failure it is. ODBC 3.x reported `28000` for a missing `WORKLOAD_IDENTITY_PROVIDER`. (snowflakedb/drivers#1883)
- Changed the diagnostic message on a failed login to lead with the server's own text: `SQLGetDiagRec` now reads `Failed to login: Login error: <server text>, code: <code>` on its first line, where the server sentence previously appeared only inside the error trace. SQLSTATE and native error code are unchanged. (snowflakedb/drivers#1881)
- Improved log output to mask OAuth client IDs and AWS access-key IDs. (snowflakedb/drivers#1840)

Bug fixes:
- Fixed `SQLPrepare` + `SQLExecute` for `PUT`/`GET` file-transfer commands, which previously failed with server error `000007` (statement not preparable) because prepare issued a `describeOnly` request the server rejects; prepare now skips the describe and the transfer runs on execute. (snowflakedb/drivers#1510)
- Fixed `SQLGetTypeInfo` to return type information matching the application's configured ODBC version. (snowflakedb/drivers#2008)
- Fixed catalog result-set string columns to report `SQL_WVARCHAR` metadata consistently, including after `SQLPrimaryKeys` and `SQLForeignKeys` results are installed on the statement; `SQLStatistics` `ASC_OR_DESC` reports `SQL_WCHAR`. (snowflakedb/drivers#2007)
- Fixed intermittent key-pair `SQLDriverConnect` failures on Windows x64 Azure during JWT login. (snowflakedb/drivers#2073)
- Fixed `SQL_C_DEFAULT` on catalog `SMALLINT` and `INTEGER` columns so `SQLGetTypeInfo` and `SQLColumns` return binary integers instead of failing with SQLSTATE 22003. (snowflakedb/drivers#1896)
- Fixed INTERVAL DAY-TIME result fetch for subtypes whose Arrow scale is not TIME precision (`MINUTE TO SECOND`, `SECOND`, and truncated `DAY TO SECOND` fractions). (snowflakedb/drivers#2002)
- Fixed `SQLColumns` sizes for `GEOGRAPHY` and `GEOMETRY` to follow the session VARCHAR maximum. (snowflakedb/drivers#2009)
- Enforced restrictive file permissions when loading ODBC DSN files on Unix. (snowflakedb/drivers#1845)
- Fixed row-wise fetches with unaligned `SQL_ATTR_ROW_BIND_TYPE` strides so length, indicator, character, wide-character, and fixed-width value writes do not crash. (snowflakedb/drivers#2006)

## v4.0.0-rc3

Breaking changes:

- Changed `SQLColumns` `BUFFER_LENGTH` for `DATE`/`TIME` from `COLUMN_SIZE` (`10` / `18` for `TIME(9)`) to `6` (`sizeof(SQL_DATE_STRUCT)` / `sizeof(SQL_TIME_STRUCT)`); query-result `SQLColAttribute` octet length for DATE/TIME remains 6. (snowflakedb/drivers#1485)

Bug fixes:
- Fixed `SQL_C_CHAR`/`SQL_C_WCHAR` binds to `TIMESTAMP_TZ` columns rejecting ISO8601 strings with a `T` date-time separator (e.g. `"2017-11-30T18:17:05.123456789+08:00"`); the parser now accepts both the space and `T` separator variants. (snowflakedb/drivers#1832)

New features:
- Added `INTERVAL YEAR TO MONTH` and `INTERVAL DAY TO SECOND` result support: `SQL_C_CHAR`/`SQL_C_WCHAR` fetch returns the canonical ANSI literal (`[-]Y-MM`, `[-]D HH:MM:SS[.f]`), same-family `SQL_C_INTERVAL_*` targets receive the parsed interval struct, and scalar numeric targets receive total months or total whole seconds (reporting `01S07` when sub-second precision is dropped). (snowflakedb/drivers#1732)
- Added native AKS Workload Identity support for Azure: when the Azure Workload Identity webhook injects `AZURE_CLIENT_ID`, `AZURE_TENANT_ID`, and `AZURE_FEDERATED_TOKEN_FILE` into a pod and the projected token file exists on disk, `WORKLOAD_IDENTITY_PROVIDER=AZURE` exchanges that federated token for an Entra ID access token directly. `WORKLOAD_IDENTITY_IMPERSONATION_PATH` is not supported in this environment. (snowflakedb/drivers#1367)
- Added a `WORKLOAD_IDENTITY_AWS_USE_OUTBOUND_TOKEN` connection parameter for AWS Workload Identity Federation. When set to true, attestation uses outbound STS `GetWebIdentityToken` instead of the default pre-signed `GetCallerIdentity` token; the connection parameter takes precedence over `SNOWFLAKE_ENABLE_AWS_WIF_OUTBOUND_TOKEN`. (snowflakedb/drivers#1551)
- Added `INCLUDE_RETRY_REASON` (default true) so retried query requests send `retryReason` (the HTTP status that triggered the retry, or `0` for transport failures) alongside `retryCount`. (snowflakedb/drivers#1568)

Changes:

- Changed `SQLBrowseConnect` so an incomplete connection string returns `SQL_NEED_DATA` and keeps the handle available for further browse calls, matching the 3.x iterative protocol under iODBC. (snowflakedb/drivers#1742)
- Changed OAuth Authorization Code connections to default `CLIENT_STORE_TEMPORARY_CREDENTIAL` to true when the caller has not set it, matching ODBC 3.x token caching. (snowflakedb/drivers#1545)
- Changed `SQLForeignKeys` with `SQL_ATTR_METADATA_ID=TRUE` to return `SQL_ERROR` (`HY009`) for a `NULL` catalog, schema, or table pointer on either side. (snowflakedb/drivers#1545)
- Changed `SQL_C_BINARY` fetch of `FLOAT`/`DOUBLE`/`REAL` to return the native 8-byte IEEE 754 value instead of a 19-byte `SQL_NUMERIC_STRUCT`. (snowflakedb/drivers#1664)
- Changed `SQL_BIT` parameter binding so integer and `SQL_C_NUMERIC` sources accept only `0` and `1` (other magnitudes return `22003`). (snowflakedb/drivers#1544)
- Changed `SQL_C_CHAR`/`SQL_C_WCHAR` binding of `"Infinity"`, `"-Infinity"`, and `"NaN"` to `SQL_FLOAT`/`SQL_REAL`/`SQL_DOUBLE` to forward the non-finite value instead of returning `22018`. (snowflakedb/drivers#1544)
- Changed `SQL_C_CHAR`/`SQL_C_WCHAR` hex literals bound to `SQL_BINARY` so an odd-length hex string drops the leftover nibble and succeeds. (snowflakedb/drivers#1544)
- Changed `SQLBindParameter` with Snowflake vendor TIMESTAMP codes (`2000` / `2001` / `2002`) to store those codes on the IPD so `SQLDescribeParam` returns them as bound. (snowflakedb/drivers#1544)
- Changed `SQLSetStmtAttr(SQL_ROWSET_SIZE, 0)` to return `SQL_ERROR` (`HY024`) instead of storing `0` or coercing to `1`. (snowflakedb/drivers#1544)
- Changed PUT result `source_compression` / `target_compression` tokens to lowercase (e.g. `gzip`), matching ODBC 3.x. (snowflakedb/drivers#1543)
- Changed gzip-compressed PUT uploads to omit the original filename from the gzip `FNAME` header, matching ODBC 3.x. (snowflakedb/drivers#1543)
- Changed PUT and GET to transfer several files at once, bounded by the statement `PARALLEL` value; result rows keep their original file order. (snowflakedb/drivers#1451)
- Changed an unreadable or empty `TOKEN_FILE_PATH` to report SQLSTATE `28000` instead of `01S00`. (snowflakedb/drivers#1477)
- Improved GET to warn when a downloaded batch contains multiple files that resolve to the same local filename. (snowflakedb/drivers#1651)
- Improved external-browser callback handling to cap HTTP header size on the localhost listener. (snowflakedb/drivers#1201)

Bug fixes:

- Fixed `SQLGetDiagField` return codes for three edge cases: a record field requested with `RecNumber=0` now returns `SQL_ERROR` instead of `SQL_NO_DATA`, a header field requested with a positive `RecNumber` now returns `SQL_SUCCESS` instead of `SQL_NO_DATA`, and a negative `BufferLength` for a string field now returns `SQL_ERROR`. (snowflakedb/drivers#1878)
- Fixed array/batch parameter binding to retry the execute with inline JSON when the `SYSTEM$BIND` stage is disabled, instead of failing the statement. (snowflakedb/drivers#1824)
- Fixed the file-based token cache changing the mode of a cache file that is not `0600` and then using it anyway; such a file is now reported and left unused. (snowflakedb/drivers#1793)
- Fixed connections failing when `CLIENT_SESSION_KEEP_ALIVE_HEARTBEAT_FREQUENCY` falls outside the accepted range; the value is now clamped before login. (snowflakedb/drivers#1780)
- Fixed a `Driver=`-only connect with no other connection-string attributes to load the default `connections.toml` profile. (snowflakedb/drivers#1221)
- Fixed session-parameter reads used by `SQLGetConnectAttr` and decimal-as-int conversion to honor typed values returned by the server (e.g. `AUTOCOMMIT` after `ALTER SESSION`). (snowflakedb/drivers#1342)
- Fixed queries returning a `FILE` column failing with `Unsupported column type`. (snowflakedb/drivers#1697)
- Fixed queries returning a `MAP` column failing with `Unsupported column type`. (snowflakedb/drivers#1666)

## v4.0.0-rc2

Breaking changes:

- Changed `SQLColumns` `BUFFER_LENGTH` for `NUMBER`/`DECIMAL` to return precision + 2 (ODBC transfer octet length); query-result `SQLColAttribute` octet/display for NUMBER remains 136. (snowflakedb/drivers#1166)
- Changed `SQLColumns` `SQL_DATA_TYPE` for DATE/TIME/TIMESTAMP to the verbose `SQL_DATETIME` (`9`) with the subtype in `SQL_DATETIME_SUB`; `DATA_TYPE` still returns the concise type (`91` / `92` / `93`). (snowflakedb/drivers#1296)
- Changed `SQLColumns` `COLUMN_SIZE` for `TIMESTAMP*` from a fixed `35` to `20` + scale (`19` when scale is `0`). (snowflakedb/drivers#1297)
- Changed `SQLColumns` `BUFFER_LENGTH` for `TIMESTAMP*` from `35` to `16` (`sizeof(SQL_TIMESTAMP_STRUCT)`). (snowflakedb/drivers#1298)
- Changed `SQLColumns` `COLUMN_SIZE` and `BUFFER_LENGTH` for `VARIANT`/`OBJECT`/`ARRAY` to follow `VARCHAR_AND_BINARY_MAX_SIZE_IN_RESULT` instead of SHOW COLUMNS' 128 MB length. (snowflakedb/drivers#1299)

New features:

- Implemented `SQLFreeConnect` (ODBC 2.x) as a thin wrapper around `SQLFreeHandle(SQL_HANDLE_DBC, …)` for direct-link and ODBC 2.x applications that bypass the Driver Manager. (snowflakedb/drivers#1186)
- Implemented `SQLFreeEnv` (ODBC 2.x) as a thin wrapper around `SQLFreeHandle(SQL_HANDLE_ENV, …)` for direct-link and ODBC 2.x applications that bypass the Driver Manager. (snowflakedb/drivers#1187)
- Added a `TOKEN_FILE_PATH` connection parameter that loads a PAT, legacy OAuth, or OIDC bearer token from a file; when both `TOKEN` and `TOKEN_FILE_PATH` are set, the file contents are used. (snowflakedb/drivers#1445)

Changes:

- Changed `SQLCancel` to cancel through the statement's operation handle so the executing path aborts the query on the server; `SQLCancel` returns once the cancel is signaled, and the canceled call still reports `HY008` after the abort is issued. (snowflakedb/drivers#1491)
- Changed query, cancel, and login timeouts to report SQLSTATE `HYT00` when no server SQLSTATE is present. (snowflakedb/drivers#1380)
- Changed `SQL_C_CHAR`/`SQL_C_WCHAR` fetch of `+/-Infinity` `FLOAT`/`REAL` to return `INFINITY` / `-INFINITY` instead of `inf` / `-inf`. (snowflakedb/drivers#1219)
- Changed PUT to resolve relative source paths to an absolute canonical form before upload. (snowflakedb/drivers#666)

Changes:

- Changed `TIMESTAMP_TZ` Arrow fetch to reject a timezone offset outside the valid biased range 0 through 2880. (snowflakedb/drivers#1484)
- Changed `TIMESTAMP_TZ` Arrow fetch to reject a flat Int64 physical type instead of decoding it as UTC with offset 0. (snowflakedb/drivers#1542)

Bug fixes:

- Fixed `SQLColumns` and `SQLProcedureColumns` `CHAR_OCTET_LENGTH` for `VARIANT`/`OBJECT`/`ARRAY` to report the session VARCHAR max (equal to `BUFFER_LENGTH`) instead of NULL. (snowflakedb/drivers#1522)
- Fixed `SQLColumns` `COLUMN_SIZE` and `BUFFER_LENGTH` for unrecognized Snowflake types such as `GEOGRAPHY`/`GEOMETRY` to report the varchar metrics implied by their `SQL_VARCHAR` `DATA_TYPE` instead of NULL. (snowflakedb/drivers#1431)
- Fixed `SQLProcedureColumns` `TYPE_NAME` for unsupported types such as `GEOGRAPHY`/`GEOMETRY` to report the Snowflake type name while `DATA_TYPE` remains `SQL_VARCHAR`. (snowflakedb/drivers#1432)
- Fixed `SQLColumns` and `SQLProcedureColumns` `CHAR_OCTET_LENGTH` for unsupported types such as `GEOGRAPHY`/`GEOMETRY` to report a byte length instead of NULL, matching the `SQL_VARCHAR` they report as `DATA_TYPE`. (snowflakedb/drivers#1432)
- Fixed `SQLGetTypeInfo` string result columns (`TYPE_NAME`, `LITERAL_PREFIX`/`SUFFIX`, `CREATE_PARAMS`, `LOCAL_TYPE_NAME`) to report `SQL_WVARCHAR` as the IRD concise type, matching `SQLTables`/`SQLColumns`. (snowflakedb/drivers#1481)
- Fixed `SQLGetTypeInfo` `INTERVAL_PRECISION` to report `SQL_SMALLINT` as the IRD concise type, matching the ODBC spec and the reference driver; `NUM_PREC_RADIX` remains `SQL_INTEGER`. (snowflakedb/drivers#1482)
- Fixed reauth-required authentication failures to report SQLSTATE `08001` instead of `28000`. (snowflakedb/drivers#1133)
- Fixed an empty `ACCOUNT` value hanging until login timed out; it is now rejected immediately. (snowflakedb/drivers#1514)
- Fixed cancelling a PUT or GET to abort the in-flight cloud transfer and remove any partial `.part` download file. (snowflakedb/drivers#1424)
- Fixed a client-side query timeout to abort the query on the server instead of giving up locally and leaving it running. (snowflakedb/drivers#1491)
- Fixed JSON-format decode of timestamps just before the Unix epoch so the fractional second does not shift the instant forward by one second. (snowflakedb/drivers#1498)
- Fixed PUT with overwrite disabled to skip an existing stage object instead of replacing it. (snowflakedb/drivers#1434)
- Fixed `SQLColumns` and `SQLProcedureColumns` `BUFFER_LENGTH` and `CHAR_OCTET_LENGTH` for `VARCHAR`/`TEXT` to report Snowflake `byteLength`, falling back to 4× `COLUMN_SIZE` capped at the session VARCHAR byte maximum where no `byteLength` is available; query-result `SQLColAttribute` octet length is unchanged. (snowflakedb/drivers#1503)

## v4.0.0-rc1

Breaking changes:

- Changed `SQL_C_CHAR` conversion of DECIMAL/NUMERIC to return `SQL_ERROR` (`22003`) when whole digits do not fit in the buffer; previously returned `SQL_SUCCESS_WITH_INFO` with truncated digits. (snowflakedb/drivers#1270)
- Changed `SQL_C_BINARY` conversion of DECIMAL/NUMERIC/DECFLOAT to return `SQL_ERROR` (`22003`) when the buffer is smaller than `sizeof(SQL_NUMERIC_STRUCT)`; previously `BufferLength` was ignored, risking buffer overflows. (snowflakedb/drivers#833)
- Changed conversion of NaN `FLOAT`/`DOUBLE` to integer and `SQL_C_BIT` targets to return `SQL_ERROR` (`22003`); previously returned `SQL_SUCCESS_WITH_INFO` with a silent `0`. (snowflakedb/drivers#807)
- Enforced interval leading-field precision for `SQL_C_INTERVAL_*` types: values that exceed the default precision of 2 now return `SQL_ERROR` (`22015`), and `SQL_DESC_DATETIME_INTERVAL_PRECISION` is respected; previously neither was enforced. (snowflakedb/drivers#784)
- Changed `SQLRowCount` with a `NULL` `RowCountPtr` to return `SQL_ERROR` (`HY009`); previously returned `SQL_SUCCESS`. (snowflakedb/drivers#1269)
- Changed `SQLBindParameter` to return `SQL_ERROR` (`HY104`) for a negative `DecimalDigits` value; previously accepted negative scale silently. (snowflakedb/drivers#1269)
- Changed conversion of DECFLOAT values with extreme exponents to `SQL_C_BINARY` to return `SQL_ERROR` (`22003`) instead of silently clamping. (snowflakedb/drivers#496)
- Changed `SQL_C_NUMERIC` to `VARCHAR` conversion to apply the scale from `SQL_NUMERIC_STRUCT` (e.g. magnitude `12345` with scale `2` produces `"123.45"`); previously scale was ignored. (snowflakedb/drivers#566)
- Changed binding of subnormal double values near `DBL_MIN` to preserve the value; previously stored as `0.0`. (snowflakedb/drivers#305)
- Changed `TIME` to `SQL_C_CHAR`/`SQL_C_WCHAR` buffer-too-small handling: a buffer that cannot hold the base time (`HH:MM:SS`) now returns `SQL_ERROR` (`22003`); a buffer that truncates only fractional seconds returns `SQL_SUCCESS_WITH_INFO` (`01004`). (snowflakedb/drivers#844)
- Changed `DATE` to `SQL_C_BINARY` conversion with an undersized buffer to return `SQL_ERROR` (`22003`); previously returned `SQL_SUCCESS` with truncated data. (snowflakedb/drivers#844)
- Changed `SQLGetFunctions` to report `SQL_API_SQLCANCELHANDLE` as supported; previously returned `SQL_FALSE` despite the function being exported. (snowflakedb/drivers#871)
- Tightened conversion from VARCHAR to `SQL_C_INTERVAL_*` types. Truncation returns `SQL_SUCCESS_WITH_INFO` (`01S07`) and interval-field overflow returns `SQL_ERROR` (`22015`). (snowflakedb/drivers#1110)
- Changed `SQL_C_INTERVAL_SECOND` with fractional seconds bound to exact-numeric SQL types to truncate the fraction and succeed; previously returned `SQL_ERROR` (`22015`). (snowflakedb/drivers#1067)
- Changed `SQLCancel` during data-at-execution to discard all accumulated `SQLPutData` data so a re-entered sequence starts fresh; previously accumulated data from an interrupted sequence could be concatenated into the next sequence. (snowflakedb/drivers#990)
- Changed `SQLForeignKeys` to resolve a `NULL` FK catalog and schema from the connection context; previously returned an empty result set unless `CLIENT_METADATA_REQUEST_USE_CONNECTION_CTX` was enabled. (snowflakedb/drivers#456)
- Changed `SQLSetConnectAttr(SQL_ATTR_LOGIN_TIMEOUT)` called after connect to return `SQL_ERROR` (`HY011`); previously returned `SQL_SUCCESS` as a silent no-op. (snowflakedb/drivers#1269)
- Changed `SQLSetStmtAttr(SQL_ATTR_CURSOR_TYPE)` for unsupported cursor types to substitute `SQL_CURSOR_FORWARD_ONLY` and return `SQL_SUCCESS_WITH_INFO` (`01S02`); previously accepted any value silently. (snowflakedb/drivers#1269)
- Changed the ODBC diagnostic vendor prefix from `[Snowflake][Support]` to `[Snowflake][Snowflake ODBC Driver]`, matching the ODBC spec format `[vendor][ODBC-component-identifier]`. (snowflakedb/drivers#1075)
- Changed `SQLGetInfo(SQL_DRIVER_NAME)` to return the loaded driver library's file name (e.g. `libsfodbc.so`, `libsfodbc.dylib`, `sfodbc.dll`) as the ODBC specification prescribes; previously returned the fixed string `Snowflake`. (snowflakedb/drivers#1076)
- Changed `SQLCancelHandle(SQL_HANDLE_DBC)` to return `SQL_ERROR` (`HY010`) when an associated statement is asynchronously executing or mid data-at-execution; previously returned `SQL_SUCCESS` as a no-op. (snowflakedb/drivers#871)
- Changed DECFLOAT fetched as `SQL_C_CHAR`/`SQL_C_WCHAR` to return normalized scientific notation (e.g. `1.2e200` instead of `12e199`). (snowflakedb/drivers#496)
- Changed stage array-binding threshold comparison from `>` to `>=` and added support for the `arrayBindSupported` and `CLIENT_STAGE_ARRAY_BINDING_THRESHOLD` session parameters; previously the server-provided `arrayBindSupported` flag was ignored. (snowflakedb/drivers#148)
- Removed support for the `SQL_SF_CONN_ATTR_PRIV_KEY` (raw `EVP_PKEY*`) connection attribute; use `PRIV_KEY_CONTENT`, `PRIV_KEY_BASE64`, or `PRIV_KEY_FILE` instead. (snowflakedb/drivers#277)
- Changed OAuth token and authorization endpoint URL validation to require HTTPS; `http://` is allowed only for loopback addresses. (snowflakedb/drivers#346)
- Changed the SQLSTATE for OAuth IdP token-exchange rejection from `HY000` to `28000` (invalid authorization specification). (snowflakedb/drivers#1217)
- Changed connection setup to reject WIF-only parameters (e.g. `WORKLOAD_IDENTITY_PROVIDER`) unless `AUTHENTICATOR=WORKLOAD_IDENTITY`; previously silently ignored them with any other authenticator. (snowflakedb/drivers#770)
- Changed `WORKLOAD_IDENTITY_IMPERSONATION_PATH` with `IDENTITY_PROVIDER=OIDC` to return a connection error; previously silently ignored the combination. (snowflakedb/drivers#821)
- Changed catalog `REMARKS` and `SQLColumns` `COLUMN_DEF` to return `SQL_NULL_DATA` when absent instead of an empty string, and `SQLTables` `REMARKS` now surfaces a `SHOW OBJECTS` comment when present. (snowflakedb/drivers#1148)
- Changed `SQL_SF_STMT_ATTR_LAST_QUERY_ID` to be read-only and fully populated after every statement execution; changed `SQL_SF_STMT_ATTR_MULTI_STATEMENT_COUNT` to support get/set with a default of `-1` (auto-detect); neither attribute was fully implemented in ODBC 3.x. (snowflakedb/drivers#617)
- Changed `SQLGetDiagRec` / `SQLGetDiagField` message text to append an internal error trace by default; set `ErrorTraceEnabled=false` in `sf.odbc.ini` to restore 3.x-shaped message text. (snowflakedb/drivers#1082)
- Fixed return codes and diagnostics under iODBC to match the ODBC specification: zero-row DML returns `SQL_NO_DATA`, length-only queries return `SQL_SUCCESS`, and required `SUCCESS_WITH_INFO` / `HY012` diagnostics are posted correctly. (snowflakedb/drivers#1107)
- Fixed orphaned child statement handles under iODBC: `SQLDisconnect` and `SQLFreeStmt(SQL_DROP)` now invalidate all associated handles through iODBC's alloc table so a subsequent `SQLFreeHandle` on an orphaned handle returns `SQL_INVALID_HANDLE` instead of `SQL_SUCCESS`. (snowflakedb/drivers#1107)
- Fixed `SQLGetDescField` and `SQLSetDescField` called during `SQL_NEED_DATA` state under iODBC to return `SQL_ERROR` (`HY010`); previously returned `SQL_SUCCESS`. (snowflakedb/drivers#1107)
- Fixed ODBC 3.x-standard SQLSTATEs under iODBC (e.g. `HY090`, `HY010`, `HY017`, `HY092`, `07009`, `HY008`); previously returned vendor `HY000` or ODBC 2.x aliases. (snowflakedb/drivers#1269)
- Fixed `SQL_C_WCHAR` fetch encoding under iODBC to use a uniform `UTF-32` width driven by `DriverManagerEncoding` in `sf.odbc.ini`; previously the code-unit width varied by Snowflake source column type. (snowflakedb/drivers#1107)
- Changed proxy resolution so `HTTP_PROXY` / `HTTPS_PROXY` / `NO_PROXY` environment variables are ignored unless `USE_PROXY_ENV=true` (alias `PROXYWITHENV`); explicit `PROXY=` / `NO_PROXY=` connection parameters still apply. ODBC 3.x fell back to those environment variables when no DSN/`simba.snowflake.ini` proxy was set. `PROXYWITHENV` no longer writes the connection's `PROXY` value into the process environment. (snowflakedb/drivers#1223)
- Changed `SF_GLOBAL_SSL_VERSION` (and TLS settings) below TLS 1.2 to fail the connection; previously logged a deprecation warning and still negotiated. (snowflakedb/drivers#411)
- Removed support of OCSP certificate validation in favor of CRL validation.

New features:

- Added VECTOR column fetch: VECTOR columns are returned as compact JSON array strings (e.g. `[1,2,3]`) via `SQL_C_CHAR`, `SQL_C_WCHAR`, and `SQL_C_BINARY`; `SQLDescribeCol` reports `SQL_VARCHAR` and `SQL_DESC_TYPE_NAME` returns `VECTOR`. (snowflakedb/drivers#1010)
- Added `ALTER USER ADD PROGRAMMATIC ACCESS TOKEN` result-set support: the generated token is returned as a result row; ODBC 3.x returned an invalid cursor state for this DDL. (snowflakedb/drivers#33)
- Implemented `SQLTransact` (ODBC 2.x) as a mapping to `SQLEndTran`, restoring compatibility with ODBC 2.x applications that bypass the Driver Manager. (snowflakedb/drivers#677)
- Added `TIME` to `SQL_C_BINARY` conversion support. (snowflakedb/drivers#844)
- Added `SQL_C_BINARY` as `SQLBindParameter` source type for `SQL_DECIMAL`, `SQL_NUMERIC`, `SQL_TIME`, and `SQL_TIMESTAMP` when the buffer length matches the SQL data length. (snowflakedb/drivers#833)
- Added PUT local-path tilde expansion: a leading `~/` or `~` in the source path is expanded to the user's home directory. (snowflakedb/drivers#376)
- Added automatic creation of a missing destination directory for `GET` operations. (snowflakedb/drivers#405)
- Added `SQLGetDiagField(SQL_DIAG_SERVER_NAME)` population: returns the connected server name instead of an empty string. (snowflakedb/drivers#303)
- Added `SQLGetDiagField(SQL_DIAG_DYNAMIC_FUNCTION)` / `SQL_DIAG_DYNAMIC_FUNCTION_CODE` population after statement execution (e.g. SELECT → `85` / `SQL_DIAG_SELECT_CURSOR`); previously returned empty / `0`. (snowflakedb/drivers#743)
- Added `SQL_C_BINARY` to `VARCHAR` binding: binary bytes are hex-encoded before send (e.g. `0xDEADBEEF` → `"deadbeef"`); ODBC 3.x forwarded raw bytes and the server rejected the request with HTTP 400. (snowflakedb/drivers#749)
- Added `SQL_SF_TIMESTAMP_TZ` parameter binding for `SQL_C_TYPE_TIMESTAMP` (stored at UTC) and `SQL_C_CHAR`/`SQL_C_WCHAR` strings with a `+/-HH:MM` offset. (snowflakedb/drivers#1005)
- Added `SQLBindParameter` support for binding to `SQL_INTERVAL_*` parameters from same-family `SQL_C_INTERVAL_*`, `SQL_C_CHAR`/`SQL_C_WCHAR`, and exact-numeric C sources (approximate-numeric sources are rejected with `SQL_ERROR` `07006`). (snowflakedb/drivers#1067)
- Added `SQL_SF_CONN_ATTR_APPLICATION` as a pre-connect get/set attribute that forwards the application name to the server. (snowflakedb/drivers#412)
- Added `SQLGetFunctions` reporting of `SQL_API_SQLSETSCROLLOPTIONS` and `SQL_API_SQLPARAMOPTIONS` as supported under iODBC. (snowflakedb/drivers#888)

Changes:

- Renamed the default driver registration name to `Snowflake ODBC`; update DSN `Driver=` entries that reference `SnowflakeDSIIDriver` (custom names still configurable via `DRIVER_NAME=` / `SF_DRIVER_NAME`). (snowflakedb/drivers#1143)
- Changed distributable ODBC package filenames to `snowflake-odbc-<version>.<arch>.<extension>` with unified architectures `aarch64` / `x86_64` / `x86_32` / `universal`. (snowflakedb/drivers#1181)
- Changed `SQLGetInfo(SQL_DRIVER_VER)` to return the zero-padded `MM.mm.bbbb` format (e.g. `04.00.0000`) per the ODBC specification instead of the unpadded semver string (e.g. `3.16.0`). (snowflakedb/drivers#1074)
- Changed process-wide driver configuration to use `sf.odbc.ini` (searched in `$SF_ODBC_INI`, `~/.config/snowflake/sf.odbc.ini`, `~/.snowflake/sf.odbc.ini`, and `/opt/snowflake/snowflakeodbc/sf.odbc.ini`); update deployment scripts that write `simba.snowflake.ini`. (snowflakedb/drivers#1260)
- Changed `PASSCODEINPASSWORD` connection parameter to accept `true` and `1` in addition to `on`. (snowflakedb/drivers#502)
- Changed `QUERY_TAG` connection parameter to be applied as a server session parameter at connection time. (snowflakedb/drivers#679)
- Changed the on-disk credential cache file from `credential_cache_v1.json` to `credential_cache_v2.json` (same lookup order: `$SF_TEMPORARY_CREDENTIAL_CACHE_DIR`, `$XDG_CACHE_HOME/snowflake`, `$HOME/.cache/snowflake`). Tokens cached by ODBC 3.x are not read by 4.x, so expect one extra authentication after upgrading. (snowflakedb/drivers#343)
- Changed GET downloads on Unix to create files with owner-only (`0600`) permissions by default; set `UNSAFE_FILE_WRITE=true` to use the process umask. (snowflakedb/drivers#1332)
- Changed `PRIV_KEY_FILE` / `private_key_file` reads on Unix to require owner-only permissions (mode `0600`); overly permissive key files now fail unless `UNSAFE_SKIP_CONFIG_FILE_PERMISSIONS_CHECK=true`. (snowflakedb/drivers#1055)
- Changed `AUTHENTICATOR=WORKLOAD_IDENTITY` to reject hosts outside the recognized Snowflake suffixes (`snowflakecomputing.com` / `.cn` / `.mil`) before fetching cloud credentials; extend the list only via the `SNOWFLAKE_WIF_ALLOWED_HOST_SUFFIXES` environment variable. (snowflakedb/drivers#938)
- Changed client-side-encryption `KeyWrappingMetadata.EncryptionLibrary` metadata to `"Rust(OpenSSL)"`. (snowflakedb/drivers#797)
- Changed client-side rejection of invalid `ACCOUNT` / `SERVER` / `PORT` characters to SQLSTATE `01S00` (native error `0`); ODBC 3.x returned `28000` (native error `20032`). (snowflakedb/drivers#349)
- Changed `CLIENT_SESSION_KEEP_ALIVE_HEARTBEAT_FREQUENCY` to be sent as a server session parameter during login (queryable via `SHOW PARAMETERS`) in addition to configuring the client heartbeat interval; ODBC 3.x used it client-side only. (snowflakedb/drivers#1203)
- Changed control-plane response body reads with a cap of 20MB (OAuth, browser, GCP metadata, CRL)
- Changed strict owner-only permissions and single-open I/O in CRL cache handling

Bug fixes:

- Fixed failed `GET` downloads to leave no partial or corrupt file at the destination; the driver now writes to a `.part` temporary file and renames it on successful completion. (snowflakedb/drivers#812)
- Fixed concurrent `SQLDisconnect` to be thread-safe. (snowflakedb/drivers#1018)
- Fixed `SQLDisconnect` to free child statement handles and explicitly allocated descriptors after a successful disconnect so a later `SQLFreeHandle` on those handles returns `SQL_INVALID_HANDLE`. (snowflakedb/drivers#1174)
- Fixed `SQLDisconnect` to return `SQL_ERROR` (`HY010`) without disconnecting when a child statement is asynchronously executing or mid data-at-execution. (snowflakedb/drivers#1174)
- Fixed `SQLCancel` on a statement with `SQL_ATTR_ASYNC_ENABLE` to actually interrupt the in-progress operation: subsequent polling returns `SQL_ERROR` (`HY008`) once the cancellation is acknowledged. Previously polling kept returning `SQL_STILL_EXECUTING` until the query completed naturally. (snowflakedb/drivers#629)
- Fixed cross-thread `SQLCancel` and `SQLCancelHandle` to always return `SQL_SUCCESS` and post no diagnostics of their own, per the ODBC specification; previously they intermittently returned `SQL_ERROR` (`HY008`, native error 604) when the server-side abort raced the query's completion. Only the canceled function returns `HY008`. (snowflakedb/drivers#629)
- Fixed FLOAT/DOUBLE boundary values (`FLT_MAX`, `DBL_MAX`) being incorrectly rejected with a numeric out-of-range error; they are now returned successfully. (snowflakedb/drivers#305)
- Fixed a crash during `TIMESTAMP` to `SQL_C_CHAR`/`SQL_C_WCHAR` conversion when the destination buffer was too small. (snowflakedb/drivers#890)
- Fixed `SQLNumResultCols` and `SQLDescribeCol` to return the correct column count and metadata after a prepared statement's cursor is closed - including the async `SQLPrepare` + `SQLExecute` + `SQLCloseCursor` path - instead of returning incorrect results or errors. (snowflakedb/drivers#475)
- Fixed `SQLDescribeCol` column size for `SQL_DOUBLE` / `SQL_FLOAT` to report `15` (decimal digit precision) instead of `53` (binary mantissa bits). (snowflakedb/drivers#1146)
- Fixed `SQLColumns` `NUM_PREC_RADIX` for FLOAT/DOUBLE/REAL to return `10`, matching decimal `COLUMN_SIZE` (query-result `SQLColAttribute` radix for DOUBLE remains `2`). (snowflakedb/drivers#1146)
- Fixed `SQLFreeHandle(SQL_HANDLE_DESC)` on an implicitly allocated descriptor to return `SQL_ERROR` (`HY017`) and leave the handle valid. (snowflakedb/drivers#1165)
- Fixed external-browser SSO/OAuth on WSL/Linux to validate the browser URL before launching the browser. (snowflakedb/drivers#502)
- Fixed `SQL_C_CHAR` fetch of `FLOAT`/`DOUBLE`/`REAL` with a truncated buffer to post SQLSTATE `01004`. (snowflakedb/drivers#305)
- Fixed `SQL_C_WCHAR` chunked `SQLGetData` buffer capacity to use `sizeof(SQLWCHAR)` so an 8-byte buffer fits three data characters plus NUL. (snowflakedb/drivers#1292)
- Fixed `SQL_C_WCHAR` decimal-string binding to `SQL_DECIMAL` to convert correctly on all platforms. (snowflakedb/drivers#303)
- Fixed crashes under iODBC during some `SQLFreeHandle` / `SQLDisconnect` / `SQLCopyDesc` handle-hierarchy sequences that aborted or segfaulted. (snowflakedb/drivers#1018)
- Fixed connection-attribute state after `SQLDisconnect` under iODBC: `SQL_ATTR_CONNECTION_DEAD` reports `SQL_CD_TRUE` and reads of other connection attributes return `SQL_ERROR` instead of stale cached values. (snowflakedb/drivers#1018)
- Fixed `SQLGetData` conversion from `DECFLOAT` to `SQL_C_WCHAR` under iODBC to convert the value instead of returning `SQL_SUCCESS` with `SQL_NULL_DATA` and an untouched buffer. (snowflakedb/drivers#1107)
- Fixed `SQLColAttribute` to map ODBC 2.x field identifiers (`SQL_COLUMN_NAME`, `SQL_COLUMN_TYPE`, etc.) to their `SQL_DESC_*` equivalents as previous behavior was inconsistent (empty strings, wrong values, or `HY091`). (snowflakedb/drivers#34)
- Fixed `SQLPrimaryKeys` / `SQLForeignKeys` with `SQL_ATTR_METADATA_ID=TRUE` to case-fold unquoted identifiers to uppercase before matching instead of comparing case-sensitively and returning empty results for lowercase names. (snowflakedb/drivers#455)
- Fixed `SQLProcedures` / `SQLProcedureColumns` with `SQL_ATTR_METADATA_ID=TRUE` to case-fold unquoted identifiers to uppercase before matching instead of returning empty results for lowercase names. (snowflakedb/drivers#534)
- Fixed `SQLTables` with `SQL_ATTR_METADATA_ID=TRUE` to case-fold unquoted identifiers to uppercase before matching instead of returning empty results for lowercase names. (snowflakedb/drivers#1006)
- Fixed `SQLGetData` with `SQL_C_NUMERIC` to honor `SQL_DESC_PRECISION` and `SQL_DESC_SCALE` set on the ARD via `SQLSetDescField`instead of hardcoded precision `38` / scale `0`. (snowflakedb/drivers#303)
- Fixed `DATE` to `SQL_C_CHAR`/`SQL_C_WCHAR` conversion with an undersized buffer to return `SQL_ERROR` (`22003`) instead of truncating. (snowflakedb/drivers#844)
- Fixed `SQLGetDescField` on an empty IPD (no parameters bound) to return `SQL_NO_DATA` instead of `SQL_ERROR`. (snowflakedb/drivers#566)
- Fixed `SQLGetStmtAttr` / `SQLGetConnectAttr` with a negative string `BufferLength` to return `SQL_ERROR` (`HY090`). (snowflakedb/drivers#1269)
- Fixed `SQLGetConnectAttr` with an out-of-range attribute identifier to return `SQL_ERROR` (`HY092`). (snowflakedb/drivers#412)
- Fixed `SQLSetConnectAttr` with ODBC 2.x statement-level attribute IDs (`SQL_ATTR_MAX_ROWS`, `SQL_ATTR_QUERY_TIMEOUT`) to return `SQL_ERROR` (`HY092`) instead of `SQL_SUCCESS` as a silent no-op. (snowflakedb/drivers#751)
- Fixed fractional truncation on a numeric-to-character fetch to return `SQL_SUCCESS_WITH_INFO` with SQLSTATE `01S07`. (snowflakedb/drivers#270)
- Fixed `FLOAT`/`REAL` to single-field interval fetch with a nonzero fractional part to return `SQL_SUCCESS_WITH_INFO` (`01S07`). (snowflakedb/drivers#784)
- Fixed numeric-to-interval conversion so a value that truncates to zero always yields `+0` (no negative-zero interval sign). (snowflakedb/drivers#784)

Known issues:

- Binding `"Infinity"`, `"-Infinity"`, or `"NaN"` as `SQL_C_CHAR` / `SQL_C_WCHAR` to a float SQL type returns `SQL_ERROR` (`22018`); ODBC 3.x forwarded the literal and stored a non-finite `FLOAT`. (snowflakedb/drivers#901)
- `SQLForeignKeys` with `SQL_ATTR_METADATA_ID=TRUE` allows unused FK-side `NULL` pointers instead of returning `HY009`; ODBC 3.x rejected the call with `SQL_ERROR`. (snowflakedb/drivers#456)
- Conversion from `FLOAT` / `DOUBLE` / `REAL` to `SQL_C_BINARY` returns a 19-byte `SQL_NUMERIC_STRUCT`; ODBC 3.x returned the raw 8-byte IEEE 754 double. (snowflakedb/drivers#833)
- Any nonzero `SQL_C_NUMERIC` value bound to `SQL_BIT` converts to `true`; ODBC 3.x rejected values outside `{0, 1}` with `SQL_ERROR` (`22003`). (snowflakedb/drivers#807)
- `SQL_C_CHAR` / `SQL_C_WCHAR` hex literals bound to `SQL_BINARY` are hex-decoded (e.g. `"DEADBEEF"` → 4 bytes); ODBC 3.x forwarded the raw ASCII bytes. (snowflakedb/drivers#1054)
- PUT compression type is returned in uppercase (e.g. `GZIP`); ODBC 3.x returned lowercase (e.g. `gzip`). (snowflakedb/drivers#958)
- Gzip-compressed PUT uploads omit the original filename from the gzip `FNAME` header and zero the mtime; ODBC 3.x included the filename in the header. (snowflakedb/drivers#1201)
- OAuth Authorization Code token caching is disabled by default; set `CLIENT_STORE_TEMPORARY_CREDENTIAL=true` to re-enable. ODBC 3.x cached by default on all platforms. (snowflakedb/drivers#1217)
- `SQLDescribeParam` normalizes Snowflake vendor TIMESTAMP type codes (`2000` / `2001` / `2002`) to `SQL_TYPE_TIMESTAMP` (`93`); ODBC 3.x preserved the vendor codes. (snowflakedb/drivers#259)
- Under iODBC, the driver manager may return different return codes or SQLSTATEs for identical calls because ODBC 4.x and ODBC 3.x advertise different driver capabilities (e.g. DSN-name lookup, catalog calls, `SQLGetFunctions` probes). (snowflakedb/drivers#1107)
- `SQLBrowseConnect` under iODBC returns `SQL_ERROR` for partial connection input instead of `SQL_NEED_DATA` for the iterative protocol. (snowflakedb/drivers#1107)
