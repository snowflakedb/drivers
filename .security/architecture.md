> Derived-from: snowflake-eng/drivers@626637efa4f9ed2168b572d84cfea24009332993 · generated 2026-08-19 · sources: snowflake-eng/drivers@626637efa4f9ed2168b572d84cfea24009332993
> Regenerate when: a new internet-facing endpoint is added, an FFI bridge crate is added or its consumption pattern changes, or the query/stage data-flow path changes

## What is internet-facing

**Primary Snowflake account host.** Derived from the `account`/`host` connection
parameter — all query and session traffic goes here.

- `sf_core/src/config/connection_config.rs#is_allowed_account_char` — an allowlist
  (`c.is_ascii_alphanumeric() || matches!(c, '.' | '-' | '_')`) preventing
  host-header-injection via a malicious `account` string.
- `sf_core/src/config/resolver.rs#derive_host_from_account` — `host =
  format!("{account}.snowflakecomputing.com")`.
- `sf_core/src/config/connection_config.rs#derive_server_url` — explicit
  `server_url` wins; else `{protocol}://{host}[:{port}]`.
- Endpoints on this host, all in `sf_core/src/rest/snowflake/mod.rs` (login
  path, query path, abort path, session-refresh path, monitoring path).

**In-band telemetry** — same account host, `/telemetry/send`, not a separate
ingestion endpoint. `sf_core/src/telemetry/snowflake_exporter.rs#ExporterSession`
reuses the same `server_url` and session-token type as ordinary query traffic
(`#send_with_token`).

**CRL (revocation) responder endpoints** — discovered per-certificate, never
statically configured. `sf_core/src/crl/certificate_parser.rs#extract_crl_distribution_points`
pulls CRL URLs from the X.509 CRL Distribution Points extension of each peer
cert; fetched by `sf_core/src/crl/validator.rs#fetch_crl_with_cache`. This
driver does not support OCSP: `sf_core/src/tls/revocation.rs`'s `RevocationError`
enum has no OCSP variant, and revocation checking is CRL-only.

**Identity-provider endpoints — three flows, three discovery mechanisms.**

- *External Browser SSO*: the SSO URL is fetched FROM the account host first
  (`sf_core/src/rest/snowflake/external_browser.rs#request_authenticator`,
  `POST /session/authenticator-request`), then opened in the system browser via
  `sf_core/src/rest/snowflake/browser.rs#open_url`.
- *Native Okta*: the IdP URL is directly user-supplied via the `authenticator`
  connection parameter — `sf_core/src/config/rest_parameters.rs#NativeOktaConfig`.
- *OAuth* (Authorization Code / Client Credentials): defaults to Snowflake-as-IdP;
  `authorization_url`/`token_url` are optional explicit overrides —
  `sf_core/src/config/rest_parameters.rs#OAuthAuthorizationCodeConfig`.
- *Password/PAT*: no IdP step — credentials are sent directly to the account
  host's `/session/v1/login-request`
  (`sf_core/src/rest/snowflake/mod.rs#send_login_request`);
  `sf_core/src/auth.rs#create_credentials` maps them straight through with no
  discovery round-trip.

**Workload Identity Federation (WIF)** — cloud-provider metadata/IdP endpoints,
gated by a Snowflake-host allowlist checked before any cloud credential is
fetched. `sf_core/src/rest/snowflake/workload_identity/mod.rs#ensure_allowed_host`
runs before `#create_attestation` dispatches to a provider
(`sf_core/src/rest/snowflake/mod.rs#auth_request_data`). Endpoints reached only once the
allowlist passes: AWS/Azure IMDS (`169.254.169.254`), GCP metadata
(`metadata.google.internal`), AWS STS, `login.microsoftonline.com`,
`iamcredentials.googleapis.com` —
`sf_core/src/rest/snowflake/workload_identity/mod.rs#AttestationEndpoints`.

## Major components and data flow

**Core module map.** `sf_core/src/lib.rs` declares every top-level module once:
`apis`, `auth`, `c_api`, `chunks`, `config`, `crl`, `file_manager`, `fs_adapter`,
`handle_manager`, `http`, `logging`, `query_types`, `rest` (`sf_core/src/rest/mod.rs`
is just `pub mod snowflake;` — the REST client only ever talks Snowflake protocol),
`sensitive`, `stage_binding`, `telemetry`, `tls`, `token_cache`.

**FFI boundary — one shared Rust core, one process-wide state, four distinct
consumption patterns.**

`sf_core/src/protobuf/c_api.rs#CApiState` is a single process-wide state
(`static STATE: OnceLock<CApiState>`) built once by `#init_core_state`, using a
single-worker-thread tokio runtime shared by every language wrapper.
`#sf_core_api_call_proto` wraps dispatch in `std::panic::catch_unwind` so a Rust
panic becomes a transport-error response rather than unwinding across the FFI
boundary.

All four bridge crates depend on `sf_core` as an ordinary Rust path dependency,
compiled into each cdylib at build time — but reach it differently:

1. **Node.js** (`nodejs_bridge`) — direct native struct/method calls into
   `sf_core::apis::database_driver_v1::DatabaseDriverV1`, bypassing protobuf
   serialization for this hop entirely.
2. **Python** (`python_bridge`) and **JDBC** (`jdbc_bridge`) — each embeds
   `sf_core::protobuf::apis::RustTransport` (the same dispatcher
   `protobuf::c_api::init_core_state` uses internally) and drives it in-process,
   without going through the exported `sf_core_api_call_proto` C symbol.
3. **ODBC** (`odbc`, cdylib `sfodbc`) — implements the ODBC standard's own C ABI
   (`odbc/src/c_api.rs`, functions like `SQLAllocEnv`), a different C surface
   than `sf_core_api_call_proto`, dictated by the ODBC spec via `odbc-sys`.
4. **.NET** — the one consumer of the *generic* protobuf C ABI via P/Invoke; has
   no Rust bridge crate of its own.
   `dotnet/src/Snowflake.Data/Interop/TfmDependent/SfCoreNativeMethods.cs`
   declares native bindings matching
   `sf_core_api_call_proto`/`sf_core_api_call_proto_async`/`sf_core_free_buffer`/
   `sf_core_api_cancel`/`sf_core_init`.

**Query-request flow.**

1. The calling app invokes execute-query on its language wrapper's own API
   surface, marshaled per the pattern above into
   `sf_core::apis::database_driver_v1`.
2. `sf_core/src/apis/database_driver_v1/query.rs` calls `rest::snowflake`:
   `#execute_sync_query` builds the request URL from `server_url` +
   `QUERY_REQUEST_PATH`, using the session token from
   `sf_core/src/rest/snowflake/mod.rs#snowflake_login` or refreshed via `#refresh_session`.
   Tokens are held as `SensitiveString` and may persist across runs via
   `token_cache/` (file cache or OS keyring).
3. Snowflake's response carries inline rows or a list of result chunks. Each
   `Chunk` (`sf_core/src/rest/snowflake/query_response.rs#Chunk`) carries its own
   `url` field — a URL distinct from the account host — fetched by
   `sf_core/src/chunks/http_downloader.rs#HttpChunkDownloader::download_chunk`
   over a plain `reqwest::Client`.
4. If the statement is a stage PUT/GET, `query.rs` calls
   `file_manager::upload_files`/`download_files` instead, which encrypts
   client-side (`sf_core/src/file_manager/encryption.rs`) before transferring
   to the cloud-storage backend named in the query response's stage info
   (`s3_transfer.rs`, `azure_transfer.rs`, `gcs_transfer.rs`).
5. Any object handed back across the FFI boundary is tracked as an opaque
   `Handle{id, magic}` (`sf_core/src/handle_manager.rs#Handle`) rather than a raw
   pointer — the wrapper holds the handle, the core validates the magic number
   on every call.

**Not covered by this document:** the protobuf request/response conversion
layer (`sf_core/src/protobuf/apis/database_driver_v1/`); the bodies of
`sf_core/src/apis/database_driver_v1/connection.rs`,
`sf_core/src/apis/database_driver_v1/statement.rs`, and
`sf_core/src/apis/database_driver_v1/result_set.rs`; an exhaustive sweep of
every `ServerCertVerifier` implementation in `sf_core/src/tls/client.rs`.
