//! Canonical Snowflake connection-parameter registry.
//!
//! This crate is the single source of truth for the set of supported
//! configuration parameters (`PARAM_DEFS`), their metadata, aliases, scopes,
//! and default values. It is intentionally free of any `sf_core` dependency so
//! that both `sf_core` (the driver) and `sf_params_codegen` (the wrapper code
//! generator) can consume it without a dependency cycle.
//!
//! Default values are expressed as the [`DefaultValue`] IR rather than a
//! `sf_core` runtime type, keeping this crate std-only. `sf_core` converts a
//! `DefaultValue` into its own `Setting` at the boundary.

use std::collections::{HashMap, HashSet};
use std::fmt;

use std::sync::LazyLock;

/// A parameter's static default value.
///
/// This is the crate-local, `sf_core`-independent representation of a default.
/// It mirrors the shape of `sf_core`'s `Setting` enum but stores only
/// compile-time-constant data (`&'static str` / `&'static [u8]`) so the whole
/// registry remains a `static` with no allocation and no external types.
/// `sf_core` provides `From<DefaultValue> for Setting` to materialize it.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum DefaultValue {
    String(&'static str),
    Bytes(&'static [u8]),
    Int(i64),
    Double(f64),
    Bool(bool),
}

/// A strongly-typed wrapper around a canonical parameter name.
///
/// Provides compile-time safety over bare `&str` keys while remaining
/// zero-cost at runtime (it is `Copy` and stores a `&'static str`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct ParamKey(pub(crate) &'static str);

impl ParamKey {
    /// Wrap a canonical parameter name.
    ///
    /// Intended for callers that already hold a `&'static str` canonical name
    /// from the registry (e.g. `ParamDef::canonical_name`) and need to feed it
    /// back into a `ParamKey`-typed API. Prefer the `param_names` constants for
    /// literal names.
    pub const fn new(name: &'static str) -> ParamKey {
        ParamKey(name)
    }

    pub const fn as_str(&self) -> &'static str {
        self.0
    }
}

impl fmt::Display for ParamKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.0)
    }
}

impl From<ParamKey> for String {
    fn from(key: ParamKey) -> String {
        key.0.to_owned()
    }
}

impl AsRef<str> for ParamKey {
    fn as_ref(&self) -> &str {
        self.0
    }
}

/// Canonical parameter name constants.
///
/// Use these instead of bare string literals when referencing parameter names
/// in production code.  This gives compile-time typo detection and
/// find-all-references support.
pub mod param_names {
    use super::ParamKey;

    pub const ACCOUNT: ParamKey = ParamKey("account");
    pub const HOST: ParamKey = ParamKey("host");
    pub const PORT: ParamKey = ParamKey("port");
    pub const PROTOCOL: ParamKey = ParamKey("protocol");
    pub const SSL: ParamKey = ParamKey("ssl");
    pub const SERVER_URL: ParamKey = ParamKey("server_url");
    pub const PRESERVE_UNDERSCORES_IN_HOSTNAME: ParamKey =
        ParamKey("preserve_underscores_in_hostname");
    pub const USER: ParamKey = ParamKey("user");
    pub const PASSWORD: ParamKey = ParamKey("password");
    pub const AUTHENTICATOR: ParamKey = ParamKey("authenticator");
    pub const PRIVATE_KEY: ParamKey = ParamKey("private_key");
    pub const PRIVATE_KEY_FILE: ParamKey = ParamKey("private_key_file");
    pub const PRIVATE_KEY_PASSWORD: ParamKey = ParamKey("private_key_password");
    pub const TOKEN: ParamKey = ParamKey("token");
    pub const TOKEN_FILE_PATH: ParamKey = ParamKey("token_file_path");
    pub const PASSCODE: ParamKey = ParamKey("passcode");
    pub const PASSCODE_IN_PASSWORD: ParamKey = ParamKey("passcodeInPassword");
    pub const CLIENT_STORE_TEMPORARY_CREDENTIAL: ParamKey =
        ParamKey("client_store_temporary_credential");
    pub const DATABASE: ParamKey = ParamKey("database");
    pub const SCHEMA: ParamKey = ParamKey("schema");
    pub const WAREHOUSE: ParamKey = ParamKey("warehouse");
    pub const ROLE: ParamKey = ParamKey("role");
    pub const SECONDARY_ROLES: ParamKey = ParamKey("secondary_roles");
    pub const CONNECTION_NAME: ParamKey = ParamKey("connection_name");
    pub const CUSTOM_ROOT_STORE_PATH: ParamKey = ParamKey("custom_root_store_path");
    pub const EXTRA_ROOT_STORE_PATH: ParamKey = ParamKey("extra_root_store_path");
    pub const VERIFY_HOSTNAME: ParamKey = ParamKey("verify_hostname");
    pub const VERIFY_CERTIFICATES: ParamKey = ParamKey("verify_certificates");
    pub const TLS_SKIP_VERIFY: ParamKey = ParamKey("tls_skip_verify");
    pub const MIN_TLS_VERSION: ParamKey = ParamKey("min_tls_version");
    pub const MAX_TLS_VERSION: ParamKey = ParamKey("max_tls_version");
    pub const CRL_CHECK_MODE: ParamKey = ParamKey("crl_check_mode");
    pub const CRL_ENABLE_DISK_CACHING: ParamKey = ParamKey("crl_enable_disk_caching");
    pub const CRL_ENABLE_MEMORY_CACHING: ParamKey = ParamKey("crl_enable_memory_caching");
    pub const CRL_CACHE_DIR: ParamKey = ParamKey("crl_cache_dir");
    pub const CRL_ALLOW_CERTIFICATES_WITHOUT_CRL_URL: ParamKey =
        ParamKey("crl_allow_certificates_without_crl_url");
    pub const CRL_MAX_DOWNLOAD_SIZE: ParamKey = ParamKey("crl_max_download_size");
    pub const CRL_VALIDITY_TIME: ParamKey = ParamKey("crl_validity_time");
    pub const CRL_ON_DISK_CACHE_REMOVAL_DELAY: ParamKey =
        ParamKey("crl_on_disk_cache_removal_delay");
    pub const CRL_CACHE_CLEANUP_INTERVAL: ParamKey = ParamKey("crl_cache_cleanup_interval");
    pub const CRL_CACHE_START_CLEANUP: ParamKey = ParamKey("crl_cache_start_cleanup");
    pub const CRL_UNSAFE_SKIP_FILE_PERMISSIONS_CHECK: ParamKey =
        ParamKey("crl_unsafe_skip_file_permissions_check");
    pub const CRL_HTTP_TIMEOUT: ParamKey = ParamKey("crl_http_timeout");
    pub const CRL_CONNECTION_TIMEOUT: ParamKey = ParamKey("crl_connection_timeout");
    pub const ASYNC_EXECUTION: ParamKey = ParamKey("async_execution");
    pub const MULTI_STATEMENT_COUNT: ParamKey = ParamKey("multi_statement_count");
    pub const QUERY_TAG: ParamKey = ParamKey("query_tag");
    pub const SKIP_UPLOAD_ON_CONTENT_MATCH: ParamKey = ParamKey("skip_upload_on_content_match");
    pub const CWD: ParamKey = ParamKey("cwd");
    pub const PUT_FASTFAIL: ParamKey = ParamKey("put_fastfail");
    pub const GET_FASTFAIL: ParamKey = ParamKey("get_fastfail");
    pub const PUT_COMPRESS_LEVEL: ParamKey = ParamKey("put_compress_level");
    pub const PUT_TEMPDIR: ParamKey = ParamKey("put_tempdir");
    pub const AUTHENTICATION_TIMEOUT: ParamKey = ParamKey("authentication_timeout");
    pub const OKTA_USERNAME: ParamKey = ParamKey("okta_username");
    pub const DISABLE_SAML_URL_CHECK: ParamKey = ParamKey("disable_saml_url_check");
    pub const DISABLE_PARALLEL_USER_PROMPT: ParamKey = ParamKey("disable_parallel_user_prompt");
    pub const DISABLE_QUERY_CONTEXT_CACHE: ParamKey = ParamKey("disable_query_context_cache");
    pub const INCLUDE_RETRY_REASON: ParamKey = ParamKey("include_retry_reason");
    pub const SERIALIZE_SESSION_OPERATIONS: ParamKey = ParamKey("serialize_session_operations");
    pub const LOG_MAX_QUERY_LENGTH: ParamKey = ParamKey("log_max_query_length");
    pub const LOG_QUERY_TEXT: ParamKey = ParamKey("log_query_text");
    pub const LOG_QUERY_PARAMETERS: ParamKey = ParamKey("log_query_parameters");
    pub const CLIENT_TELEMETRY_ENABLED: ParamKey = ParamKey("CLIENT_TELEMETRY_ENABLED");
    pub const CLIENT_SESSION_KEEP_ALIVE: ParamKey = ParamKey("CLIENT_SESSION_KEEP_ALIVE");
    pub const CLIENT_SESSION_KEEP_ALIVE_HEARTBEAT_FREQUENCY: ParamKey =
        ParamKey("CLIENT_SESSION_KEEP_ALIVE_HEARTBEAT_FREQUENCY");
    // ── OAuth (cross-driver configuration matrix) ─────────────────────────
    pub const OAUTH_CLIENT_ID: ParamKey = ParamKey("oauth_client_id");
    pub const OAUTH_CLIENT_SECRET: ParamKey = ParamKey("oauth_client_secret");
    pub const OAUTH_AUTHORIZATION_URL: ParamKey = ParamKey("oauth_authorization_url");
    pub const OAUTH_TOKEN_REQUEST_URL: ParamKey = ParamKey("oauth_token_request_url");
    pub const OAUTH_REDIRECT_URI: ParamKey = ParamKey("oauth_redirect_uri");
    pub const OAUTH_SCOPE: ParamKey = ParamKey("oauth_scope");
    pub const OAUTH_ENABLE_SINGLE_USE_REFRESH_TOKENS: ParamKey =
        ParamKey("oauth_enable_single_use_refresh_tokens");
    pub const OAUTH_DISABLE_PKCE: ParamKey = ParamKey("oauth_disable_pkce");
    pub const OAUTH_ENABLE_DPOP: ParamKey = ParamKey("oauth_enable_dpop");
    pub const OAUTH_CREDENTIALS_IN_BODY: ParamKey = ParamKey("oauth_credentials_in_body");
    pub const OAUTH_DISABLE_CONSOLE_LOGIN: ParamKey = ParamKey("oauth_disable_console_login");
    // Logout configuration
    pub const SERVER_SESSION_KEEP_ALIVE: ParamKey = ParamKey("server_session_keep_alive");
    pub const ENABLE_SERVER_SESSION_KEEP_ALIVE_AUTO_DETECTION: ParamKey =
        ParamKey("enable_server_session_keep_alive_auto_detection");
    pub const LOGOUT_ERROR_STRATEGY: ParamKey = ParamKey("logout_error_strategy");
    pub const LOGOUT_TOTAL_TIMEOUT_SECONDS: ParamKey = ParamKey("logout_total_timeout_seconds");
    pub const LOGOUT_MAX_ATTEMPTS: ParamKey = ParamKey("logout_max_attempts");
    pub const LOGOUT_REQUEST_TIMEOUT_SECONDS: ParamKey = ParamKey("logout_request_timeout_seconds");
    // HTTP retry configuration
    pub const RETRY_MAX_ATTEMPTS: ParamKey = ParamKey("retry_max_attempts");
    // Exponential-backoff curve, shared by the HTTP and PUT/GET retry
    // pipelines (a single set of knobs overrides both).
    pub const RETRY_BACKOFF_BASE_MS: ParamKey = ParamKey("retry_backoff_base_ms");
    pub const RETRY_BACKOFF_CAP_MS: ParamKey = ParamKey("retry_backoff_cap_ms");
    pub const RETRY_BACKOFF_FACTOR: ParamKey = ParamKey("retry_backoff_factor");
    pub const RETRY_BACKOFF_JITTER: ParamKey = ParamKey("retry_backoff_jitter");
    pub const RETRY_EXTRA_STATUS_CODES: ParamKey = ParamKey("retry_extra_status_codes");
    // PUT/GET file transfer configuration
    pub const PUT_GET_MAX_ATTEMPTS: ParamKey = ParamKey("put_get_max_attempts");
    /// JDBC-only. When `false`, client-side PUT/GET (file transfers) are
    /// rejected before dispatch with "File transfers have been disabled."
    /// Default `true`. Mirrors legacy snowflake-jdbc's `enablePutGet` client
    /// property and is only honored by wrappers that opt in via
    /// `WrapperPresets::honor_put_get_disable` (JDBC).
    pub const ENABLE_PUT_GET: ParamKey = ParamKey("enable_put_get");
    /// When `true`, skip file permission checks on `config.toml` and
    /// `connections.toml` during connection setup (SNOW-3548119). Use this
    /// in environments where file permissions cannot be controlled (shared CI
    /// runners, containers, mounted volumes). The `unsafe_` prefix signals that
    /// skipping the check weakens protection against local tampering. Default
    /// `false`. Unix-only; ignored on Windows.
    pub const UNSAFE_SKIP_CONFIG_FILE_PERMISSIONS_CHECK: ParamKey =
        ParamKey("unsafe_skip_config_file_permissions_check");
    pub const UNSAFE_FILE_WRITE: ParamKey = ParamKey("unsafe_file_write");
    // Application identity
    pub const CLIENT_APP_ID: ParamKey = ParamKey("client_app_id");
    pub const CLIENT_APP_VERSION: ParamKey = ParamKey("client_app_version");
    pub const APPLICATION: ParamKey = ParamKey("application");
    // Prefetch configuration
    pub const CLIENT_PREFETCH_THREADS: ParamKey = ParamKey("CLIENT_PREFETCH_THREADS");
    pub const CLIENT_MEMORY_LIMIT: ParamKey = ParamKey("CLIENT_MEMORY_LIMIT");
    // PUT/GET — S3 regional endpoint override. Server pushes this as the
    // session parameter `ENABLE_STAGE_S3_PRIVATELINK_FOR_US_EAST_1`; the
    // canonical name matches the field on `StageInfo` (and libsfclient's
    // `use_s3_regional_url` connection attribute).
    pub const USE_S3_REGIONAL_URL: ParamKey = ParamKey("use_s3_regional_url");
    pub const VALIDATE_DEFAULT_PARAMETERS: ParamKey = ParamKey("validate_default_parameters");
    // ── Timeout configuration ──────────────────────────────────────────
    pub const CONNECT_TIMEOUT: ParamKey = ParamKey("connect_timeout");
    pub const LOGIN_TIMEOUT: ParamKey = ParamKey("login_timeout");
    pub const QUERY_TIMEOUT: ParamKey = ParamKey("query_timeout");
    pub const REQUEST_TIMEOUT: ParamKey = ParamKey("request_timeout");
    pub const RETRY_TIMEOUT: ParamKey = ParamKey("retry_timeout");
    // Proxy configuration
    pub const PROXY_HOST: ParamKey = ParamKey("proxy_host");
    pub const PROXY_PORT: ParamKey = ParamKey("proxy_port");
    pub const PROXY_SCHEME: ParamKey = ParamKey("proxy_scheme");
    pub const PROXY_USER: ParamKey = ParamKey("proxy_user");
    pub const PROXY_PASSWORD: ParamKey = ParamKey("proxy_password");
    pub const NO_PROXY: ParamKey = ParamKey("no_proxy");
    /// Full proxy URL `[scheme://][user:pass@]host[:port]`, accepted as the
    /// legacy ODBC `PROXY` connection string key.  Parsed in
    /// `build_proxy_config` and merged with the individual `proxy_*` fields,
    /// which override URL components when both are set.
    pub const PROXY: ParamKey = ParamKey("proxy");
    /// Whether to fall back to `HTTP_PROXY` / `HTTPS_PROXY` / `NO_PROXY`
    /// environment variables when no explicit proxy is configured.
    /// Default `false`: env detection is suppressed.
    pub const USE_PROXY_ENV: ParamKey = ParamKey("use_proxy_env");
    /// When `true` (default), an empty `PROXY` value explicitly disables the
    /// proxy and overrides config/env settings, mirroring legacy ODBC
    /// `AllowEmptyProxy=true`. When `false`, an empty value is ignored.
    pub const ALLOW_EMPTY_PROXY: ParamKey = ParamKey("allow_empty_proxy");
    /// ODBC-only. When `true`, a NULL `CatalogName` on catalog functions is
    /// replaced with the current database. When `false` (default, matching
    /// legacy snowflake-odbc `UseCurrentCatalog`), the catalog stays
    /// unconstrained unless `CLIENT_METADATA_REQUEST_USE_CONNECTION_CTX` fills
    /// it.
    pub const USE_CURRENT_CATALOG: ParamKey = ParamKey("use_current_catalog");

    /// When `true`, run connectivity diagnostics during connect.
    /// Default `false`.
    pub const ENABLE_CONNECTION_DIAG: ParamKey = ParamKey("enable_connection_diag");
    /// Directory path where the diagnostic report file is written.
    /// Only used when `ENABLE_CONNECTION_DIAG` is `true`.
    pub const CONNECTION_DIAG_LOG_PATH: ParamKey = ParamKey("connection_diag_log_path");
    /// Path to a pre-fetched `allowlist.json` file used during diagnostics.
    /// When absent, the driver fetches the allowlist live via `system$allowlist()`.
    pub const CONNECTION_DIAG_ALLOWLIST_PATH: ParamKey = ParamKey("connection_diag_allowlist_path");
    // ── Session token authentication ────────────────────────────────────
    pub const SESSION_TOKEN: ParamKey = ParamKey("session_token");
    pub const MASTER_TOKEN: ParamKey = ParamKey("master_token");
    pub const MASTER_VALIDITY_IN_SECONDS: ParamKey = ParamKey("master_validity_in_seconds");

    // ── Workload Identity Federation (WIF) ────────────────────────────
    /// Cloud provider used for WIF attestation token acquisition.
    /// Required when authenticator = WORKLOAD_IDENTITY.
    /// Accepted values (case-insensitive): `AWS`, `AZURE`, `GCP`, `OIDC`.
    pub const WORKLOAD_IDENTITY_PROVIDER: ParamKey = ParamKey("workload_identity_provider");
    /// Override the Azure Entra resource URI for the managed-identity token
    /// request. Defaults to `api://fd3f753b-eed3-462c-b6a7-a4b5bb650aad`
    /// when absent.  Azure provider only.
    pub const WORKLOAD_IDENTITY_ENTRA_RESOURCE: ParamKey =
        ParamKey("workload_identity_entra_resource");
    /// Comma-separated impersonation chain.
    /// AWS: IAM role ARNs to assume in order (e.g. `arn:aws:iam::123:role/A`).
    /// GCP: service account emails to impersonate in order.
    /// Not supported for AZURE or OIDC providers.
    pub const WORKLOAD_IDENTITY_IMPERSONATION_PATH: ParamKey =
        ParamKey("workload_identity_impersonation_path");
    /// When `true` (AWS provider only), acquire the WIF attestation via outbound
    /// STS `GetWebIdentityToken` instead of the default pre-signed
    /// `GetCallerIdentity` token. Takes precedence over
    /// `SNOWFLAKE_ENABLE_AWS_WIF_OUTBOUND_TOKEN`. Default `false`.
    pub const WORKLOAD_IDENTITY_AWS_USE_OUTBOUND_TOKEN: ParamKey =
        ParamKey("workload_identity_aws_use_outbound_token");
    /// Pre-acquired OIDC JWT forwarded directly to Snowflake.
    /// Required when `workload_identity_provider = OIDC`.
    /// Reuses the existing `token` param key for OIDC so callers that
    /// already set `token` do not need a separate key.
    pub const WORKLOAD_IDENTITY_TOKEN: ParamKey = ParamKey("token");

    // ── ODBC keywords the driver accepts and does not apply ───────────
    pub const DSN: ParamKey = ParamKey("dsn");
    pub const DRIVER: ParamKey = ParamKey("driver");
    pub const FILEDSN: ParamKey = ParamKey("filedsn");
    pub const SAVEFILE: ParamKey = ParamKey("savefile");
    pub const DESCRIPTION: ParamKey = ParamKey("description");
    pub const LOCALE: ParamKey = ParamKey("locale");
    pub const SETUP: ParamKey = ParamKey("setup");
    pub const DRIVER_ODBC_VER: ParamKey = ParamKey("driverodbcver");
    pub const API_LEVEL: ParamKey = ParamKey("apilevel");
    pub const SQL_LEVEL: ParamKey = ParamKey("sqllevel");
    pub const CONNECT_FUNCTIONS: ParamKey = ParamKey("connectfunctions");
    pub const TRACING: ParamKey = ParamKey("tracing");
    // ── Deprecated ODBC connection-string keys ────────────────────────
    pub const LOG_LEVEL: ParamKey = ParamKey("log_level");
    pub const LOG_PATH: ParamKey = ParamKey("log_path");
    pub const LOG_FILE_SIZE: ParamKey = ParamKey("log_file_size");
    pub const LOG_FILE_COUNT: ParamKey = ParamKey("log_file_count");
    pub const CURL_VERBOSE_MODE: ParamKey = ParamKey("curl_verbose_mode");
    pub const ENABLE_PID_LOG_FILE_NAMES: ParamKey = ParamKey("enable_pid_log_file_names");
    pub const CLIENT_CONFIG_FILE: ParamKey = ParamKey("client_config_file");
    pub const DEFAULT_VARCHAR_SIZE: ParamKey = ParamKey("default_varchar_size");
    pub const DEFAULT_BINARY_SIZE: ParamKey = ParamKey("default_binary_size");
}

/// Default `retry_max_attempts` for general HTTP calls (mirrors the `ParamDef`).
pub const DEFAULT_RETRY_MAX_ATTEMPTS: u32 = 6;

/// Default `put_get_max_attempts` (mirrors the `ParamDef`).
pub const DEFAULT_PUT_GET_MAX_ATTEMPTS: u32 = 6;

/// Default `login_timeout` in seconds (mirrors the `ParamDef`).
pub const DEFAULT_LOGIN_TIMEOUT_SECS: u64 = 120;

/// ODBC's default `authentication_timeout` in seconds, matching the old
/// snowflake-odbc driver's `S_DEFAULT_LOGIN_TIMEOUT`. It replaces the registry
/// default for that wrapper only, and is applied underneath `connections.toml`
/// so a profile can still lower it.
pub const ODBC_DEFAULT_AUTHENTICATION_TIMEOUT_SECS: i64 = 300;

/// Default `query_timeout` in seconds. 0 = no timeout (queries can be long-running).
pub const DEFAULT_QUERY_TIMEOUT_SECS: u64 = 0;

/// Default `request_timeout` in seconds for non-login, non-query operations.
pub const DEFAULT_REQUEST_TIMEOUT_SECS: u64 = 120;

/// Common exponential-backoff defaults shared by the HTTP and PUT/GET retry
/// pipelines. These are the single source of truth: both the `ParamDef`
/// defaults below and `RetryPolicy`'s backoff construction in
/// [`crate::config::retry`] reference them.
pub const DEFAULT_RETRY_BACKOFF_BASE_MS: u64 = 250;
pub const DEFAULT_RETRY_BACKOFF_CAP_MS: u64 = 16_000;
pub const DEFAULT_RETRY_BACKOFF_FACTOR: f64 = 2.0;
/// Default backoff jitter strategy (see `Jitter` in [`crate::config::retry`]).
pub const DEFAULT_RETRY_BACKOFF_JITTER: &str = "decorrelated";

/// Which API layer owns writes for a parameter.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ParamScope {
    Connection,
    Session,
    Statement,
}

/// Identity of a language/protocol wrapper that talks to core.
///
/// Used by [`Alias`] scoping and [`ParamRegistry::resolve_for`] so the same
/// wire spelling can map to different canonicals depending on which wrapper
/// sent it (e.g. ODBC `LOGIN_TIMEOUT` vs JDBC `LOGIN_TIMEOUT`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Wrapper {
    Odbc,
    Jdbc,
    Python,
    NodeJs,
    DotNet,
}

/// Which wrappers may resolve a parameter's canonical name.
///
/// Shared core settings use [`VisibleTo::All`] so a newly added [`Wrapper`]
/// inherits them. Wrapper-owned settings use [`VisibleTo::Only`] so other
/// wrappers cannot resolve the canonical name — the same rule already applied
/// to scoped aliases.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VisibleTo {
    All,
    Only(&'static [Wrapper]),
}

/// An alternative accepted name for a parameter.
///
/// `wrapper: None` means the alias is accepted by every wrapper (global);
/// `Some(w)` means it is only visible when resolving in the context of `w`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Alias {
    pub name: &'static str,
    pub wrapper: Option<Wrapper>,
}

impl Alias {
    pub const fn global(name: &'static str) -> Self {
        Self {
            name,
            wrapper: None,
        }
    }

    pub const fn scoped(wrapper: Wrapper, name: &'static str) -> Self {
        Self {
            name,
            wrapper: Some(wrapper),
        }
    }
}

/// Builds the `aliases:` slice for a [`ParamDef`], in either global or
/// wrapper-scoped form:
///
/// * `aliases![]` — no aliases (the common case; a [`VisibleTo::All`]
///   canonical still resolves case-insensitively).
/// * `aliases!["A", "B"]` — global aliases visible to every wrapper via
///   [`ParamRegistry::resolve`]. **Do not use this form.** After the
///   per-wrapper migration no parameter has a global alias, and the
///   `every_alias_is_wrapper_scoped` test fails if one appears; the arm exists
///   only because the empty `aliases![]` above parses through it. Scope the
///   spelling to the wrapper(s) whose old driver accepted it instead — a
///   spelling every wrapper really shares belongs in that test's exception list
///   with a comment naming each driver.
/// * `aliases![Odbc; "A", "B"]` — each name scoped to one wrapper.
///
/// Scoped spellings resolve under [`ParamRegistry::resolve_for`] for the listed
/// wrappers only and are invisible to the wrapper-agnostic
/// [`ParamRegistry::resolve`]. An alias exists **only** where the old driver for
/// that wrapper actually accepted the spelling — UD is no more lenient than the
/// driver it replaces, so a convenience spelling nobody shipped is not added:
/// * SCREAMING/DSN spellings (`SERVER`, `PRIV_KEY_FILE`, …) are legacy ODBC
///   connection-string keys from snowflake-odbc's `Snowflake.h`. The exception is
///   `CRL_MODE`/`CRL_ENABLED`, UD-ODBC's own DSN keys for a feature legacy spelled
///   `CRL_CHECK`.
/// * camelCase spellings (`oauthClientId`, `allowUnderscoresInHost`, …) are JDBC
///   `SFSessionProperty` keys. JDBC also has lowercase-underscore properties that
///   differ from our canonical name (`private_key_base64`, `private_key_pwd`),
///   so those are `Jdbc`-scoped too — not ODBC keys despite the shape.
/// * `Python` scope covers two callers: the Python wrapper (whose `_ALIAS_MAP` is
///   generated from these aliases) and `config.toml`/`connections.toml` profiles,
///   which `config_manager` canonicalizes under `Wrapper::Python`. Every
///   `Python`-scoped spelling is a legacy `snowflake-connector-python` kwarg from
///   its `DEFAULT_CONFIGURATION`.
/// * `NodeJs` spellings are legacy snowflake-connector-nodejs connection options
///   (`lib/connection/connection_config.js`).
///
/// A spelling that matches the canonical name case-insensitively (legacy ODBC's
/// `NO_PROXY`, legacy Python's `no_proxy`) needs no alias at all.
///
/// `aliases![Odbc; "KEY"]` == `&[Alias::scoped(Wrapper::Odbc, "KEY")]`.
///
/// Three cases are written with explicit [`Alias::scoped`] entries instead of
/// the macro: a spelling two or more wrappers share (one entry per wrapper),
/// wrappers that map *different* spellings to the same canonical (the ODBC
/// `PRIV_KEY_PWD` / JDBC `private_key_pwd` split), and the same spelling
/// mapping to different canonicals (the ODBC-only `LOGIN_TIMEOUT`).
#[macro_export]
macro_rules! aliases {
    // Global aliases (visible to every wrapper), including the empty list.
    ($($name:expr),* $(,)?) => {
        &[$($crate::Alias::global($name)),*]
    };
    // One wrapper, one or more names.
    ($wrapper:ident; $($name:expr),+ $(,)?) => {
        &[$($crate::Alias::scoped($crate::Wrapper::$wrapper, $name)),+]
    };
}

/// Builds a [`VisibleTo`] value for a [`ParamDef`].
///
/// * `visible_to!(All)` — every wrapper (the common case).
/// * `visible_to!(Odbc)` / `visible_to!(Odbc, Jdbc)` — listed wrappers only.
#[macro_export]
macro_rules! visible_to {
    (All) => {
        $crate::VisibleTo::All
    };
    ($($wrapper:ident),+ $(,)?) => {
        $crate::VisibleTo::Only(&[$($crate::Wrapper::$wrapper),+])
    };
}

/// Defines a single supported configuration parameter.
///
/// Construct with [`ParamDef::builder`]. Required setters have no default;
/// omitted optional setters keep empty aliases, `Required::Never`, and
/// `None` for the remaining fields.
pub struct ParamDef {
    /// The canonical key name used internally (e.g. `"host"`).
    pub canonical_name: &'static str,

    /// Alternative names accepted from wrappers (case-insensitive lookup).
    /// Global aliases apply to every wrapper; scoped ones only when resolving
    /// via [`ParamRegistry::resolve_for`] for that wrapper.
    pub aliases: &'static [Alias],

    /// Primary expected value type.
    pub value_type: ValueType,

    /// Additional accepted value type when a wrapper legitimately sends a
    /// second representation for the same parameter.
    pub additional_value_type: Option<ValueType>,

    /// When this parameter is required.
    pub required: Required,

    /// Default value, if any.
    pub default: Option<DefaultValue>,

    /// Whether the value contains secrets (for log redaction).
    pub sensitive: bool,

    /// Whether the parameter participates in authentication (establishing the
    /// caller's identity). Wrappers use this to classify auth-time failures —
    /// e.g. the ODBC SQLSTATE mapper reports an invalid/missing value for an
    /// auth parameter as `28000` (invalid authorization specification) rather
    /// than the generic connection-string-attribute state. Covers the primary
    /// credential family (`user`, `password`, `authenticator`, key-pair,
    /// pre-acquired `token`), every OAuth parameter, and every WIF parameter.
    pub auth: bool,

    /// Human-readable description.
    pub description: &'static str,

    /// If set, the parameter is deprecated. [`Deprecation::ReplacedBy`] names
    /// the successor; [`Deprecation::Ignored`] carries guidance instead.
    pub deprecated: Option<Deprecation>,

    /// Which API layer(s) may write this parameter. A parameter may be valid at
    /// more than one level (e.g. `QUERY_TAG` is settable both at the
    /// session/connection level and per-statement).
    pub scopes: &'static [ParamScope],

    /// When true, the resolved connection-seed value participates in login / new session.
    pub used_at_connect: bool,

    /// When false, connection-level setters must reject changes once connected.
    pub mutable_after_connect: bool,

    /// Wrappers that may resolve this parameter's canonical name. Restricted
    /// params are invisible to [`ParamRegistry::resolve`] and to
    /// [`ParamRegistry::resolve_for`] for wrappers not listed here.
    /// [`ParamRegistry::is_known`] still returns true for every canonical.
    pub visible_to: VisibleTo,

    /// When true, wrappers drop the key during option normalization. The
    /// parameter is registered so it is not an unknown session parameter, and
    /// its value is not applied.
    pub ignored: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ValueType {
    String,
    Int,
    Double,
    #[allow(dead_code)]
    Bytes,
    Bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Required {
    /// Always required (e.g. `account`).
    Always,
    /// Required only when the authenticator matches (e.g. `password` for
    /// `SNOWFLAKE_PASSWORD`).
    WhenAuthMethod(&'static str),
    /// Never required.
    Never,
}

/// Why a [`ParamDef`] is deprecated.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Deprecation {
    /// Superseded by the named parameter, which the driver still applies.
    ReplacedBy(&'static str),
    /// Accepted for compatibility and no longer applied. `guidance` is a
    /// sentence naming the supported way to get the behavior.
    Ignored { guidance: &'static str },
}

impl Deprecation {
    /// The diagnostic every wrapper reports for `parameter`.
    pub fn message_for(&self, parameter: &str) -> String {
        match self {
            Self::ReplacedBy(replacement) => {
                format!("Parameter '{parameter}' is deprecated, use '{replacement}' instead")
            }
            Self::Ignored { guidance } => {
                format!("Parameter '{parameter}' is deprecated and has no effect. {guidance}")
            }
        }
    }
}

impl ParamDef {
    pub const fn builder() -> ParamDefBuilder {
        ParamDefBuilder {
            canonical_name: None,
            value_type: None,
            description: None,
            scopes: None,
            used_at_connect: None,
            mutable_after_connect: None,
            sensitive: None,
            auth: None,
            aliases: &[],
            additional_value_type: None,
            required: Required::Never,
            default: None,
            deprecated: None,
            visible_to: VisibleTo::All,
            ignored: false,
        }
    }

    /// Alias names visible to `wrapper`: globals plus that wrapper's scoped ones.
    pub fn alias_names_for(&self, wrapper: Wrapper) -> impl Iterator<Item = &'static str> + '_ {
        self.aliases
            .iter()
            .filter(move |a| a.wrapper.is_none() || a.wrapper == Some(wrapper))
            .map(|a| a.name)
    }

    /// Whether the resolved value may participate in login / new session creation.
    ///
    /// Statement-only parameters (no connection/session scope) are never consumed
    /// at connect regardless of stored metadata.
    #[inline]
    pub fn effective_used_at_connect(&self) -> bool {
        if self.is_statement_only() {
            return false;
        }
        self.used_at_connect
    }

    /// True when the parameter can only be set per-statement (no connection or
    /// session scope).
    #[inline]
    pub fn is_statement_only(&self) -> bool {
        !self.scopes.contains(&ParamScope::Connection)
            && !self.scopes.contains(&ParamScope::Session)
    }

    /// True when the parameter may be overridden per-statement.
    #[inline]
    pub fn is_statement_scoped(&self) -> bool {
        self.scopes.contains(&ParamScope::Statement)
    }

    /// True when the parameter may be set at the session level (at connect or via
    /// a post-connect session override).
    #[inline]
    pub fn is_session_scoped(&self) -> bool {
        self.scopes.contains(&ParamScope::Session)
    }

    /// True when `wrapper` may resolve this parameter's canonical name.
    #[inline]
    pub fn is_visible_to(&self, wrapper: Wrapper) -> bool {
        match self.visible_to {
            VisibleTo::All => true,
            VisibleTo::Only(wrappers) => wrappers.contains(&wrapper),
        }
    }
}

/// Accumulates a [`ParamDef`]. Required setters have no default.
/// [`build`](Self::build) panics during const evaluation when one is missing.
pub struct ParamDefBuilder {
    canonical_name: Option<&'static str>,
    value_type: Option<ValueType>,
    description: Option<&'static str>,
    scopes: Option<&'static [ParamScope]>,
    used_at_connect: Option<bool>,
    mutable_after_connect: Option<bool>,
    sensitive: Option<bool>,
    auth: Option<bool>,
    aliases: &'static [Alias],
    additional_value_type: Option<ValueType>,
    required: Required,
    default: Option<DefaultValue>,
    deprecated: Option<Deprecation>,
    visible_to: VisibleTo,
    ignored: bool,
}

impl ParamDefBuilder {
    pub const fn canonical_name(mut self, value: &'static str) -> Self {
        self.canonical_name = Some(value);
        self
    }

    pub const fn value_type(mut self, value: ValueType) -> Self {
        self.value_type = Some(value);
        self
    }

    pub const fn description(mut self, value: &'static str) -> Self {
        self.description = Some(value);
        self
    }

    pub const fn scopes(mut self, value: &'static [ParamScope]) -> Self {
        self.scopes = Some(value);
        self
    }

    pub const fn used_at_connect(mut self, value: bool) -> Self {
        self.used_at_connect = Some(value);
        self
    }

    pub const fn mutable_after_connect(mut self, value: bool) -> Self {
        self.mutable_after_connect = Some(value);
        self
    }

    pub const fn aliases(mut self, value: &'static [Alias]) -> Self {
        self.aliases = value;
        self
    }

    pub const fn additional_value_type(mut self, value: ValueType) -> Self {
        self.additional_value_type = Some(value);
        self
    }

    pub const fn required(mut self, value: Required) -> Self {
        self.required = value;
        self
    }

    pub const fn default(mut self, value: DefaultValue) -> Self {
        self.default = Some(value);
        self
    }

    pub const fn sensitive(mut self, value: bool) -> Self {
        self.sensitive = Some(value);
        self
    }

    pub const fn auth(mut self, value: bool) -> Self {
        self.auth = Some(value);
        self
    }

    pub const fn deprecated(mut self, value: Deprecation) -> Self {
        self.deprecated = Some(value);
        self
    }

    pub const fn deprecated_by(mut self, value: &'static str) -> Self {
        self.deprecated = Some(Deprecation::ReplacedBy(value));
        self
    }

    pub const fn visible_to(mut self, value: VisibleTo) -> Self {
        self.visible_to = value;
        self
    }

    pub const fn ignored(mut self, value: bool) -> Self {
        self.ignored = value;
        self
    }

    pub const fn build(self) -> ParamDef {
        ParamDef {
            canonical_name: match self.canonical_name {
                Some(value) => value,
                None => panic!("ParamDef::builder() requires canonical_name"),
            },
            value_type: match self.value_type {
                Some(value) => value,
                None => panic!("ParamDef::builder() requires value_type"),
            },
            description: match self.description {
                Some(value) => value,
                None => panic!("ParamDef::builder() requires description"),
            },
            scopes: match self.scopes {
                Some(value) => value,
                None => panic!("ParamDef::builder() requires scopes"),
            },
            used_at_connect: match self.used_at_connect {
                Some(value) => value,
                None => panic!("ParamDef::builder() requires used_at_connect"),
            },
            mutable_after_connect: match self.mutable_after_connect {
                Some(value) => value,
                None => panic!("ParamDef::builder() requires mutable_after_connect"),
            },
            sensitive: match self.sensitive {
                Some(value) => value,
                None => panic!("ParamDef::builder() requires sensitive"),
            },
            auth: match self.auth {
                Some(value) => value,
                None => panic!("ParamDef::builder() requires auth"),
            },
            aliases: self.aliases,
            additional_value_type: self.additional_value_type,
            required: self.required,
            default: self.default,
            deprecated: self.deprecated,
            visible_to: self.visible_to,
            ignored: self.ignored,
        }
    }
}

static PARAM_DEFS: &[ParamDef] = &[
    // ── Server ──────────────────────────────────────────────────────────
    ParamDef::builder()
        .canonical_name(param_names::ACCOUNT.as_str())
        .value_type(ValueType::String)
        .required(Required::Always)
        .sensitive(false)
        .auth(false)
        .description("Snowflake account identifier")
        .scopes(&[ParamScope::Connection])
        .used_at_connect(true)
        .mutable_after_connect(false)
        .build(),
    ParamDef::builder()
        .canonical_name(param_names::HOST.as_str())
        // `HOST` is redundant with the canonical name (case-insensitive match).
        // `SERVER` is the ODBC DSN spelling (`Snowflake.h` `SF_HOST_KEY`) and is
        // ODBC-only: the legacy Python connector has no `server` kwarg, and JDBC
        // carries the host in the JDBC URL.
        .aliases(aliases![Odbc; "SERVER"])
        .value_type(ValueType::String)
        .sensitive(false)
        .auth(false)
        .description("Snowflake server hostname")
        .scopes(&[ParamScope::Connection])
        .used_at_connect(true)
        .mutable_after_connect(false)
        .build(),
    ParamDef::builder()
        .canonical_name(param_names::PORT.as_str())
        .value_type(ValueType::Int)
        .sensitive(false)
        .auth(false)
        .description("Server port number")
        .scopes(&[ParamScope::Connection])
        .used_at_connect(true)
        .mutable_after_connect(false)
        .build(),
    ParamDef::builder()
        .canonical_name(param_names::PROTOCOL.as_str())
        .value_type(ValueType::String)
        .sensitive(false)
        .auth(false)
        .description("Connection protocol (http or https)")
        .scopes(&[ParamScope::Connection])
        .used_at_connect(true)
        .mutable_after_connect(false)
        .build(),
    ParamDef::builder()
        .canonical_name(param_names::SSL.as_str())
        .value_type(ValueType::Bool)
        .sensitive(false)
        .auth(false)
        .description("Enable or disable SSL/TLS (sets protocol to https or http)")
        .deprecated_by("protocol")
        .scopes(&[ParamScope::Connection])
        .used_at_connect(true)
        .mutable_after_connect(false)
        .build(),
    ParamDef::builder()
        .canonical_name(param_names::SERVER_URL.as_str())
        .value_type(ValueType::String)
        .sensitive(false)
        .auth(false)
        .description("Full server URL (alternative to host/port/protocol)")
        .scopes(&[ParamScope::Connection])
        .used_at_connect(true)
        .mutable_after_connect(false)
        .build(),
    ParamDef::builder()
        .canonical_name(param_names::PRESERVE_UNDERSCORES_IN_HOSTNAME.as_str())
        // JDBC-only `allowUnderscoresInHost` property (case-insensitive).
        .aliases(aliases![Jdbc; "allowUnderscoresInHost"])
        .value_type(ValueType::Bool)
        .default(DefaultValue::Bool(false))
        .sensitive(false)
        .auth(false)
        .description("Preserve underscores in the hostname derived from the account name")
        .scopes(&[ParamScope::Connection])
        .used_at_connect(true)
        .mutable_after_connect(false)
        .build(),
    // ── Auth ────────────────────────────────────────────────────────────
    ParamDef::builder()
        .canonical_name(param_names::USER.as_str())
        // ODBC DSN `UID` (`Snowflake.h`). ODBC-only: the legacy Python connector
        // has no `uid` kwarg.
        .aliases(aliases![Odbc; "UID"])
        .value_type(ValueType::String)
        .required(Required::Always)
        .sensitive(false)
        .auth(true)
        .description("Login username")
        .scopes(&[ParamScope::Connection])
        .used_at_connect(true)
        .mutable_after_connect(false)
        .build(),
    ParamDef::builder()
        .canonical_name(param_names::PASSWORD.as_str())
        // ODBC DSN `PWD` (`Snowflake.h`). ODBC-only: the legacy Python connector
        // has no `pwd` kwarg.
        .aliases(aliases![Odbc; "PWD"])
        .value_type(ValueType::String)
        .required(Required::WhenAuthMethod("SNOWFLAKE_PASSWORD"))
        .sensitive(true)
        .auth(true)
        .description("Login password")
        .scopes(&[ParamScope::Connection])
        .used_at_connect(true)
        .mutable_after_connect(false)
        .build(),
    ParamDef::builder()
        .canonical_name(param_names::AUTHENTICATOR.as_str())
        .value_type(ValueType::String)
        .sensitive(false)
        .auth(true)
        .description("Authenticator type for the connection")
        .scopes(&[ParamScope::Connection])
        .used_at_connect(true)
        .mutable_after_connect(false)
        .build(),
    ParamDef::builder()
        .canonical_name(param_names::PRIVATE_KEY.as_str())
        // `PRIV_KEY_BASE64` is the legacy ODBC DSN key (`Snowflake.h`
        // `SF_PRIV_KEY_BASE64_KEY`); `private_key_base64` is the JDBC property
        // (`SFSessionProperty.PRIVATE_KEY_BASE64`) — legacy ODBC never accepted
        // the fully-underscored spelling.
        .aliases(&[
            Alias::scoped(Wrapper::Odbc, "PRIV_KEY_BASE64"),
            Alias::scoped(Wrapper::Jdbc, "PRIVATE_KEY_BASE64"),
        ])
        .value_type(ValueType::String)
        .additional_value_type(ValueType::Bytes)
        .required(Required::WhenAuthMethod("SNOWFLAKE_JWT"))
        .sensitive(true)
        .auth(true)
        .description("Private key for key-pair authentication (base64-encoded or PEM)")
        .scopes(&[ParamScope::Connection])
        .used_at_connect(true)
        .mutable_after_connect(false)
        .build(),
    ParamDef::builder()
        .canonical_name(param_names::PRIVATE_KEY_FILE.as_str())
        // ODBC DSN `PRIV_KEY_FILE`.
        .aliases(aliases![Odbc; "PRIV_KEY_FILE"])
        .value_type(ValueType::String)
        .sensitive(false)
        .auth(true)
        .description("Path to private key file for key-pair authentication")
        .scopes(&[ParamScope::Connection])
        .used_at_connect(true)
        .mutable_after_connect(false)
        .build(),
    ParamDef::builder()
        .canonical_name(param_names::PRIVATE_KEY_PASSWORD.as_str())
        // `PRIV_KEY_FILE_PWD` / `PRIV_KEY_PWD` are the legacy ODBC DSN
        // passphrase keys (`Snowflake.h`). `private_key_pwd` and
        // `private_key_file_pwd` are JDBC properties (`SFSessionProperty`);
        // `private_key_file_pwd` is also a legacy snowflake-connector-python
        // kwarg, and needs `Python` scope for the TOML loader, which
        // canonicalizes through the registry under the Python flavor rather
        // than through the Python wrapper's generated `_ALIAS_MAP`.
        .aliases(&[
            Alias::scoped(Wrapper::Odbc, "PRIV_KEY_FILE_PWD"),
            Alias::scoped(Wrapper::Odbc, "PRIV_KEY_PWD"),
            Alias::scoped(Wrapper::Jdbc, "PRIVATE_KEY_PWD"),
            Alias::scoped(Wrapper::Jdbc, "PRIVATE_KEY_FILE_PWD"),
            Alias::scoped(Wrapper::Python, "PRIVATE_KEY_FILE_PWD"),
        ])
        .value_type(ValueType::String)
        .sensitive(true)
        .auth(true)
        .description("Passphrase for encrypted private key")
        .scopes(&[ParamScope::Connection])
        .used_at_connect(true)
        .mutable_after_connect(false)
        .build(),
    ParamDef::builder()
        .canonical_name(param_names::TOKEN.as_str())
        .value_type(ValueType::String)
        .required(Required::WhenAuthMethod("PROGRAMMATIC_ACCESS_TOKEN"))
        .sensitive(true)
        .auth(true)
        .description("Pre-acquired bearer token (PAT, legacy OAUTH, or OIDC WIF). Alternative to token_file_path")
        .scopes(&[ParamScope::Connection])
        .used_at_connect(true)
        .mutable_after_connect(false)
        .build(),
    ParamDef::builder()
        .canonical_name(param_names::TOKEN_FILE_PATH.as_str())
        // `tokenFilePath` is legacy snowflake-connector-nodejs' option spelling
        // (`lib/connection/connection_config.js`). Legacy .NET and JDBC read the
        // snake_case `token_file_path`, which matches the canonical name
        // case-insensitively and so needs no alias. This alias is inert for any
        // wrapper that takes the `Default` presets — see
        // `WrapperPresets::default`.
        .aliases(aliases![NodeJs; "tokenFilePath"])
        .value_type(ValueType::String)
        // The path is not itself a credential, but supplying it is how the
        // caller presents one — a bad path is an auth failure, not a bad
        // connection-string attribute.
        .sensitive(false)
        .auth(true)
        .description("Path to a file containing a pre-acquired bearer token (PAT, legacy OAUTH, or OIDC WIF). If both token and token_file_path are set, the file contents are used")
        .scopes(&[ParamScope::Connection])
        .used_at_connect(true)
        .mutable_after_connect(false)
        .build(),
    ParamDef::builder()
        .canonical_name(param_names::SESSION_TOKEN.as_str())
        .value_type(ValueType::String)
        .sensitive(true)
        .auth(false)
        .description("Pre-acquired session token for session token authentication")
        .scopes(&[ParamScope::Connection])
        .used_at_connect(true)
        .mutable_after_connect(false)
        .build(),
    ParamDef::builder()
        .canonical_name(param_names::MASTER_TOKEN.as_str())
        .value_type(ValueType::String)
        .sensitive(true)
        .auth(false)
        .description("Pre-acquired master token for session token authentication")
        .scopes(&[ParamScope::Connection])
        .used_at_connect(true)
        .mutable_after_connect(false)
        .build(),
    ParamDef::builder()
        .canonical_name(param_names::MASTER_VALIDITY_IN_SECONDS.as_str())
        .value_type(ValueType::Int)
        .sensitive(false)
        .auth(false)
        .description("Remaining validity in seconds for the master token (session token auth)")
        .scopes(&[ParamScope::Connection])
        .used_at_connect(true)
        .mutable_after_connect(false)
        .build(),
    ParamDef::builder()
        .canonical_name(param_names::PASSCODE.as_str())
        .value_type(ValueType::String)
        .sensitive(true)
        .auth(false)
        .description("MFA passcode for USERNAME_PASSWORD_MFA authentication")
        .scopes(&[ParamScope::Connection])
        .used_at_connect(true)
        .mutable_after_connect(false)
        .build(),
    ParamDef::builder()
        .canonical_name(param_names::PASSCODE_IN_PASSWORD.as_str())
        // `PASSCODE_IN_PASSWORD` is the legacy snowflake-connector-python kwarg
        // spelling. It needs `Python` scope for the TOML loader: the canonical
        // camelCase name does *not* match `passcode_in_password`
        // case-insensitively (the underscores differ), so a
        // `config.toml`/`connections.toml` profile would otherwise fail to
        // canonicalize. Legacy ODBC's DSN key is `PASSCODEINPASSWORD` (no
        // separators) and is rewritten wrapper-side in
        // `odbc/src/api/connection.rs`, so no ODBC alias belongs here.
        .aliases(aliases![Python; "PASSCODE_IN_PASSWORD"])
        .value_type(ValueType::Bool)
        .default(DefaultValue::Bool(false))
        .sensitive(false)
        .auth(false)
        .description("Whether the MFA passcode is appended to the password")
        .scopes(&[ParamScope::Connection])
        .used_at_connect(true)
        .mutable_after_connect(false)
        .build(),
    ParamDef::builder()
        .canonical_name(param_names::CLIENT_STORE_TEMPORARY_CREDENTIAL.as_str())
        // JDBC-only camelCase property.
        .aliases(aliases![Jdbc; "clientStoreTemporaryCredential"])
        .value_type(ValueType::Bool)
        .default(DefaultValue::Bool(true))
        .sensitive(false)
        .auth(false)
        .description(
            "When true, persist ID, OAuth, and MFA tokens in the OS credential store. \
             Defaults to true. An explicit value always wins",
        )
        .scopes(&[ParamScope::Connection])
        .used_at_connect(true)
        .mutable_after_connect(false)
        .build(),
    ParamDef::builder()
        .canonical_name(param_names::ENABLE_PUT_GET.as_str())
        // JDBC `SFSessionProperty.ENABLE_PUT_GET`. `ParameterKeyNormalizer` does
        // not carry this key, so the spelling reaches core verbatim and this
        // alias is what resolves it.
        .aliases(aliases![Jdbc; "enablePutGet"])
        .value_type(ValueType::Bool)
        .default(DefaultValue::Bool(true))
        .sensitive(false)
        .auth(false)
        .description("JDBC-only. When false, client-side PUT/GET file transfers are disabled")
        .scopes(&[ParamScope::Connection])
        .used_at_connect(true)
        .mutable_after_connect(false)
        .visible_to(visible_to!(Jdbc))
        .build(),
    ParamDef::builder()
        .canonical_name(param_names::DISABLE_PARALLEL_USER_PROMPT.as_str())
        .value_type(ValueType::Bool)
        .default(DefaultValue::Bool(true))
        .sensitive(false)
        .auth(false)
        .description("When true (default), enables process-global serialization of interactive auth \
                      prompts (external browser, MFA, OAuth) so that only one prompt is shown per \
                      <user, host> when clientStoreTemporaryCredential is enabled. Set to false to \
                      allow each concurrent connection to show its own prompt.")
        .scopes(&[ParamScope::Connection])
        .used_at_connect(true)
        .mutable_after_connect(false)
        .build(),
    ParamDef::builder()
        .canonical_name(param_names::DISABLE_QUERY_CONTEXT_CACHE.as_str())
        // Legacy ODBC (libsnowflakeclient `connection.c`) and legacy .NET
        // (`SFSessionProperty`) spell this `DISABLEQUERYCONTEXTCACHE`; legacy JDBC
        // (`SFSessionProperty`) and snowflake-connector-nodejs
        // (`connection_config.js`) spell it `disableQueryContextCache`. The two
        // differ only by case and resolve identically. Legacy Python's
        // `disable_query_context_cache` matches the canonical name, so `Python`
        // is absent here.
        .aliases(&[
            Alias::scoped(Wrapper::Odbc, "DISABLEQUERYCONTEXTCACHE"),
            Alias::scoped(Wrapper::DotNet, "DISABLEQUERYCONTEXTCACHE"),
            Alias::scoped(Wrapper::Jdbc, "disableQueryContextCache"),
            Alias::scoped(Wrapper::NodeJs, "disableQueryContextCache"),
        ])
        .value_type(ValueType::Bool)
        .default(DefaultValue::Bool(false))
        .sensitive(false)
        .auth(false)
        .description("When true, disables the client-side query context cache. \
                      No context is sent in requests and server-returned context is ignored.")
        .scopes(&[ParamScope::Connection])
        .used_at_connect(false)
        .mutable_after_connect(false)
        .build(),
    ParamDef::builder()
        .canonical_name(param_names::SERIALIZE_SESSION_OPERATIONS.as_str())
        .value_type(ValueType::Bool)
        .sensitive(false)
        .auth(false)
        .description(
            "When true, core holds a per-connection gate across every backend session \
             operation so only one is in flight at a time. When unset, defaults to the \
             wrapper's preset, client-only. Set at connect, cannot be changed after.",
        )
        .scopes(&[ParamScope::Connection])
        .used_at_connect(false)
        .mutable_after_connect(false)
        .build(),
    ParamDef::builder()
        .canonical_name(param_names::INCLUDE_RETRY_REASON.as_str())
        .aliases(&[
            Alias::scoped(Wrapper::Odbc, "includeRetryReason"),
            Alias::scoped(Wrapper::DotNet, "INCLUDERETRYREASON"),
            Alias::scoped(Wrapper::NodeJs, "includeRetryReason"),
            Alias::scoped(Wrapper::Python, "enable_retry_reason_in_query_response"),
        ])
        .value_type(ValueType::Bool)
        .default(DefaultValue::Bool(true))
        .sensitive(false)
        .auth(false)
        .description("When true, appends retryReason (the HTTP status code that triggered \
                      the retry, or 0 for transport errors with no HTTP response) \
                      alongside retryCount on retried query requests.")
        .scopes(&[ParamScope::Connection])
        .used_at_connect(true)
        .mutable_after_connect(false)
        .build(),
    ParamDef::builder()
        .canonical_name(param_names::AUTHENTICATION_TIMEOUT.as_str())
        // ODBC's LOGIN_TIMEOUT historically means authentication_timeout (it is an
        // auth-retry budget, not a socket timeout); for the other wrappers
        // `LOGIN_TIMEOUT` keeps matching the canonical `login_timeout` parameter
        // below, case-insensitively.
        .aliases(&[Alias::scoped(Wrapper::Odbc, "LOGIN_TIMEOUT")])
        .value_type(ValueType::Int)
        .default(DefaultValue::Int(120))
        .sensitive(false)
        .auth(false)
        .description("Timeout in seconds for native Okta SSO authentication")
        .scopes(&[ParamScope::Connection])
        .used_at_connect(true)
        .mutable_after_connect(false)
        .build(),
    ParamDef::builder()
        .canonical_name(param_names::OKTA_USERNAME.as_str())
        .value_type(ValueType::String)
        .sensitive(false)
        .auth(false)
        .description("Okta username (defaults to the Snowflake user if omitted)")
        .scopes(&[ParamScope::Connection])
        .used_at_connect(true)
        .mutable_after_connect(false)
        .build(),
    ParamDef::builder()
        .canonical_name(param_names::DISABLE_SAML_URL_CHECK.as_str())
        .value_type(ValueType::Bool)
        .default(DefaultValue::Bool(false))
        .sensitive(false)
        .auth(false)
        .description("Skip the Okta SAML URL host-match safety check")
        .scopes(&[ParamScope::Connection])
        .used_at_connect(true)
        .mutable_after_connect(false)
        .build(),
    // ── OAuth ───────────────────────────────────────────────────────────
    // Cross-driver canonical naming follows JDBC `SFSessionProperty.OAUTH_*`.
    // All OAuth params are connect-time and immutable for the life of the
    // connection.
    //
    // The camelCase `oauth*` aliases below — and `allowUnderscoresInHost` — are
    // also rewritten to these canonical names Java-side, by the JDBC bridge's
    // `ParameterKeyNormalizer.LEGACY_KEY_ALIASES`, which
    // `SnowflakeConnectionImpl.setOptions` applies to every key. sf_core
    // therefore sees the camelCase spelling only from a direct `resolve_for`
    // caller, never from a real JDBC connection; the aliases stay so the
    // registry remains an accurate record of what JDBC accepts until that
    // mapping moves wrapper-side wholesale. The other `Jdbc`-scoped aliases
    // (`clientStoreTemporaryCredential`, `enablePutGet`,
    // `oauthEnableSingleUseRefreshTokens`, `PRIVATE_KEY_*`) have no Java-side
    // entry and resolve here only.
    ParamDef::builder()
        .canonical_name(param_names::OAUTH_CLIENT_ID.as_str())
        // `OAUTH_CLIENT_ID` is the canonical spelling (case-insensitive match);
        // the camelCase form is the JDBC-only `SFSessionProperty` key.
        .aliases(aliases![Jdbc; "oauthClientId"])
        .value_type(ValueType::String)
        .sensitive(false)
        .auth(true)
        .description("OAuth client identifier (LOCAL_APPLICATION when Snowflake is the IdP)")
        .scopes(&[ParamScope::Connection])
        .used_at_connect(true)
        .mutable_after_connect(false)
        .build(),
    ParamDef::builder()
        .canonical_name(param_names::OAUTH_CLIENT_SECRET.as_str())
        .aliases(aliases![Jdbc; "oauthClientSecret"])
        .value_type(ValueType::String)
        .sensitive(true)
        .auth(true)
        .description("OAuth client secret (redacted from logs)")
        .scopes(&[ParamScope::Connection])
        .used_at_connect(true)
        .mutable_after_connect(false)
        .build(),
    ParamDef::builder()
        .canonical_name(param_names::OAUTH_AUTHORIZATION_URL.as_str())
        .aliases(aliases![Jdbc; "oauthAuthorizationUrl"])
        .value_type(ValueType::String)
        .sensitive(false)
        .auth(true)
        .description("IdP authorization endpoint (defaults to https://{host}/oauth/authorize)")
        .scopes(&[ParamScope::Connection])
        .used_at_connect(true)
        .mutable_after_connect(false)
        .build(),
    ParamDef::builder()
        .canonical_name(param_names::OAUTH_TOKEN_REQUEST_URL.as_str())
        // `OAUTH_TOKEN_REQUEST_URL` matches the canonical name case-insensitively,
        // which is also the legacy Python kwarg and the legacy ODBC DSN key
        // (`Snowflake.h` `SF_OAUTH_TOKEN_REQUEST_URL_KEY`). Only the camelCase
        // JDBC property needs an alias; the shorter `OAUTH_TOKEN_URL` was UD-only
        // leniency (no such kwarg in the legacy connector) and is gone.
        .aliases(aliases![Jdbc; "oauthTokenRequestUrl"])
        .value_type(ValueType::String)
        .required(Required::WhenAuthMethod("OAUTH_CLIENT_CREDENTIALS"))
        .sensitive(false)
        .auth(true)
        .description("IdP token endpoint (CC only; defaults to https://{host}/oauth/token-request for AC)")
        .scopes(&[ParamScope::Connection])
        .used_at_connect(true)
        .mutable_after_connect(false)
        .build(),
    ParamDef::builder()
        .canonical_name(param_names::OAUTH_REDIRECT_URI.as_str())
        .aliases(aliases![Jdbc; "oauthRedirectUri"])
        .value_type(ValueType::String)
        .sensitive(false)
        .auth(true)
        .description("Loopback redirect URI advertised to the IdP (defaults to http://127.0.0.1:<random>)")
        .scopes(&[ParamScope::Connection])
        .used_at_connect(true)
        .mutable_after_connect(false)
        .build(),
    ParamDef::builder()
        .canonical_name(param_names::OAUTH_SCOPE.as_str())
        .aliases(aliases![Jdbc; "oauthScope"])
        .value_type(ValueType::String)
        .sensitive(false)
        .auth(true)
        .description("OAuth scope (space-separated; defaults to session:role:<role>)")
        .scopes(&[ParamScope::Connection])
        .used_at_connect(true)
        .mutable_after_connect(false)
        .build(),
    ParamDef::builder()
        .canonical_name(param_names::OAUTH_ENABLE_SINGLE_USE_REFRESH_TOKENS.as_str())
        .aliases(aliases![Jdbc; "oauthEnableSingleUseRefreshTokens"])
        .value_type(ValueType::Bool)
        .default(DefaultValue::Bool(false))
        .sensitive(false)
        .auth(true)
        .description("Request single-use refresh-token rotation (Snowflake-IdP only)")
        .scopes(&[ParamScope::Connection])
        .used_at_connect(true)
        .mutable_after_connect(false)
        .build(),
    ParamDef::builder()
        .canonical_name(param_names::OAUTH_DISABLE_PKCE.as_str())
        .value_type(ValueType::Bool)
        .default(DefaultValue::Bool(false))
        .sensitive(false)
        .auth(true)
        .description("Disable PKCE S256 challenge for OAUTH_AUTHORIZATION_CODE (Python-compatible escape hatch)")
        .scopes(&[ParamScope::Connection])
        .used_at_connect(true)
        .mutable_after_connect(false)
        .build(),
    ParamDef::builder()
        .canonical_name(param_names::OAUTH_ENABLE_DPOP.as_str())
        .value_type(ValueType::Bool)
        .default(DefaultValue::Bool(false))
        .sensitive(false)
        .auth(true)
        .description("Enable RFC 9449 DPoP proof-of-possession (JDBC-compatible)")
        .scopes(&[ParamScope::Connection])
        .used_at_connect(true)
        .mutable_after_connect(false)
        .build(),
    ParamDef::builder()
        .canonical_name(param_names::OAUTH_CREDENTIALS_IN_BODY.as_str())
        .value_type(ValueType::Bool)
        .default(DefaultValue::Bool(false))
        .sensitive(false)
        .auth(true)
        .description("Send client_id/client_secret in the OAUTH_CLIENT_CREDENTIALS token request body (client_secret_post) instead of the HTTP Basic header")
        .scopes(&[ParamScope::Connection])
        .used_at_connect(true)
        .mutable_after_connect(false)
        .build(),
    ParamDef::builder()
        .canonical_name(param_names::OAUTH_DISABLE_CONSOLE_LOGIN.as_str())
        .value_type(ValueType::Bool)
        .default(DefaultValue::Bool(false))
        .sensitive(false)
        .auth(true)
        .description("Disable EXTERNALBROWSER console-login (JDBC parity; does not gate OAuth)")
        .scopes(&[ParamScope::Connection])
        .used_at_connect(true)
        .mutable_after_connect(false)
        .build(),
    // ── Session ─────────────────────────────────────────────────────────
    ParamDef::builder()
        .canonical_name(param_names::DATABASE.as_str())
        .value_type(ValueType::String)
        .sensitive(false)
        .auth(false)
        .description("Default database to use")
        .scopes(&[ParamScope::Session])
        .used_at_connect(true)
        .mutable_after_connect(true)
        .build(),
    ParamDef::builder()
        .canonical_name(param_names::SCHEMA.as_str())
        .value_type(ValueType::String)
        .sensitive(false)
        .auth(false)
        .description("Default schema to use")
        .scopes(&[ParamScope::Session])
        .used_at_connect(true)
        .mutable_after_connect(true)
        .build(),
    ParamDef::builder()
        .canonical_name(param_names::WAREHOUSE.as_str())
        .value_type(ValueType::String)
        .sensitive(false)
        .auth(false)
        .description("Default warehouse to use")
        .scopes(&[ParamScope::Session])
        .used_at_connect(true)
        .mutable_after_connect(true)
        .build(),
    ParamDef::builder()
        .canonical_name(param_names::ROLE.as_str())
        .value_type(ValueType::String)
        .sensitive(false)
        .auth(false)
        .description("Default role to use")
        .scopes(&[ParamScope::Session])
        .used_at_connect(true)
        .mutable_after_connect(true)
        .build(),
    ParamDef::builder()
        .canonical_name(param_names::SECONDARY_ROLES.as_str())
        // Legacy ODBC's `SecondaryRoles` connection attribute has no separator,
        // so the connection-string parser uppercases it to `SECONDARYROLES`
        // (not `SECONDARY_ROLES`); scope that spelling to ODBC so the wrapper
        // canonicalizes it to `secondary_roles`.
        .aliases(aliases![Odbc; "SECONDARYROLES"])
        .value_type(ValueType::String)
        .sensitive(false)
        .auth(false)
        .description("Secondary-roles activation mode sent at login (e.g. ALL or NONE)")
        .scopes(&[ParamScope::Connection])
        .used_at_connect(true)
        .mutable_after_connect(false)
        .build(),
    // ── TLS ─────────────────────────────────────────────────────────────
    ParamDef::builder()
        .canonical_name(param_names::CUSTOM_ROOT_STORE_PATH.as_str())
        // No alias: legacy ODBC had no custom-root-store DSN key (only `SSL`), and
        // the `TLS_`-prefixed spelling had no users — the canonical name resolves
        // for every wrapper case-insensitively.
        .value_type(ValueType::String)
        .sensitive(false)
        .auth(false)
        .description("Path to custom root certificate store")
        .scopes(&[ParamScope::Connection])
        .used_at_connect(true)
        .mutable_after_connect(false)
        .build(),
    ParamDef::builder()
        .canonical_name(param_names::EXTRA_ROOT_STORE_PATH.as_str())
        .value_type(ValueType::String)
        .sensitive(false)
        .auth(false)
        .description("Path to root certificates added to the default root store")
        .scopes(&[ParamScope::Connection])
        .used_at_connect(true)
        .mutable_after_connect(false)
        .visible_to(visible_to!(NodeJs))
        .build(),
    ParamDef::builder()
        .canonical_name(param_names::VERIFY_HOSTNAME.as_str())
        // No alias: no legacy TLS-verification DSN key existed, and the
        // `TLS_VERIFY_HOSTNAME` spelling had no users.
        .value_type(ValueType::Bool)
        .default(DefaultValue::Bool(true))
        .sensitive(false)
        .auth(false)
        .description("Whether to verify the server hostname in TLS")
        .scopes(&[ParamScope::Connection])
        .used_at_connect(true)
        .mutable_after_connect(false)
        .build(),
    ParamDef::builder()
        .canonical_name(param_names::VERIFY_CERTIFICATES.as_str())
        // No alias: see `verify_hostname` above.
        .value_type(ValueType::Bool)
        .default(DefaultValue::Bool(true))
        .sensitive(false)
        .auth(false)
        .description("Whether to verify TLS certificates")
        .scopes(&[ParamScope::Connection])
        .used_at_connect(true)
        .mutable_after_connect(false)
        .build(),
    ParamDef::builder()
        .canonical_name(param_names::TLS_SKIP_VERIFY.as_str())
        .value_type(ValueType::Bool)
        .default(DefaultValue::Bool(false))
        .sensitive(false)
        .auth(false)
        .description("Skip all TLS verification with a single switch: disables both certificate and hostname checks (and, since certificate verification is off, CRL revocation checks are bypassed too). Insecure; intended for testing only")
        .scopes(&[ParamScope::Connection])
        .used_at_connect(true)
        .mutable_after_connect(false)
        .build(),
    // TLS protocol-version window.
    ParamDef::builder()
        .canonical_name(param_names::MIN_TLS_VERSION.as_str())
        .value_type(ValueType::String)
        .default(DefaultValue::String("tls12"))
        .sensitive(false)
        .auth(false)
        .description("Minimum TLS protocol version to negotiate (tls12 or tls13)")
        .scopes(&[ParamScope::Connection])
        .used_at_connect(true)
        .mutable_after_connect(false)
        .build(),
    ParamDef::builder()
        .canonical_name(param_names::MAX_TLS_VERSION.as_str())
        .value_type(ValueType::String)
        .default(DefaultValue::String("tls13"))
        .sensitive(false)
        .auth(false)
        .description("Maximum TLS protocol version to negotiate (tls12 or tls13)")
        .scopes(&[ParamScope::Connection])
        .used_at_connect(true)
        .mutable_after_connect(false)
        .build(),
    // ── CRL ─────────────────────────────────────────────────────────────
    ParamDef::builder()
        .canonical_name(param_names::CRL_CHECK_MODE.as_str())
        // UD-ODBC's own DSN spellings for CRL checking (legacy snowflake-odbc
        // spelled this family `CRL_CHECK` / `CRL_ADVISORY`, `Snowflake.h:197-202`;
        // UD has not adopted those names). Wired wrapper-side for value
        // normalization and exercised by `odbc_tests/tests/e2e/tls/crl_enabled.cpp`.
        // Python uses the `cert_revocation_check_mode` legacy kwarg, rewritten
        // wrapper-side by `_LEGACY_REWRITES`.
        .aliases(aliases![Odbc; "CRL_MODE", "CRL_ENABLED"])
        // Free-form string in core: the ODBC wrapper maps its `CRL_MODE` /
        // `CRL_ENABLED` wire spellings to the `DISABLED` / `ENABLED` / `ADVISORY`
        // tokens that `build_crl_config` accepts before the value reaches core.
        .value_type(ValueType::String)
        .default(DefaultValue::String("DISABLED"))
        .sensitive(false)
        .auth(false)
        .description("Certificate revocation check mode (DISABLED, ENABLED, ADVISORY)")
        .scopes(&[ParamScope::Connection])
        .used_at_connect(true)
        .mutable_after_connect(false)
        .build(),
    ParamDef::builder()
        .canonical_name(param_names::CRL_ENABLE_DISK_CACHING.as_str())
        .value_type(ValueType::Bool)
        .default(DefaultValue::Bool(true))
        .sensitive(false)
        .auth(false)
        .description("Enable disk caching for CRL responses")
        .scopes(&[ParamScope::Connection])
        .used_at_connect(true)
        .mutable_after_connect(false)
        .build(),
    ParamDef::builder()
        .canonical_name(param_names::CRL_ENABLE_MEMORY_CACHING.as_str())
        .value_type(ValueType::Bool)
        .default(DefaultValue::Bool(true))
        .sensitive(false)
        .auth(false)
        .description("Enable in-memory caching for CRL responses")
        .scopes(&[ParamScope::Connection])
        .used_at_connect(true)
        .mutable_after_connect(false)
        .build(),
    ParamDef::builder()
        .canonical_name(param_names::CRL_CACHE_DIR.as_str())
        .value_type(ValueType::String)
        .sensitive(false)
        .auth(false)
        .description("Directory for CRL cache files")
        .scopes(&[ParamScope::Connection])
        .used_at_connect(true)
        .mutable_after_connect(false)
        .build(),
    ParamDef::builder()
        .canonical_name(param_names::CRL_MAX_DOWNLOAD_SIZE.as_str())
        .value_type(ValueType::Int)
        .default(DefaultValue::Int(20 * 1024 * 1024))
        .sensitive(false)
        .auth(false)
        .description("Maximum CRL download size in bytes before the download is aborted")
        .scopes(&[ParamScope::Connection])
        .used_at_connect(true)
        .mutable_after_connect(false)
        .build(),
    ParamDef::builder()
        .canonical_name(param_names::CRL_VALIDITY_TIME.as_str())
        .value_type(ValueType::Int)
        .default(DefaultValue::Int(86400))
        .sensitive(false)
        .auth(false)
        .description("Maximum age in seconds of a cached CRL before it is re-fetched")
        .scopes(&[ParamScope::Connection])
        .used_at_connect(true)
        .mutable_after_connect(false)
        .build(),
    ParamDef::builder()
        .canonical_name(param_names::CRL_ON_DISK_CACHE_REMOVAL_DELAY.as_str())
        .value_type(ValueType::Int)
        .default(DefaultValue::Int(604800))
        .sensitive(false)
        .auth(false)
        .description("Delay in seconds after a CRL's nextUpdate before it is purged from the on-disk cache")
        .scopes(&[ParamScope::Connection])
        .used_at_connect(true)
        .mutable_after_connect(false)
        .build(),
    ParamDef::builder()
        .canonical_name(param_names::CRL_CACHE_CLEANUP_INTERVAL.as_str())
        .value_type(ValueType::Int)
        .default(DefaultValue::Int(3600))
        .sensitive(false)
        .auth(false)
        .description("Interval in seconds between background CRL cache cleanup passes")
        .scopes(&[ParamScope::Connection])
        .used_at_connect(true)
        .mutable_after_connect(false)
        .build(),
    ParamDef::builder()
        .canonical_name(param_names::CRL_CACHE_START_CLEANUP.as_str())
        .value_type(ValueType::Bool)
        .default(DefaultValue::Bool(false))
        .sensitive(false)
        .auth(false)
        .description("Run the background CRL cache cleanup task")
        .scopes(&[ParamScope::Connection])
        .used_at_connect(true)
        .mutable_after_connect(false)
        .build(),
    ParamDef::builder()
        .canonical_name(param_names::CRL_UNSAFE_SKIP_FILE_PERMISSIONS_CHECK.as_str())
        .value_type(ValueType::Bool)
        .default(DefaultValue::Bool(false))
        .sensitive(false)
        .auth(false)
        .description("Skip verification that on-disk CRL cache files and directory are owner-only (0600/0700)")
        .scopes(&[ParamScope::Connection])
        .used_at_connect(true)
        .mutable_after_connect(false)
        .build(),
    ParamDef::builder()
        .canonical_name(param_names::CRL_ALLOW_CERTIFICATES_WITHOUT_CRL_URL.as_str())
        .value_type(ValueType::Bool)
        .default(DefaultValue::Bool(false))
        .sensitive(false)
        .auth(false)
        .description("Allow certificates that do not include a CRL distribution URL")
        .scopes(&[ParamScope::Connection])
        .used_at_connect(true)
        .mutable_after_connect(false)
        .build(),
    ParamDef::builder()
        .canonical_name(param_names::CRL_HTTP_TIMEOUT.as_str())
        .value_type(ValueType::Int)
        .default(DefaultValue::Int(10))
        .sensitive(false)
        .auth(false)
        .description("HTTP timeout in seconds for CRL endpoint requests")
        .scopes(&[ParamScope::Connection])
        .used_at_connect(true)
        .mutable_after_connect(false)
        .build(),
    ParamDef::builder()
        .canonical_name(param_names::CRL_CONNECTION_TIMEOUT.as_str())
        .value_type(ValueType::Int)
        .default(DefaultValue::Int(10))
        .sensitive(false)
        .auth(false)
        .description("Connection timeout in seconds for CRL endpoints")
        .scopes(&[ParamScope::Connection])
        .used_at_connect(true)
        .mutable_after_connect(false)
        .build(),
    // ── Client ──────────────────────────────────────────────────────────
    ParamDef::builder()
        .canonical_name(param_names::CONNECTION_NAME.as_str())
        .value_type(ValueType::String)
        .sensitive(false)
        .auth(false)
        .description("Named connection to load from TOML configuration files")
        .scopes(&[ParamScope::Connection])
        .used_at_connect(false)
        .mutable_after_connect(false)
        .build(),
    ParamDef::builder()
        .canonical_name(param_names::LOG_MAX_QUERY_LENGTH.as_str())
        .value_type(ValueType::Int)
        .default(DefaultValue::Int(80))
        .sensitive(false)
        .auth(false)
        .description("Maximum number of characters of a query string to include in log messages")
        .scopes(&[ParamScope::Connection])
        .used_at_connect(false)
        .mutable_after_connect(false)
        .build(),
    ParamDef::builder()
        .canonical_name(param_names::LOG_QUERY_TEXT.as_str())
        .value_type(ValueType::Bool)
        .additional_value_type(ValueType::String)
        .default(DefaultValue::Bool(false))
        .sensitive(false)
        .auth(false)
        .description("Include the (truncated) SQL text in INFO query logs")
        .scopes(&[ParamScope::Connection])
        .used_at_connect(false)
        .mutable_after_connect(false)
        .build(),
    ParamDef::builder()
        .canonical_name(param_names::LOG_QUERY_PARAMETERS.as_str())
        .value_type(ValueType::Bool)
        .additional_value_type(ValueType::String)
        .default(DefaultValue::Bool(false))
        .sensitive(false)
        .auth(false)
        .description("Include the (truncated) JSON bindings in INFO query logs (requires log_query_text)")
        .scopes(&[ParamScope::Connection])
        .used_at_connect(false)
        .mutable_after_connect(false)
        .build(),
    // ── Logout ────────────────────────────────────────────────────────
    ParamDef::builder()
        .canonical_name(param_names::SERVER_SESSION_KEEP_ALIVE.as_str())
        .value_type(ValueType::Bool)
        .sensitive(false)
        .auth(false)
        .description("Control server session lifecycle: true=keep alive, false=always logout, null=auto-detect")
        .scopes(&[ParamScope::Connection])
        .used_at_connect(false)
        .mutable_after_connect(false)
        .build(),
    ParamDef::builder()
        .canonical_name(param_names::ENABLE_SERVER_SESSION_KEEP_ALIVE_AUTO_DETECTION.as_str())
        .value_type(ValueType::Bool)
        .sensitive(false)
        .auth(false)
        .description("Enable auto-detection of async queries before logout (SNOW-2314152)")
        .scopes(&[ParamScope::Connection])
        .used_at_connect(false)
        .mutable_after_connect(false)
        .build(),
    ParamDef::builder()
        .canonical_name(param_names::LOGOUT_ERROR_STRATEGY.as_str())
        .value_type(ValueType::String)
        .sensitive(false)
        .auth(false)
        .description("Error handling strategy for logout: 'best_effort' or 'strict'")
        .scopes(&[ParamScope::Connection])
        .used_at_connect(false)
        .mutable_after_connect(true)
        .build(),
    ParamDef::builder()
        .canonical_name(param_names::LOGOUT_TOTAL_TIMEOUT_SECONDS.as_str())
        .value_type(ValueType::Int)
        .sensitive(false)
        .auth(false)
        .description("Total timeout budget for logout operation including retries")
        .scopes(&[ParamScope::Connection])
        .used_at_connect(false)
        .mutable_after_connect(true)
        .build(),
    ParamDef::builder()
        .canonical_name(param_names::LOGOUT_MAX_ATTEMPTS.as_str())
        .value_type(ValueType::Int)
        .sensitive(false)
        .auth(false)
        .description("Maximum total attempts for logout (1 = no retry, 3 = 2 retries)")
        .scopes(&[ParamScope::Connection])
        .used_at_connect(false)
        .mutable_after_connect(true)
        .build(),
    ParamDef::builder()
        .canonical_name(param_names::LOGOUT_REQUEST_TIMEOUT_SECONDS.as_str())
        .value_type(ValueType::Int)
        .sensitive(false)
        .auth(false)
        .description("Per-request socket timeout for individual logout attempts")
        .scopes(&[ParamScope::Connection])
        .used_at_connect(false)
        .mutable_after_connect(true)
        .build(),
    ParamDef::builder()
        .canonical_name(param_names::RETRY_MAX_ATTEMPTS.as_str())
        .aliases(aliases![Odbc; "MaxHttpRetries"])
        .value_type(ValueType::Int)
        .default(DefaultValue::Int(DEFAULT_RETRY_MAX_ATTEMPTS as i64))
        .sensitive(false)
        .auth(false)
        .description("Maximum total attempts for general HTTP calls (login, query, logout). 1 = no retry")
        .scopes(&[ParamScope::Connection])
        .used_at_connect(false)
        .mutable_after_connect(false)
        .build(),
    ParamDef::builder()
        .canonical_name(param_names::RETRY_EXTRA_STATUS_CODES.as_str())
        .value_type(ValueType::String)
        .sensitive(false)
        .auth(false)
        .description("Additional HTTP status codes (comma-separated) to retry on general HTTP and PUT/GET calls, beyond the built-in 408/429/307/308/5xx set")
        .scopes(&[ParamScope::Connection])
        .used_at_connect(false)
        .mutable_after_connect(false)
        .build(),
    ParamDef::builder()
        .canonical_name(param_names::PUT_GET_MAX_ATTEMPTS.as_str())
        // ODBC-only 3.x spellings (`Snowflake.h`). The ODBC wrapper resolves conflicts:
        // `PUT_GET_MAX_ATTEMPTS` wins when present; otherwise the maximum of any
        // supplied legacy alias values is used.
        .aliases(aliases![Odbc; "PUT_MAXRETRIES", "GET_MAXRETRIES"])
        .value_type(ValueType::Int)
        .default(DefaultValue::Int(DEFAULT_PUT_GET_MAX_ATTEMPTS as i64))
        .sensitive(false)
        .auth(false)
        .description("Maximum total attempts for a single PUT/GET file transfer (1 = no retry)")
        .scopes(&[ParamScope::Connection])
        .used_at_connect(false)
        .mutable_after_connect(true)
        .build(),
    // ── Retry backoff curve (shared by HTTP and PUT/GET pipelines) ──────
    ParamDef::builder()
        .canonical_name(param_names::RETRY_BACKOFF_BASE_MS.as_str())
        .value_type(ValueType::Int)
        .default(DefaultValue::Int(DEFAULT_RETRY_BACKOFF_BASE_MS as i64))
        .sensitive(false)
        .auth(false)
        .description("Initial exponential-backoff delay in milliseconds between retry attempts")
        .scopes(&[ParamScope::Connection])
        .used_at_connect(false)
        .mutable_after_connect(false)
        .build(),
    ParamDef::builder()
        .canonical_name(param_names::RETRY_BACKOFF_CAP_MS.as_str())
        .value_type(ValueType::Int)
        .default(DefaultValue::Int(DEFAULT_RETRY_BACKOFF_CAP_MS as i64))
        .sensitive(false)
        .auth(false)
        .description("Maximum exponential-backoff delay in milliseconds between retry attempts")
        .scopes(&[ParamScope::Connection])
        .used_at_connect(false)
        .mutable_after_connect(false)
        .build(),
    ParamDef::builder()
        .canonical_name(param_names::RETRY_BACKOFF_FACTOR.as_str())
        .value_type(ValueType::Double)
        .default(DefaultValue::Double(DEFAULT_RETRY_BACKOFF_FACTOR))
        .sensitive(false)
        .auth(false)
        .description("Multiplier applied to the backoff delay after each retry attempt")
        .scopes(&[ParamScope::Connection])
        .used_at_connect(false)
        .mutable_after_connect(false)
        .build(),
    ParamDef::builder()
        .canonical_name(param_names::RETRY_BACKOFF_JITTER.as_str())
        .value_type(ValueType::String)
        .default(DefaultValue::String(DEFAULT_RETRY_BACKOFF_JITTER))
        .sensitive(false)
        .auth(false)
        .description("Backoff jitter strategy: 'none', 'full', or 'decorrelated'")
        .scopes(&[ParamScope::Connection])
        .used_at_connect(false)
        .mutable_after_connect(false)
        .build(),
    // ── Timeout configuration ─────────────────────────────────────────
    ParamDef::builder()
        .canonical_name(param_names::LOGIN_TIMEOUT.as_str())
        // `LOGIN_TIMEOUT` matches this canonical case-insensitively for every
        // wrapper except ODBC, where it is scoped to `authentication_timeout`
        // (see that param's `Alias::scoped(Wrapper::Odbc, "LOGIN_TIMEOUT")`).
        .value_type(ValueType::Int)
        .default(DefaultValue::Int(DEFAULT_LOGIN_TIMEOUT_SECS as i64))
        .sensitive(false)
        .auth(false)
        .description("Wall-clock timeout in seconds for the entire login operation including retries (0 = no timeout)")
        .scopes(&[ParamScope::Connection])
        .used_at_connect(true)
        .mutable_after_connect(false)
        .build(),
    ParamDef::builder()
        .canonical_name(param_names::QUERY_TIMEOUT.as_str())
        .value_type(ValueType::Int)
        .default(DefaultValue::Int(DEFAULT_QUERY_TIMEOUT_SECS as i64))
        .sensitive(false)
        .auth(false)
        .description("Wall-clock timeout in seconds for query execution including retries (0 = no timeout)")
        .scopes(&[ParamScope::Connection])
        .used_at_connect(true)
        .mutable_after_connect(false)
        .build(),
    ParamDef::builder()
        .canonical_name(param_names::REQUEST_TIMEOUT.as_str())
        .value_type(ValueType::Int)
        .default(DefaultValue::Int(DEFAULT_REQUEST_TIMEOUT_SECS as i64))
        .sensitive(false)
        .auth(false)
        .description("Wall-clock timeout in seconds for all other operations (close session, heartbeat, etc.) including retries (0 = no timeout)")
        .scopes(&[ParamScope::Connection])
        .used_at_connect(false)
        .mutable_after_connect(false)
        .build(),
    ParamDef::builder()
        .canonical_name(param_names::RETRY_TIMEOUT.as_str())
        .value_type(ValueType::Int)
        .sensitive(false)
        .auth(false)
        .description("Per-request timeout in seconds for a single HTTP attempt within a retry loop (0 or absent = no per-request timeout)")
        .scopes(&[ParamScope::Connection])
        .used_at_connect(true)
        .mutable_after_connect(false)
        .build(),
    ParamDef::builder()
        .canonical_name(param_names::CONNECT_TIMEOUT.as_str())
        .value_type(ValueType::Int)
        .sensitive(false)
        .auth(false)
        .description("TCP connect timeout in seconds for the HTTP client (0 or absent = system default)")
        .scopes(&[ParamScope::Connection])
        .used_at_connect(true)
        .mutable_after_connect(false)
        .build(),
    ParamDef::builder()
        .canonical_name(param_names::UNSAFE_SKIP_CONFIG_FILE_PERMISSIONS_CHECK.as_str())
        .value_type(ValueType::Bool)
        .default(DefaultValue::Bool(false))
        .sensitive(false)
        .auth(false)
        .description("When true, skip file permission checks on config.toml and connections.toml \
                      during connection setup. Use in environments where permissions cannot be \
                      controlled (CI runners, containers). Unix-only; ignored on Windows")
        .scopes(&[ParamScope::Connection])
        .used_at_connect(true)
        .mutable_after_connect(false)
        .build(),
    ParamDef::builder()
        .canonical_name(param_names::UNSAFE_FILE_WRITE.as_str())
        .value_type(ValueType::Bool)
        .default(DefaultValue::Bool(false))
        .sensitive(false)
        .auth(false)
        .description("When true, GET downloads use the process umask permissions instead of owner-only \
                      (0600). Unix-only; ignored on Windows")
        .scopes(&[ParamScope::Connection])
        .used_at_connect(false)
        .mutable_after_connect(true)
        .build(),
    ParamDef::builder()
        .canonical_name(param_names::CLIENT_APP_ID.as_str())
        .value_type(ValueType::String)
        .sensitive(false)
        .auth(false)
        .description("Driver identity sent as CLIENT_APP_ID in the login request (e.g. PythonConnector, SnowSQL)")
        .scopes(&[ParamScope::Connection])
        .used_at_connect(false)
        .mutable_after_connect(false)
        .build(),
    ParamDef::builder()
        .canonical_name(param_names::CLIENT_APP_VERSION.as_str())
        .value_type(ValueType::String)
        .sensitive(false)
        .auth(false)
        .description("Driver version sent as CLIENT_APP_VERSION in the login request")
        .scopes(&[ParamScope::Connection])
        .used_at_connect(false)
        .mutable_after_connect(false)
        .build(),
    ParamDef::builder()
        .canonical_name(param_names::APPLICATION.as_str())
        .value_type(ValueType::String)
        .sensitive(false)
        .auth(false)
        .description("User-facing application name sent as CLIENT_ENVIRONMENT.APPLICATION (falls back to client_app_id)")
        .scopes(&[ParamScope::Connection])
        .used_at_connect(false)
        .mutable_after_connect(false)
        .build(),
    // ── Statement ──────────────────────────────────────────────────────
    ParamDef::builder()
        .canonical_name(param_names::ASYNC_EXECUTION.as_str())
        .value_type(ValueType::Bool)
        .default(DefaultValue::Bool(false))
        .sensitive(false)
        .auth(false)
        .description("Execute queries asynchronously")
        .scopes(&[ParamScope::Statement])
        .used_at_connect(false)
        .mutable_after_connect(true)
        .build(),
    ParamDef::builder()
        .canonical_name(param_names::MULTI_STATEMENT_COUNT.as_str())
        .value_type(ValueType::Int)
        .sensitive(false)
        .auth(false)
        .description("Exact number of statements in a multi-statement query")
        .scopes(&[ParamScope::Statement])
        .used_at_connect(false)
        .mutable_after_connect(true)
        .build(),
    ParamDef::builder()
        .canonical_name(param_names::QUERY_TAG.as_str())
        .value_type(ValueType::String)
        .sensitive(false)
        .auth(false)
        .description("String label attached to queries and surfaced in QUERY_HISTORY. \
                      Settable at the session level (connection option or session override, \
                      forwarded as a login session parameter) and overridable per-statement.")
        // A session parameter that may also be overridden per-statement.
        .scopes(&[ParamScope::Session, ParamScope::Statement])
        .used_at_connect(false)
        .mutable_after_connect(true)
        .build(),
    ParamDef::builder()
        .canonical_name(param_names::SKIP_UPLOAD_ON_CONTENT_MATCH.as_str())
        .value_type(ValueType::Bool)
        .default(DefaultValue::Bool(false))
        .sensitive(false)
        .auth(false)
        .description("Skip re-uploading a PUT object when the remote stored digest (S3 x-amz-meta-sfc-digest / Azure x-ms-meta-sfcdigest / GCS x-goog-meta-sfc-digest) equals the local SHA-256. Optimization for racing concurrent uploaders; only meaningful when overwrite=true. Set per-statement via statement_set_options before each execute. Client-only, never forwarded to GS.")
        .scopes(&[ParamScope::Statement])
        .used_at_connect(false)
        .mutable_after_connect(true)
        .build(),
    ParamDef::builder()
        .canonical_name(param_names::CWD.as_str())
        .value_type(ValueType::String)
        .sensitive(false)
        .auth(false)
        .description("Directory used to resolve relative local paths in PUT and GET. Set per-statement via statement_set_options before each execute. Client-only, never forwarded to GS.")
        .scopes(&[ParamScope::Statement])
        .used_at_connect(false)
        .mutable_after_connect(true)
        .build(),
    ParamDef::builder()
        .canonical_name(param_names::PUT_FASTFAIL.as_str())
        .value_type(ValueType::Bool)
        // No registry default: unset must resolve to `None` so the dispatch
        // site can fall back to `WrapperPresets::put_get_fastfail_default`
        // (true for Python/JDBC, false for ODBC) instead of a fixed value.
        .sensitive(false)
        .auth(false)
        .description("Controls whether a PUT batch stops at the first failing file (true, fail-fast) or attempts every file and reports failures as ERROR-status rows in the result set (false, collect-all). Defaults to the active wrapper's preset when unset. Mirrors old ODBC's PUT_FASTFAIL connection attribute. Set per-statement via statement_set_options before each execute. Client-only, never forwarded to GS.")
        .scopes(&[
            ParamScope::Connection,
            ParamScope::Session,
            ParamScope::Statement,
        ])
        .used_at_connect(false)
        .mutable_after_connect(true)
        .visible_to(visible_to!(Odbc))
        .build(),
    ParamDef::builder()
        .canonical_name(param_names::PUT_COMPRESS_LEVEL.as_str())
        .aliases(aliases![Odbc; "PUT_COMPRESSLV"])
        .value_type(ValueType::Int)
        // No registry default: unset must resolve to `None` so the dispatch
        // site can fall back to `WrapperPresets::put_compress_level_default`.
        .sensitive(false)
        .auth(false)
        .description("Gzip compression level (0–9) used when PUT AUTO_COMPRESS rewrites a file. Unset and out-of-range values use gzip default preset level. Client-only, never forwarded to GS.")
        .scopes(&[ParamScope::Connection])
        .used_at_connect(false)
        .mutable_after_connect(true)
        .visible_to(visible_to!(Odbc))
        .build(),
    ParamDef::builder()
        .canonical_name(param_names::PUT_TEMPDIR.as_str())
        .aliases(aliases![Odbc; "PUT_TEMPDIR"])
        .value_type(ValueType::String)
        .sensitive(false)
        .auth(false)
        .description("Directory used for gzip tempfiles created by PUT AUTO_COMPRESS. Unset uses the process temp directory. Nested directories are created. Mirrors old ODBC's PUT_TEMPDIR connection attribute. Client-only, never forwarded to GS.")
        .scopes(&[ParamScope::Connection])
        .used_at_connect(false)
        .mutable_after_connect(true)
        .visible_to(visible_to!(Odbc))
        .build(),
    ParamDef::builder()
        .canonical_name(param_names::GET_FASTFAIL.as_str())
        .value_type(ValueType::Bool)
        // See PUT_FASTFAIL above: `None` is load-bearing, not an oversight.
        .sensitive(false)
        .auth(false)
        .description("Controls whether a GET batch stops at the first failing file (true, fail-fast) or attempts every file and reports failures as ERROR-status rows in the result set (false, collect-all). Defaults to the active wrapper's preset when unset. Mirrors old ODBC's GET_FASTFAIL connection attribute. Set per-statement via statement_set_options before each execute. Client-only, never forwarded to GS.")
        .scopes(&[
            ParamScope::Connection,
            ParamScope::Session,
            ParamScope::Statement,
        ])
        .used_at_connect(false)
        .mutable_after_connect(true)
        .visible_to(visible_to!(Odbc))
        .build(),
    // ── Prefetch ───────────────────────────────────────────────────────
    ParamDef::builder()
        .canonical_name(param_names::CLIENT_PREFETCH_THREADS.as_str())
        .value_type(ValueType::Int)
        .default(DefaultValue::Int(4))
        .sensitive(false)
        .auth(false)
        .description("Number of concurrent chunk prefetch threads for result set downloading")
        .scopes(&[ParamScope::Session])
        .used_at_connect(true)
        .mutable_after_connect(true)
        .build(),
    ParamDef::builder()
        .canonical_name(param_names::CLIENT_MEMORY_LIMIT.as_str())
        .value_type(ValueType::Int)
        .default(DefaultValue::Int(1536))
        .sensitive(false)
        .auth(false)
        .description("Memory budget in MB for chunk prefetch buffer (0 = unlimited)")
        .scopes(&[ParamScope::Session])
        .used_at_connect(false)
        .mutable_after_connect(true)
        .build(),
    // ── Session keep-alive ─────────────────────────────────────────────
    ParamDef::builder()
        .canonical_name(param_names::CLIENT_SESSION_KEEP_ALIVE.as_str())
        .value_type(ValueType::Bool)
        .default(DefaultValue::Bool(false))
        .sensitive(false)
        .auth(false)
        .description("Keep the session alive with periodic heartbeat requests")
        .scopes(&[ParamScope::Session])
        .used_at_connect(true)
        .mutable_after_connect(false)
        .build(),
    ParamDef::builder()
        .canonical_name(param_names::CLIENT_SESSION_KEEP_ALIVE_HEARTBEAT_FREQUENCY.as_str())
        .value_type(ValueType::Int)
        .sensitive(false)
        .auth(false)
        .description("Heartbeat frequency in seconds (clamped to interval master_token_validity/16..master_token_validity/4)")
        .scopes(&[ParamScope::Session])
        .used_at_connect(true)
        .mutable_after_connect(false)
        .build(),
    // ── PUT/GET — S3 regional endpoint ─────────────────────────────────
    //
    // Forces the regional S3 endpoint (`s3.<region>.amazonaws.com[.cn]`) for
    // PUT/GET. Mirrors the OR-with-stage-info-flags semantics that the
    // Python connector, snowflake-jdbc, and libsnowflakeclient all implement.
    //
    // `ENABLE_STAGE_S3_PRIVATELINK_FOR_US_EAST_1` is the server-pushed
    // session-parameter key (read directly by
    // `read_use_s3_regional_url_session_param`, not via the registry).
    // As a connection alias it is the legacy Python kwarg name
    // (`enable_stage_s3_privatelink_for_us_east_1`), so it is Python-scoped;
    // the Python wrapper additionally rewrites it via `_DEPRECATED_REWRITES`.
    ParamDef::builder()
        .canonical_name(param_names::USE_S3_REGIONAL_URL.as_str())
        .aliases(aliases![Python; "ENABLE_STAGE_S3_PRIVATELINK_FOR_US_EAST_1"])
        .value_type(ValueType::Bool)
        .default(DefaultValue::Bool(false))
        .sensitive(false)
        .auth(false)
        .description("Force the S3 regional endpoint for PUT/GET (PrivateLink-to-S3)")
        .scopes(&[ParamScope::Session])
        .used_at_connect(false)
        .mutable_after_connect(true)
        .build(),
    ParamDef::builder()
        .canonical_name(param_names::VALIDATE_DEFAULT_PARAMETERS.as_str())
        .value_type(ValueType::Bool)
        .default(DefaultValue::Bool(false))
        .sensitive(false)
        .auth(false)
        .description("Validate that the default database, schema, and warehouse exist on the server at connect time")
        .scopes(&[ParamScope::Connection])
        .used_at_connect(true)
        .mutable_after_connect(false)
        .build(),
    // ── Proxy ──────────────────────────────────────────────────────────
    ParamDef::builder()
        .canonical_name(param_names::PROXY_HOST.as_str())
        // The legacy ODBC `PROXY` DSN key uses a different *format* (full URL
        // with embedded creds), so it is registered as a distinct canonical
        // param `proxy` rather than aliased here. `build_proxy_config` parses
        // the URL and merges it with the fields below.
        .value_type(ValueType::String)
        .sensitive(false)
        .auth(false)
        .description("Proxy server hostname")
        .scopes(&[ParamScope::Connection])
        .used_at_connect(true)
        .mutable_after_connect(false)
        .build(),
    ParamDef::builder()
        .canonical_name(param_names::PROXY_PORT.as_str())
        .value_type(ValueType::Int)
        .sensitive(false)
        .auth(false)
        .description("Proxy server port")
        .scopes(&[ParamScope::Connection])
        .used_at_connect(true)
        .mutable_after_connect(false)
        .build(),
    ParamDef::builder()
        .canonical_name(param_names::PROXY_SCHEME.as_str())
        .value_type(ValueType::String)
        .sensitive(false)
        .auth(false)
        .description("Scheme for the hop to the proxy: http (default) or https")
        .scopes(&[ParamScope::Connection])
        .used_at_connect(true)
        .mutable_after_connect(false)
        .build(),
    ParamDef::builder()
        .canonical_name(param_names::PROXY_USER.as_str())
        .value_type(ValueType::String)
        .sensitive(false)
        .auth(false)
        .description("Proxy server username for Basic auth")
        .scopes(&[ParamScope::Connection])
        .used_at_connect(true)
        .mutable_after_connect(false)
        .build(),
    ParamDef::builder()
        .canonical_name(param_names::PROXY_PASSWORD.as_str())
        .value_type(ValueType::String)
        .sensitive(true)
        .auth(false)
        .description("Proxy server password for Basic auth")
        .scopes(&[ParamScope::Connection])
        .used_at_connect(true)
        .mutable_after_connect(false)
        .build(),
    ParamDef::builder()
        .canonical_name(param_names::NO_PROXY.as_str())
        // No alias: legacy ODBC's DSN key is `NO_PROXY` (`Snowflake.h`
        // `SF_NO_PROXY_KEY`) and legacy Python's kwarg is `no_proxy`, both of
        // which match the canonical name case-insensitively. The separator-less
        // `NOPROXY` was UD-only leniency and is no longer accepted.
        .value_type(ValueType::String)
        .sensitive(false)
        .auth(false)
        .description("Comma-separated list of hosts to bypass the proxy for")
        .scopes(&[ParamScope::Connection])
        .used_at_connect(true)
        .mutable_after_connect(false)
        .build(),
    // ── Workload Identity Federation (WIF) ────────────────────────────
    ParamDef::builder()
        .canonical_name(param_names::WORKLOAD_IDENTITY_PROVIDER.as_str())
        .value_type(ValueType::String)
        .required(Required::WhenAuthMethod("WORKLOAD_IDENTITY"))
        .sensitive(false)
        .auth(true)
        .description("Cloud provider for WIF attestation (AWS, AZURE, GCP, OIDC)")
        .scopes(&[ParamScope::Connection])
        .used_at_connect(true)
        .mutable_after_connect(false)
        .build(),
    ParamDef::builder()
        .canonical_name(param_names::WORKLOAD_IDENTITY_ENTRA_RESOURCE.as_str())
        .value_type(ValueType::String)
        .sensitive(false)
        .auth(true)
        .description("Azure Entra resource URI for managed-identity token (Azure only; defaults to api://fd3f753b-eed3-462c-b6a7-a4b5bb650aad)")
        .scopes(&[ParamScope::Connection])
        .used_at_connect(true)
        .mutable_after_connect(false)
        .build(),
    ParamDef::builder()
        .canonical_name(param_names::WORKLOAD_IDENTITY_IMPERSONATION_PATH.as_str())
        .value_type(ValueType::String)
        .sensitive(false)
        .auth(true)
        .description("Comma-separated impersonation chain for WIF (AWS role ARNs or GCP service account emails)")
        .scopes(&[ParamScope::Connection])
        .used_at_connect(true)
        .mutable_after_connect(false)
        .build(),
    ParamDef::builder()
        .canonical_name(param_names::WORKLOAD_IDENTITY_AWS_USE_OUTBOUND_TOKEN.as_str())
        .value_type(ValueType::Bool)
        .default(DefaultValue::Bool(false))
        .sensitive(false)
        .auth(true)
        .description("Use outbound STS GetWebIdentityToken for AWS WIF (default: pre-signed GetCallerIdentity)")
        .scopes(&[ParamScope::Connection])
        .used_at_connect(true)
        .mutable_after_connect(false)
        .build(),
    // Legacy ODBC PROXY URL form (parsed and merged with the fields above).
    ParamDef::builder()
        .canonical_name(param_names::PROXY.as_str())
        .value_type(ValueType::String)
        .sensitive(true)
        .auth(false)
        .description("Proxy URL ([scheme://][user:pass@]host[:port]); legacy ODBC `PROXY` form")
        .scopes(&[ParamScope::Connection])
        .used_at_connect(true)
        .mutable_after_connect(false)
        .build(),
    ParamDef::builder()
        .canonical_name(param_names::USE_PROXY_ENV.as_str())
        // Legacy ODBC DSN `ProxyWithEnv` (`Snowflake.h`), uppercased by the
        // connection-string parser. ODBC-only: the legacy Python connector has
        // no equivalent kwarg (it consulted the proxy env vars unconditionally).
        .aliases(aliases![Odbc; "PROXYWITHENV"])
        .value_type(ValueType::Bool)
        .default(DefaultValue::Bool(false))
        .sensitive(false)
        .auth(false)
        .description("Honour HTTP_PROXY/HTTPS_PROXY/NO_PROXY env vars when no explicit proxy is set")
        .scopes(&[ParamScope::Connection])
        .used_at_connect(true)
        .mutable_after_connect(false)
        .build(),
    ParamDef::builder()
        .canonical_name(param_names::ALLOW_EMPTY_PROXY.as_str())
        // Legacy ODBC DSN `AllowEmptyProxy` (`Snowflake.h`), uppercased by the
        // connection-string parser. ODBC-only: no legacy Python equivalent.
        .aliases(aliases![Odbc; "ALLOWEMPTYPROXY"])
        .value_type(ValueType::Bool)
        .default(DefaultValue::Bool(true))
        .sensitive(false)
        .auth(false)
        .description("Empty PROXY value explicitly disables proxy (overrides env)")
        .scopes(&[ParamScope::Connection])
        .used_at_connect(true)
        .mutable_after_connect(false)
        .build(),
    ParamDef::builder()
        .canonical_name(param_names::USE_CURRENT_CATALOG.as_str())
        // Legacy ODBC DSN `UseCurrentCatalog` (`Snowflake.h`), uppercased by
        // the connection-string parser. ODBC-only: JDBC/Python/Node have no
        // equivalent client property.
        .aliases(aliases![Odbc; "USECURRENTCATALOG"])
        .value_type(ValueType::Bool)
        .default(DefaultValue::Bool(false))
        .sensitive(false)
        .auth(false)
        .description(
            "When true, a NULL CatalogName on catalog functions is the current \
             database. When false (default), the catalog is unconstrained unless \
             CLIENT_METADATA_REQUEST_USE_CONNECTION_CTX fills it",
        )
        .scopes(&[ParamScope::Connection])
        .used_at_connect(true)
        .mutable_after_connect(false)
        .build(),
    ParamDef::builder()
        .canonical_name(param_names::ENABLE_CONNECTION_DIAG.as_str())
        .value_type(ValueType::Bool)
        // No registry default: the consumer uses `.unwrap_or(false)`.  Omitting
        // the default keeps the Python dataclass field at `None` so that a
        // TOML profile setting `enable_connection_diag = true` is not silently
        // overridden by a Python-side `False` default passed as an explicit
        // Layer-1 option.
        .sensitive(false)
        .auth(false)
        .description("Run connectivity diagnostics during connect and write a report")
        .scopes(&[ParamScope::Connection])
        .used_at_connect(true)
        .mutable_after_connect(false)
        .build(),
    ParamDef::builder()
        .canonical_name(param_names::CONNECTION_DIAG_LOG_PATH.as_str())
        .value_type(ValueType::String)
        .sensitive(false)
        .auth(false)
        .description("Directory where the diagnostic report file is written (defaults to system tmpdir)")
        .scopes(&[ParamScope::Connection])
        .used_at_connect(true)
        .mutable_after_connect(false)
        .build(),
    ParamDef::builder()
        .canonical_name(param_names::CONNECTION_DIAG_ALLOWLIST_PATH.as_str())
        .value_type(ValueType::String)
        .sensitive(false)
        .auth(false)
        .description("Path to a pre-fetched allowlist.json; if absent the driver fetches it via system$allowlist()")
        .scopes(&[ParamScope::Connection])
        .used_at_connect(true)
        .mutable_after_connect(false)
        .build(),
    // ── ODBC keywords the driver accepts and does not apply ───────────
    ignored_odbc_param(param_names::DSN.as_str()),
    ignored_odbc_param(param_names::DRIVER.as_str()),
    ignored_odbc_param(param_names::FILEDSN.as_str()),
    ignored_odbc_param(param_names::SAVEFILE.as_str()),
    ignored_odbc_param(param_names::DESCRIPTION.as_str()),
    ignored_odbc_param(param_names::LOCALE.as_str()),
    ignored_odbc_param(param_names::SETUP.as_str()),
    ignored_odbc_param(param_names::DRIVER_ODBC_VER.as_str()),
    ignored_odbc_param(param_names::API_LEVEL.as_str()),
    ignored_odbc_param(param_names::SQL_LEVEL.as_str()),
    ignored_odbc_param(param_names::CONNECT_FUNCTIONS.as_str()),
    ignored_odbc_param(param_names::TRACING.as_str()),
    // ── Deprecated ODBC connection-string keys ────────────────────────
    odbc_deprecated(
        param_names::LOG_LEVEL.as_str(),
        aliases![Odbc; "LogLevel"],
        "Legacy connection-string LogLevel. The driver accepts the key and does not apply it; file logging is configured in sf.odbc.ini",
        Deprecation::Ignored {
            guidance: "Set LogLevel in sf.odbc.ini to configure driver log verbosity.",
        },
    ),
    odbc_deprecated(
        param_names::LOG_PATH.as_str(),
        aliases![Odbc; "LogPath"],
        "Legacy connection-string LogPath. The driver accepts the key and does not apply it; file logging is configured in sf.odbc.ini",
        Deprecation::Ignored {
            guidance: "Set LogPath in sf.odbc.ini to choose the driver log directory.",
        },
    ),
    odbc_deprecated(
        param_names::LOG_FILE_SIZE.as_str(),
        aliases![Odbc; "LogFileSize"],
        "Legacy connection-string LogFileSize. The driver accepts the key and does not apply it; file logging is configured in sf.odbc.ini",
        Deprecation::Ignored {
            guidance: "Set LogMaxSize in sf.odbc.ini to cap the driver log file size.",
        },
    ),
    odbc_deprecated(
        param_names::LOG_FILE_COUNT.as_str(),
        aliases![Odbc; "LogFileCount"],
        "Legacy connection-string LogFileCount. The driver accepts the key and does not apply it; file logging is configured in sf.odbc.ini",
        Deprecation::Ignored {
            guidance: "Set LogMaxCount in sf.odbc.ini to cap how many rotated driver log files are kept.",
        },
    ),
    odbc_deprecated(
        param_names::CURL_VERBOSE_MODE.as_str(),
        aliases![Odbc; "CURLVerboseMode"],
        "Legacy curl verbose-mode switch. The driver accepts the key and does not apply it",
        Deprecation::Ignored {
            guidance: "Set LogLevel=DEBUG in sf.odbc.ini for verbose request logging.",
        },
    ),
    odbc_deprecated(
        param_names::ENABLE_PID_LOG_FILE_NAMES.as_str(),
        aliases![Odbc; "EnablePidLogFileNames"],
        "Legacy PID log-file-name switch. The driver accepts the key and does not apply it",
        Deprecation::Ignored {
            guidance: "Set LogFile in sf.odbc.ini to name the driver log file.",
        },
    ),
    odbc_deprecated(
        param_names::CLIENT_CONFIG_FILE.as_str(),
        aliases![],
        "Legacy sf_client_config.json path. The driver accepts the key and does not apply it",
        Deprecation::Ignored {
            guidance: "Configure driver logging in sf.odbc.ini; sf_client_config.json is not read.",
        },
    ),
    odbc_deprecated(
        param_names::DEFAULT_VARCHAR_SIZE.as_str(),
        aliases![],
        "Legacy DEFAULT_VARCHAR_SIZE. The driver accepts the key and does not apply it",
        Deprecation::Ignored {
            guidance: "VARCHAR column sizes come from result-set metadata; this connection-string key is not applied.",
        },
    ),
    odbc_deprecated(
        param_names::DEFAULT_BINARY_SIZE.as_str(),
        aliases![],
        "Legacy DEFAULT_BINARY_SIZE. The driver accepts the key and does not apply it",
        Deprecation::Ignored {
            guidance: "BINARY column sizes come from result-set metadata; this connection-string key is not applied.",
        },
    ),
];

const fn ignored_odbc_param(canonical: &'static str) -> ParamDef {
    ParamDef::builder()
        .canonical_name(canonical)
        .value_type(ValueType::String)
        .sensitive(false)
        .auth(false)
        .description("ODBC keyword the driver accepts and does not apply.")
        .scopes(&[ParamScope::Connection])
        .used_at_connect(false)
        .mutable_after_connect(false)
        .ignored(true)
        .visible_to(visible_to!(Odbc))
        .build()
}

const fn odbc_deprecated(
    canonical: &'static str,
    aliases: &'static [Alias],
    description: &'static str,
    deprecated: Deprecation,
) -> ParamDef {
    ParamDef::builder()
        .canonical_name(canonical)
        .aliases(aliases)
        .value_type(ValueType::String)
        .sensitive(false)
        .auth(false)
        .description(description)
        .deprecated(deprecated)
        .scopes(&[ParamScope::Connection])
        .used_at_connect(false)
        .mutable_after_connect(false)
        .ignored(true)
        .visible_to(visible_to!(Odbc))
        .build()
}

/// The registry singleton. Built once at startup, immutable thereafter.
pub struct ParamRegistry {
    params: &'static [ParamDef],
    /// Case-insensitive map: lowercased [`VisibleTo::All`] canonical + global
    /// alias → index into `params`.
    alias_index: HashMap<String, usize>,
    /// Case-insensitive map: (wrapper, lowercased scoped alias or restricted
    /// canonical) → index into `params`.
    wrapper_alias_index: HashMap<(Wrapper, String), usize>,
    /// Lowercased canonical names of every registered parameter, including
    /// wrapper-restricted ones. Backs [`Self::is_known`].
    canonicals: HashSet<String>,
}

impl ParamRegistry {
    fn new(params: &'static [ParamDef]) -> Self {
        let mut alias_index = HashMap::new();
        let mut wrapper_alias_index = HashMap::new();
        let mut canonicals = HashSet::new();
        for (i, param) in params.iter().enumerate() {
            let canonical = param.canonical_name.to_ascii_lowercase();
            canonicals.insert(canonical.clone());
            match param.visible_to {
                VisibleTo::All => {
                    alias_index.insert(canonical, i);
                }
                VisibleTo::Only(wrappers) => {
                    for wrapper in wrappers {
                        wrapper_alias_index.insert((*wrapper, canonical.clone()), i);
                    }
                }
            }
            for alias in param.aliases {
                let key = alias.name.to_ascii_lowercase();
                match alias.wrapper {
                    None => {
                        alias_index.insert(key, i);
                    }
                    Some(wrapper) => {
                        wrapper_alias_index.insert((wrapper, key), i);
                    }
                }
            }
        }
        Self {
            params,
            alias_index,
            wrapper_alias_index,
            canonicals,
        }
    }

    /// Resolve a globally visible canonical name or global alias to its
    /// `ParamDef`.
    ///
    /// Accepts any type that can be viewed as a string — `ParamKey`, `&str`,
    /// or `String` — so callers with a typed key can pass it directly without
    /// calling `.as_str()`.  Lookup is case-insensitive. Scoped aliases and
    /// [`VisibleTo::Only`] canonicals are not visible here; use
    /// [`Self::resolve_for`].
    pub fn resolve(&self, key: impl AsRef<str>) -> Option<&ParamDef> {
        self.alias_index
            .get(&key.as_ref().to_ascii_lowercase())
            .map(|&i| &self.params[i])
    }

    /// Wrapper-scoped alias wins; else fall back to global [`Self::resolve`].
    /// Case-insensitive.
    pub fn resolve_for(&self, wrapper: Wrapper, key: impl AsRef<str>) -> Option<&ParamDef> {
        let k = key.as_ref().to_ascii_lowercase();
        if let Some(&i) = self.wrapper_alias_index.get(&(wrapper, k)) {
            return Some(&self.params[i]);
        }
        self.resolve(key)
    }

    /// Canonical name for a key under wrapper-scoped resolution.
    pub fn canonical_name_for(
        &self,
        wrapper: Wrapper,
        key: impl AsRef<str>,
    ) -> Option<&'static str> {
        self.resolve_for(wrapper, key).map(|d| d.canonical_name)
    }

    /// Canonical name for a globally-resolvable key (no wrapper context).
    pub fn canonical_name(&self, key: impl AsRef<str>) -> Option<&'static str> {
        self.resolve(key).map(|d| d.canonical_name)
    }

    /// Whether the parameter resolved by a global lookup is marked sensitive.
    pub fn is_sensitive(&self, key: impl AsRef<str>) -> bool {
        self.resolve(key).is_some_and(|d| d.sensitive)
    }

    /// Whether the parameter is marked sensitive under wrapper-scoped
    /// resolution. Wrappers redacting raw wire spellings (ODBC `PWD`,
    /// `PRIV_KEY_FILE_PWD`) need this: those aliases are scoped and so are
    /// invisible to the global [`Self::is_sensitive`].
    pub fn is_sensitive_for(&self, wrapper: Wrapper, key: impl AsRef<str>) -> bool {
        self.resolve_for(wrapper, key).is_some_and(|d| d.sensitive)
    }

    /// Whether the parameter resolved under wrapper-scoped resolution
    /// participates in authentication (see [`ParamDef::auth`]). Wrappers use
    /// this to classify auth-time failures — e.g. the ODBC SQLSTATE mapper
    /// treats an invalid/missing value for an auth parameter as `28000`. The
    /// `key` may be a canonical name or a wrapper alias (both the raw wire
    /// spelling and the canonical name core echoes back resolve). Unknown keys
    /// are not auth (`false`). Case-insensitive.
    pub fn is_auth_for(&self, wrapper: Wrapper, key: impl AsRef<str>) -> bool {
        self.resolve_for(wrapper, key).is_some_and(|d| d.auth)
    }

    /// Return all registered parameter definitions.
    pub fn all_params(&self) -> &[ParamDef] {
        self.params
    }

    /// Whether `key` is a registered canonical name (case-insensitive).
    ///
    /// Wrapper-restricted canonicals are known: core validation sees them
    /// after a wrapper has already forwarded the canonical key.
    /// Wrapper-scoped wire spellings (`SERVER`) are not known without
    /// wrapper context.
    pub fn is_known(&self, key: &str) -> bool {
        self.canonicals.contains(&key.to_ascii_lowercase())
    }
}

static REGISTRY: LazyLock<ParamRegistry> = LazyLock::new(|| ParamRegistry::new(PARAM_DEFS));

/// Global registry accessor.
pub fn registry() -> &'static ParamRegistry {
    &REGISTRY
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn canonical_names_resolve_for_every_wrapper() {
        // SCREAMING_SNAKE spellings that match a VisibleTo::All canonical name
        // case-insensitively must resolve for every wrapper (and globally),
        // regardless of alias scoping — they are not aliases at all.
        let r = registry();
        let canonical_cases: &[(&str, &str)] = &[
            ("HOST", "host"),
            ("PORT", "port"),
            ("PROTOCOL", "protocol"),
            ("ACCOUNT", "account"),
            ("DATABASE", "database"),
            ("SCHEMA", "schema"),
            ("WAREHOUSE", "warehouse"),
            ("ROLE", "role"),
            ("AUTHENTICATOR", "authenticator"),
            ("TOKEN", "token"),
            ("TOKEN_FILE_PATH", "token_file_path"),
            ("PASSCODE", "passcode"),
            ("OAUTH_CLIENT_ID", "oauth_client_id"),
            ("OAUTH_CLIENT_SECRET", "oauth_client_secret"),
            ("OAUTH_AUTHORIZATION_URL", "oauth_authorization_url"),
            ("OAUTH_TOKEN_REQUEST_URL", "oauth_token_request_url"),
            ("OAUTH_REDIRECT_URI", "oauth_redirect_uri"),
            ("OAUTH_SCOPE", "oauth_scope"),
            (
                "OAUTH_ENABLE_SINGLE_USE_REFRESH_TOKENS",
                "oauth_enable_single_use_refresh_tokens",
            ),
            ("TLS_SKIP_VERIFY", "tls_skip_verify"),
            ("PROXY_HOST", "proxy_host"),
            ("PROXY_PORT", "proxy_port"),
            ("PROXY_SCHEME", "proxy_scheme"),
            ("PROXY_USER", "proxy_user"),
            ("PROXY_PASSWORD", "proxy_password"),
            ("NO_PROXY", "no_proxy"),
            ("PROXY", "proxy"),
            ("USE_PROXY_ENV", "use_proxy_env"),
            ("ALLOW_EMPTY_PROXY", "allow_empty_proxy"),
            ("WORKLOAD_IDENTITY_PROVIDER", "workload_identity_provider"),
            (
                "WORKLOAD_IDENTITY_ENTRA_RESOURCE",
                "workload_identity_entra_resource",
            ),
            (
                "WORKLOAD_IDENTITY_IMPERSONATION_PATH",
                "workload_identity_impersonation_path",
            ),
            (
                "WORKLOAD_IDENTITY_AWS_USE_OUTBOUND_TOKEN",
                "workload_identity_aws_use_outbound_token",
            ),
        ];
        for (name, expected_canonical) in canonical_cases {
            let def = r
                .resolve(expected_canonical)
                .unwrap_or_else(|| panic!("{expected_canonical:?} should resolve globally"));
            assert_eq!(
                def.visible_to,
                VisibleTo::All,
                "{expected_canonical:?} is listed as resolving for every wrapper"
            );
            for wrapper in [Wrapper::Odbc, Wrapper::Jdbc, Wrapper::Python] {
                let def = r
                    .resolve_for(wrapper, name)
                    .unwrap_or_else(|| panic!("{name:?} should resolve for {wrapper:?}"));
                assert_eq!(def.canonical_name, *expected_canonical);
            }
            assert!(
                r.resolve(name).is_some(),
                "canonical spelling {name:?} must also resolve globally"
            );
        }
    }

    #[test]
    fn is_sensitive_for_sees_wrapper_scoped_secret_aliases() {
        let r = registry();
        for scoped_secret in [
            "PWD",
            "PRIV_KEY_FILE_PWD",
            "PRIV_KEY_PWD",
            "PRIV_KEY_BASE64",
        ] {
            assert!(
                r.is_sensitive_for(Wrapper::Odbc, scoped_secret),
                "{scoped_secret:?} must be sensitive under the ODBC flavor"
            );
            assert!(
                !r.is_sensitive(scoped_secret),
                "{scoped_secret:?} is an Odbc-scoped alias and must not resolve globally"
            );
        }
        for not_secret in ["SERVER", "user", "totally_unknown_key"] {
            assert!(
                !r.is_sensitive_for(Wrapper::Odbc, not_secret),
                "{not_secret:?} must not be classified sensitive"
            );
        }
    }

    #[test]
    fn scoped_aliases_resolve_only_for_owning_wrappers() {
        use Wrapper::{DotNet, Jdbc, NodeJs, Odbc, Python};
        // Each wire alias resolves for exactly the wrappers whose old driver
        // accepted that spelling — UD is no more lenient than the driver it
        // replaces:
        //   * legacy ODBC DSN keys (`Snowflake.h`) -> Odbc, plus UD-ODBC's own
        //     `CRL_MODE`/`CRL_ENABLED`.
        //   * JDBC `SFSessionProperty` keys -> Jdbc, both camelCase and the
        //     lowercase-underscore key-pair properties.
        //   * legacy snowflake-connector-python kwargs (`DEFAULT_CONFIGURATION`,
        //     which are also the TOML-profile spellings `config_manager` resolves
        //     under the Python flavor) -> Python.
        let r = registry();
        let scoped_cases: &[(&str, &str, &[Wrapper])] = &[
            // Legacy ODBC DSN keys. None of these are Python kwargs or JDBC
            // properties, so no other wrapper may resolve them.
            ("SERVER", "host", &[Odbc]),
            ("UID", "user", &[Odbc]),
            ("PWD", "password", &[Odbc]),
            ("PROXYWITHENV", "use_proxy_env", &[Odbc]),
            ("ALLOWEMPTYPROXY", "allow_empty_proxy", &[Odbc]),
            ("USECURRENTCATALOG", "use_current_catalog", &[Odbc]),
            ("PRIV_KEY_FILE", "private_key_file", &[Odbc]),
            ("PRIV_KEY_BASE64", "private_key", &[Odbc]),
            ("PRIV_KEY_FILE_PWD", "private_key_password", &[Odbc]),
            ("PRIV_KEY_PWD", "private_key_password", &[Odbc]),
            // JDBC key-pair properties (`SFSessionProperty`); legacy ODBC used
            // the `PRIV_KEY_*` spellings above, never these.
            ("PRIVATE_KEY_BASE64", "private_key", &[Jdbc]),
            ("PRIVATE_KEY_PWD", "private_key_password", &[Jdbc]),
            // JDBC property that is also a legacy Python kwarg (the latter
            // required by the TOML loader).
            (
                "PRIVATE_KEY_FILE_PWD",
                "private_key_password",
                &[Jdbc, Python],
            ),
            // Legacy Python kwarg spelling; the canonical camelCase name does
            // not match it case-insensitively, so the TOML loader needs it.
            ("PASSCODE_IN_PASSWORD", "passcodeInPassword", &[Python]),
            // Legacy ODBC `SecondaryRoles` DSN attribute: the parser uppercases
            // the separator-less key to `SECONDARYROLES`.
            ("SECONDARYROLES", "secondary_roles", &[Odbc]),
            // UD-ODBC's own CRL DSN keys (legacy spelled the family `CRL_CHECK`).
            ("CRL_MODE", "crl_check_mode", &[Odbc]),
            ("CRL_ENABLED", "crl_check_mode", &[Odbc]),
            // 3.x ODBC PUT/GET DSN keys (`Snowflake.h`
            // `SF_CON_PUT_MAXRETRIES` / `SF_CON_GET_MAXRETRIES`). Both map
            // onto the shared `put_get_max_attempts` setting.
            ("PUT_MAXRETRIES", "put_get_max_attempts", &[Odbc]),
            ("GET_MAXRETRIES", "put_get_max_attempts", &[Odbc]),
            ("PUT_COMPRESSLV", "put_compress_level", &[Odbc]),
            ("PUT_TEMPDIR", "put_tempdir", &[Odbc]),
            ("MaxHttpRetries", "retry_max_attempts", &[Odbc]),
            // JDBC-only camelCase properties.
            ("oauthClientId", "oauth_client_id", &[Jdbc]),
            ("oauthClientSecret", "oauth_client_secret", &[Jdbc]),
            ("oauthAuthorizationUrl", "oauth_authorization_url", &[Jdbc]),
            ("oauthTokenRequestUrl", "oauth_token_request_url", &[Jdbc]),
            ("oauthRedirectUri", "oauth_redirect_uri", &[Jdbc]),
            ("oauthScope", "oauth_scope", &[Jdbc]),
            (
                "oauthEnableSingleUseRefreshTokens",
                "oauth_enable_single_use_refresh_tokens",
                &[Jdbc],
            ),
            (
                "clientStoreTemporaryCredential",
                "client_store_temporary_credential",
                &[Jdbc],
            ),
            (
                "allowUnderscoresInHost",
                "preserve_underscores_in_hostname",
                &[Jdbc],
            ),
            ("enablePutGet", "enable_put_get", &[Jdbc]),
            // Python-only legacy kwarg.
            (
                "ENABLE_STAGE_S3_PRIVATELINK_FOR_US_EAST_1",
                "use_s3_regional_url",
                &[Python],
            ),
            // Legacy snowflake-connector-nodejs option spelling. Legacy .NET and
            // JDBC use `token_file_path`, a canonical case variant.
            ("tokenFilePath", "token_file_path", &[NodeJs]),
        ];
        for (alias, expected_canonical, owners) in scoped_cases {
            for wrapper in [Odbc, Jdbc, Python, NodeJs, DotNet] {
                let resolved = r.resolve_for(wrapper, alias).map(|d| d.canonical_name);
                if owners.contains(&wrapper) {
                    assert_eq!(
                        resolved,
                        Some(*expected_canonical),
                        "alias {alias:?} should resolve to {expected_canonical:?} for {wrapper:?}"
                    );
                } else {
                    assert_eq!(
                        resolved, None,
                        "alias {alias:?} must NOT resolve for {wrapper:?} (owned by {owners:?})"
                    );
                }
            }
            // Scoped aliases are never globally resolvable.
            assert!(
                r.resolve(alias).is_none(),
                "scoped alias {alias:?} must not resolve without wrapper context"
            );
        }
    }

    #[test]
    fn spellings_no_old_driver_accepted_resolve_for_no_wrapper() {
        // UD must be no more lenient than the driver it replaces. These
        // convenience spellings were accepted while aliases were global; none of
        // them appears in snowflake-odbc's `Snowflake.h`, JDBC's
        // `SFSessionProperty`, or the Python connector's `DEFAULT_CONFIGURATION`,
        // so no wrapper may resolve them. The canonical name (right column) is
        // what each driver actually accepted, and still resolves everywhere.
        let r = registry();
        let removed: &[(&str, &str)] = &[
            // Legacy ODBC's key is `NO_PROXY`; legacy Python's kwarg is `no_proxy`.
            ("NOPROXY", "no_proxy"),
            // The legacy Python kwarg is the full `oauth_token_request_url`.
            ("OAUTH_TOKEN_URL", "oauth_token_request_url"),
            // No legacy driver had TLS-verification or root-store DSN keys.
            ("TLS_VERIFY_HOSTNAME", "verify_hostname"),
            ("TLS_VERIFY_CERTIFICATES", "verify_certificates"),
            ("TLS_CUSTOM_ROOT_STORE_PATH", "custom_root_store_path"),
        ];
        for (spelling, canonical) in removed {
            for wrapper in [Wrapper::Odbc, Wrapper::Jdbc, Wrapper::Python] {
                assert!(
                    r.resolve_for(wrapper, spelling).is_none(),
                    "{spelling:?} was accepted by no old driver and must not resolve for {wrapper:?}"
                );
            }
            assert_eq!(
                r.resolve(canonical).map(|d| d.canonical_name),
                Some(*canonical),
                "canonical {canonical:?} must still resolve"
            );
        }
    }

    #[test]
    fn wire_aliases_are_not_globally_resolvable() {
        // The whole point of the per-wrapper migration: DSN/wire spellings are
        // scoped, so the wrapper-agnostic `resolve` (used by TOML profile
        // loading with an explicit flavor, and by `is_known`) no longer remaps
        // them. Canonical names still resolve globally, case-insensitively.
        let r = registry();
        for scoped_only in ["SERVER", "UID", "PWD", "PRIV_KEY_FILE", "oauthClientId"] {
            assert!(
                r.resolve(scoped_only).is_none(),
                "{scoped_only:?} must not resolve without wrapper context after scoping"
            );
        }
        for canonical in ["host", "user", "password", "private_key_file"] {
            assert_eq!(
                r.resolve(canonical).map(|d| d.canonical_name),
                Some(canonical),
                "canonical {canonical:?} must still resolve globally"
            );
        }
    }

    #[test]
    fn every_alias_is_wrapper_scoped() {
        // Structural invariant, not a spelling list: an alias exists only where
        // some old driver accepted the spelling, so every alias must name the
        // wrapper it came from. Asserting it over `PARAM_DEFS` — rather than
        // over an enumerated list of known spellings like the tests above — is
        // what makes a newly added global alias fail here instead of drifting
        // in unnoticed.
        //
        // A genuinely wrapper-agnostic spelling would need a deliberate
        // exception here plus a comment naming every driver that accepts it.
        // A spelling that merely matches a canonical name case-insensitively
        // (legacy ODBC's `NO_PROXY` for `no_proxy`, `TOKEN_FILE_PATH` for
        // `token_file_path`, `PASSCODEINPASSWORD` for `passcodeInPassword`) is
        // not an alias at all and must not be added as one — see
        // `canonical_names_resolve_for_every_wrapper`.
        for param in registry().all_params() {
            for alias in param.aliases {
                assert!(
                    alias.wrapper.is_some(),
                    "alias {:?} of {:?} is global; scope it to the wrapper(s) whose old \
                     driver accepted that spelling, e.g. `aliases![Jdbc; {:?}]`",
                    alias.name,
                    param.canonical_name,
                    alias.name
                );
            }
        }
    }

    #[test]
    fn omitted_fields_take_struct_defaults() {
        const DEF: ParamDef = ParamDef::builder()
            .canonical_name("test_only")
            .value_type(ValueType::String)
            .sensitive(false)
            .auth(false)
            .description("")
            .scopes(&[ParamScope::Connection])
            .used_at_connect(false)
            .mutable_after_connect(false)
            .build();
        assert!(DEF.aliases.is_empty());
        assert_eq!(DEF.additional_value_type, None);
        assert_eq!(DEF.required, Required::Never);
        assert_eq!(DEF.default, None);
        const {
            assert!(!DEF.sensitive);
            assert!(!DEF.auth);
        }
        assert_eq!(DEF.deprecated, None);
        assert_eq!(DEF.visible_to, VisibleTo::All);
        const {
            assert!(!DEF.ignored);
        }
    }

    #[test]
    fn ignored_odbc_params_resolve_only_for_odbc_and_are_unused() {
        let r = registry();
        for (key, canonical) in [
            ("DSN", "dsn"),
            ("DRIVER", "driver"),
            ("FILEDSN", "filedsn"),
            ("SAVEFILE", "savefile"),
            ("DESCRIPTION", "description"),
            ("LOCALE", "locale"),
            ("SETUP", "setup"),
            ("DriverODBCVer", "driverodbcver"),
            ("APILevel", "apilevel"),
            ("SQLLevel", "sqllevel"),
            ("ConnectFunctions", "connectfunctions"),
            ("TRACING", "tracing"),
        ] {
            let def = r
                .resolve_for(Wrapper::Odbc, key)
                .unwrap_or_else(|| panic!("{key} should resolve for ODBC"));
            assert_eq!(def.canonical_name, canonical);
            assert!(def.ignored, "{canonical} should be ignored");
            assert!(!def.used_at_connect, "{canonical} is unused at connect");
            assert!(
                r.resolve(key).is_none(),
                "{key} must not resolve without wrapper context"
            );
            for wrapper in [
                Wrapper::Jdbc,
                Wrapper::Python,
                Wrapper::NodeJs,
                Wrapper::DotNet,
            ] {
                assert!(
                    r.resolve_for(wrapper, key).is_none(),
                    "{key} must not resolve for {wrapper:?}"
                );
            }
        }
    }

    #[test]
    fn odbc_deprecated_logging_dsn_keys_resolve_only_for_odbc() {
        let r = registry();
        let cases: &[(&str, &str)] = &[
            ("LogLevel", "log_level"),
            ("LogPath", "log_path"),
            ("LogFileSize", "log_file_size"),
            ("LogFileCount", "log_file_count"),
            ("CURLVerboseMode", "curl_verbose_mode"),
            ("EnablePidLogFileNames", "enable_pid_log_file_names"),
            ("CLIENT_CONFIG_FILE", "client_config_file"),
        ];
        for (key, canonical) in cases {
            let def = r
                .resolve_for(Wrapper::Odbc, key)
                .unwrap_or_else(|| panic!("{key:?} should resolve for Odbc"));
            assert_eq!(def.canonical_name, *canonical);
            let Some(Deprecation::Ignored { guidance }) = def.deprecated else {
                panic!("{canonical} should be deprecated with guidance");
            };
            assert!(
                guidance.contains("sf.odbc.ini"),
                "{canonical} guidance should point at sf.odbc.ini, got {guidance:?}"
            );
            assert!(def.ignored, "{canonical} should be ignored");
            assert_eq!(def.visible_to, visible_to!(Odbc));
            assert!(r.resolve(canonical).is_none());
            for wrapper in [
                Wrapper::Jdbc,
                Wrapper::Python,
                Wrapper::NodeJs,
                Wrapper::DotNet,
            ] {
                assert!(
                    r.resolve_for(wrapper, key).is_none(),
                    "{key} must not resolve for {wrapper:?}"
                );
            }
        }
    }

    #[test]
    fn odbc_deprecated_default_size_keys_resolve_only_for_odbc() {
        let r = registry();
        let cases: &[(&str, &str, &str)] = &[
            (
                "DEFAULT_VARCHAR_SIZE",
                "default_varchar_size",
                "VARCHAR column sizes come from result-set metadata",
            ),
            (
                "DEFAULT_BINARY_SIZE",
                "default_binary_size",
                "BINARY column sizes come from result-set metadata",
            ),
        ];
        for (key, canonical, guidance_prefix) in cases {
            let def = r
                .resolve_for(Wrapper::Odbc, key)
                .unwrap_or_else(|| panic!("{key:?} should resolve for Odbc"));
            assert_eq!(def.canonical_name, *canonical);
            let Some(Deprecation::Ignored { guidance }) = def.deprecated else {
                panic!("{canonical} should be deprecated with guidance");
            };
            assert!(
                guidance.contains(guidance_prefix),
                "{canonical} guidance should name the metadata source, got {guidance:?}"
            );
            assert!(def.ignored, "{canonical} should be ignored");
            assert_eq!(def.visible_to, visible_to!(Odbc));
            assert!(r.resolve(canonical).is_none());
            for wrapper in [
                Wrapper::Jdbc,
                Wrapper::Python,
                Wrapper::NodeJs,
                Wrapper::DotNet,
            ] {
                assert!(
                    r.resolve_for(wrapper, key).is_none(),
                    "{key} must not resolve for {wrapper:?}"
                );
            }
        }
    }

    #[test]
    #[should_panic(expected = "ParamDef::builder() requires canonical_name")]
    fn builder_panics_when_required_setters_are_omitted() {
        let _ = ParamDef::builder().build();
    }

    #[test]
    fn credential_like_params_are_sensitive() {
        fn holds_secret(name: &str) -> bool {
            name == "private_key"
                || name == "token"
                || name == "session_token"
                || name == "master_token"
                || name == "passcode"
                || name == "proxy"
                || name == "password"
                || name.ends_with("_password")
                || name.ends_with("_secret")
        }
        for param in registry().all_params() {
            assert_eq!(
                param.sensitive,
                holds_secret(param.canonical_name),
                "{:?} sensitive={} but holds_secret={}",
                param.canonical_name,
                param.sensitive,
                holds_secret(param.canonical_name)
            );
        }
    }

    #[test]
    fn resolve_canonical_names() {
        let r = registry();
        for param in r.all_params() {
            if param.visible_to != VisibleTo::All {
                continue;
            }
            assert!(
                r.resolve(param.canonical_name).is_some(),
                "canonical name {:?} should resolve",
                param.canonical_name
            );
        }
    }

    #[test]
    fn restricted_canonicals_resolve_only_for_listed_wrappers() {
        let r = registry();
        let restricted: Vec<_> = r
            .all_params()
            .iter()
            .filter(|p| p.visible_to != VisibleTo::All)
            .collect();
        assert!(
            !restricted.is_empty(),
            "expected at least one VisibleTo::Only parameter"
        );

        let wrappers = [
            Wrapper::Odbc,
            Wrapper::Jdbc,
            Wrapper::Python,
            Wrapper::NodeJs,
            Wrapper::DotNet,
        ];
        for param in restricted {
            assert!(
                r.resolve(param.canonical_name).is_none(),
                "restricted canonical {:?} must not resolve globally",
                param.canonical_name
            );
            assert!(
                r.is_known(param.canonical_name),
                "restricted canonical {:?} must still be known",
                param.canonical_name
            );
            for wrapper in wrappers {
                let resolved = r
                    .resolve_for(wrapper, param.canonical_name)
                    .map(|d| d.canonical_name);
                if param.is_visible_to(wrapper) {
                    assert_eq!(
                        resolved,
                        Some(param.canonical_name),
                        "{:?} should resolve for {wrapper:?}",
                        param.canonical_name
                    );
                } else {
                    assert_eq!(
                        resolved, None,
                        "{:?} must not resolve for {wrapper:?}",
                        param.canonical_name
                    );
                }
            }
        }
    }

    #[test]
    fn aliases_are_subset_of_visible_to() {
        for param in registry().all_params() {
            match param.visible_to {
                VisibleTo::Only(wrappers) => {
                    assert!(
                        !wrappers.is_empty(),
                        "parameter {:?} has VisibleTo::Only(&[])",
                        param.canonical_name
                    );
                }
                VisibleTo::All => {}
            }
            for alias in param.aliases {
                let Some(wrapper) = alias.wrapper else {
                    continue;
                };
                assert!(
                    param.is_visible_to(wrapper),
                    "alias {:?} of {:?} is scoped to {wrapper:?} but the \
                     parameter is not visible to that wrapper",
                    alias.name,
                    param.canonical_name
                );
            }
        }
    }

    #[test]
    fn disable_parallel_user_prompt_registered_with_correct_defaults() {
        let r = registry();

        let def = r
            .resolve("DISABLE_PARALLEL_USER_PROMPT")
            .expect("DISABLE_PARALLEL_USER_PROMPT alias should resolve");
        assert_eq!(def.canonical_name, "disable_parallel_user_prompt");
        assert_eq!(def.value_type, ValueType::Bool);
        // Default is true: locking is ON by default.
        let default_val = def.default.expect("param must have a default");
        assert_eq!(default_val, DefaultValue::Bool(true));
        assert!(def.used_at_connect);
        assert!(!def.mutable_after_connect);
        assert!(!def.sensitive);

        // Canonical name also resolves.
        assert!(r.resolve("disable_parallel_user_prompt").is_some());
    }

    #[test]
    fn client_store_temporary_credential_defaults_true() {
        let def = registry()
            .resolve("client_store_temporary_credential")
            .expect("client_store_temporary_credential should resolve");
        assert_eq!(def.default, Some(DefaultValue::Bool(true)));
    }

    #[test]
    fn serialize_session_operations_has_no_registry_default() {
        let r = registry();
        let def = r
            .resolve("serialize_session_operations")
            .expect("serialize_session_operations should resolve");
        assert_eq!(def.value_type, ValueType::Bool);
        assert!(def.default.is_none());
        assert_eq!(def.scopes, &[ParamScope::Connection]);
        assert!(!def.used_at_connect);
        assert!(!def.mutable_after_connect);
        assert!(r.resolve("SERIALIZE_SESSION_OPERATIONS").is_some());
    }

    #[test]
    fn client_session_keep_alive_params_registered() {
        let r = registry();
        let keep_alive = r
            .resolve("CLIENT_SESSION_KEEP_ALIVE")
            .expect("CLIENT_SESSION_KEEP_ALIVE should resolve");
        assert_eq!(keep_alive.value_type, ValueType::Bool);
        assert_eq!(keep_alive.scopes, &[ParamScope::Session]);
        assert!(keep_alive.used_at_connect);

        let freq = r
            .resolve("CLIENT_SESSION_KEEP_ALIVE_HEARTBEAT_FREQUENCY")
            .expect("heartbeat frequency param should resolve");
        assert_eq!(freq.value_type, ValueType::Int);
        assert_eq!(freq.scopes, &[ParamScope::Session]);
        assert!(freq.used_at_connect);
    }

    #[test]
    fn retry_backoff_params_have_correct_metadata() {
        let r = registry();

        for (key, value_type) in [
            ("retry_backoff_base_ms", ValueType::Int),
            ("retry_backoff_cap_ms", ValueType::Int),
            ("retry_backoff_factor", ValueType::Double),
            ("retry_backoff_jitter", ValueType::String),
        ] {
            let d = r
                .resolve(key)
                .unwrap_or_else(|| panic!("expected registry entry for {key}"));
            assert_eq!(d.canonical_name, key);
            assert_eq!(d.value_type, value_type, "key {key}");
            assert_eq!(d.scopes, &[ParamScope::Connection], "key {key}");
            // Client-only knobs: not sent at login, immutable after connect.
            assert!(!d.used_at_connect, "key {key}");
            assert!(!d.mutable_after_connect, "key {key}");
            assert!(!d.sensitive, "key {key}");
            assert!(d.default.is_some(), "key {key} must have a static default");
        }
    }

    #[test]
    fn unknown_key_returns_none() {
        let r = registry();
        assert!(r.resolve("nonexistent_param").is_none());
        assert!(r.resolve("").is_none());
        assert!(r.resolve("FOOBAR").is_none());
        assert!(!r.is_known("nonexistent_param"));
    }

    #[test]
    fn tls_version_params_have_correct_metadata() {
        let r = registry();
        for key in ["min_tls_version", "max_tls_version"] {
            let d = r
                .resolve(key)
                .unwrap_or_else(|| panic!("expected registry entry for {key}"));
            assert_eq!(d.canonical_name, key);
            assert_eq!(d.value_type, ValueType::String);
            assert_eq!(d.scopes, &[ParamScope::Connection]);
            assert!(d.used_at_connect, "key {key} must be used at connect");
            assert!(
                !d.mutable_after_connect,
                "key {key} must be immutable after connect"
            );
            assert!(d.default.is_some(), "key {key} must have a static default");
            assert!(!d.sensitive);
        }
    }

    #[test]
    fn case_insensitive_lookup() {
        let r = registry();
        let variants = ["Host", "HOST", "host", "hOsT"];
        for key in variants {
            let def = r
                .resolve(key)
                .unwrap_or_else(|| panic!("{key:?} should resolve"));
            assert_eq!(def.canonical_name, "host");
        }
    }

    #[test]
    fn canonical_names_are_unique() {
        let r = registry();
        let mut seen = std::collections::HashSet::new();
        for param in r.all_params() {
            assert!(
                seen.insert(param.canonical_name),
                "duplicate canonical name: {:?}",
                param.canonical_name
            );
        }
    }

    #[test]
    fn every_param_has_at_least_one_scope() {
        // A scopeless param is an illegal state: the scope-membership helpers
        // (`is_statement_only` / `is_session_scoped`) would misclassify it.
        for param in registry().all_params() {
            assert!(
                !param.scopes.is_empty(),
                "parameter {:?} has no scopes",
                param.canonical_name
            );
        }
    }

    #[test]
    fn no_wrapper_visible_alias_shadows_another_canonical_name() {
        // Every spelling a wrapper can see must mean that wrapper's own canonical
        // name, with one deliberate exception per entry below: legacy ODBC's
        // `LOGIN_TIMEOUT` means `authentication_timeout`, shadowing the canonical
        // `login_timeout` param. Per-wrapper divergence like that is the point of
        // `configuration_flavor` (see `resolve_for_prefers_wrapper_scoped_alias`),
        // so it is enumerated rather than tolerated wholesale.
        const DELIBERATE_SHADOWS: &[(Wrapper, &str)] = &[(Wrapper::Odbc, "LOGIN_TIMEOUT")];

        let r = registry();
        // Lowercased canonical -> canonical, so camelCase canonicals
        // (`passcodeInPassword`) compare case-insensitively like the resolver does.
        let canonical_by_lower: HashMap<String, &'static str> = r
            .all_params()
            .iter()
            .map(|p| (p.canonical_name.to_ascii_lowercase(), p.canonical_name))
            .collect();

        for wrapper in [
            Wrapper::Odbc,
            Wrapper::Jdbc,
            Wrapper::Python,
            Wrapper::NodeJs,
            Wrapper::DotNet,
        ] {
            for param in r.all_params() {
                for alias in param.alias_names_for(wrapper) {
                    let lower = alias.to_ascii_lowercase();
                    let Some(shadowed) = canonical_by_lower.get(lower.as_str()) else {
                        continue; // not a canonical spelling at all
                    };
                    if *shadowed == param.canonical_name {
                        continue; // merely a case variant of this param's own name
                    }
                    assert!(
                        DELIBERATE_SHADOWS.contains(&(wrapper, alias)),
                        "alias {alias:?} of {:?} shadows the canonical name {shadowed:?} for \
                         {wrapper:?}; add it to DELIBERATE_SHADOWS only if that remap is intended",
                        param.canonical_name
                    );
                }
            }
        }
    }

    #[test]
    fn is_known_works() {
        let r = registry();
        // `is_known` is any registered canonical, including wrapper-restricted
        // ones. Canonical names match case-insensitively; wrapper-scoped wire
        // spellings (e.g. `SERVER`) are not known without wrapper context.
        assert!(r.is_known("account"));
        assert!(r.is_known("ACCOUNT"));
        assert!(r.is_known("host"));
        assert!(r.is_known("HOST"));
        assert!(r.is_known("enable_put_get"));
        assert!(r.is_known("ENABLE_PUT_GET"));
        assert!(!r.is_known("SERVER"));
        assert!(!r.is_known("unknown_key"));
        assert!(r.resolve("enable_put_get").is_none());
    }

    #[test]
    fn wif_params_registered() {
        let r = registry();
        for key in [
            "workload_identity_provider",
            "WORKLOAD_IDENTITY_PROVIDER",
            "workload_identity_entra_resource",
            "WORKLOAD_IDENTITY_ENTRA_RESOURCE",
            "workload_identity_impersonation_path",
            "WORKLOAD_IDENTITY_IMPERSONATION_PATH",
            "workload_identity_aws_use_outbound_token",
            "WORKLOAD_IDENTITY_AWS_USE_OUTBOUND_TOKEN",
        ] {
            assert!(
                r.is_known(key),
                "Expected WIF param '{key}' to be registered"
            );
        }
    }

    #[test]
    fn log_max_query_length_has_correct_defaults() {
        let r = registry();
        let def = r
            .resolve("log_max_query_length")
            .expect("log_max_query_length should be registered");
        assert_eq!(def.canonical_name, "log_max_query_length");
        assert_eq!(def.value_type, ValueType::Int);
        assert_eq!(def.scopes, &[ParamScope::Connection]);
        assert!(!def.used_at_connect);
        assert!(!def.mutable_after_connect);
        assert_eq!(def.default.unwrap(), DefaultValue::Int(80));
    }

    #[test]
    fn log_query_text_has_correct_defaults() {
        let r = registry();
        let def = r
            .resolve("log_query_text")
            .expect("log_query_text should be registered");
        assert_eq!(def.canonical_name, "log_query_text");
        assert_eq!(def.value_type, ValueType::Bool);
        assert_eq!(def.additional_value_type, Some(ValueType::String));
        assert_eq!(def.scopes, &[ParamScope::Connection]);
        assert!(!def.used_at_connect);
        assert!(!def.mutable_after_connect);
        assert!(!def.sensitive);
        assert_eq!(def.default.unwrap(), DefaultValue::Bool(false));
    }

    #[test]
    fn log_query_parameters_has_correct_defaults() {
        let r = registry();
        let def = r
            .resolve("log_query_parameters")
            .expect("log_query_parameters should be registered");
        assert_eq!(def.canonical_name, "log_query_parameters");
        assert_eq!(def.value_type, ValueType::Bool);
        assert_eq!(def.additional_value_type, Some(ValueType::String));
        assert_eq!(def.scopes, &[ParamScope::Connection]);
        assert!(!def.used_at_connect);
        assert!(!def.mutable_after_connect);
        assert!(!def.sensitive);
        assert_eq!(def.default.unwrap(), DefaultValue::Bool(false));
    }

    #[test]
    fn log_query_text_resolves_uppercase_alias() {
        let r = registry();
        let def = r
            .resolve("LOG_QUERY_TEXT")
            .expect("LOG_QUERY_TEXT alias should resolve");
        assert_eq!(def.canonical_name, "log_query_text");
    }

    #[test]
    fn log_query_parameters_resolves_uppercase_alias() {
        let r = registry();
        let def = r
            .resolve("LOG_QUERY_PARAMETERS")
            .expect("LOG_QUERY_PARAMETERS alias should resolve");
        assert_eq!(def.canonical_name, "log_query_parameters");
    }

    #[test]
    fn statement_scope_params_are_never_used_at_connect() {
        let r = registry();
        for p in r.all_params() {
            if p.scopes.contains(&ParamScope::Statement) {
                assert!(
                    !p.used_at_connect,
                    "expected used_at_connect == false for {}",
                    p.canonical_name
                );
                assert!(!p.effective_used_at_connect());
            }
        }
    }

    #[test]
    fn session_context_params_are_session_scoped_and_mutable_after_connect() {
        let r = registry();
        for key in ["database", "schema", "warehouse", "role"] {
            let d = r
                .resolve(key)
                .unwrap_or_else(|| panic!("expected registry entry for {key}"));
            assert_eq!(d.scopes, &[ParamScope::Session], "key {key}");
            assert!(d.used_at_connect, "key {key}");
            assert!(d.mutable_after_connect, "key {key}");
        }
    }

    #[test]
    fn secondary_roles_is_connection_scoped_and_immutable_after_connect() {
        let r = registry();
        let d = r
            .resolve("secondary_roles")
            .expect("expected registry entry for secondary_roles");
        assert_eq!(d.scopes, &[ParamScope::Connection]);
        assert!(d.used_at_connect);
        assert!(!d.mutable_after_connect);
        assert_eq!(d.value_type, ValueType::String);
    }

    #[test]
    fn put_compress_level_is_odbc_only_connection_int() {
        let r = registry();
        assert!(r.resolve("put_compress_level").is_none());
        let d = r
            .resolve_for(Wrapper::Odbc, "PUT_COMPRESSLV")
            .expect("PUT_COMPRESSLV should resolve for Odbc");
        assert_eq!(d.canonical_name, "put_compress_level");
        assert_eq!(d.scopes, &[ParamScope::Connection]);
        assert!(!d.used_at_connect);
        assert!(d.mutable_after_connect);
        assert_eq!(d.value_type, ValueType::Int);
        assert!(d.is_visible_to(Wrapper::Odbc));
        assert!(!d.is_visible_to(Wrapper::Python));
        assert!(!d.is_visible_to(Wrapper::Jdbc));
    }

    #[test]
    fn put_tempdir_is_odbc_only_connection_string() {
        let r = registry();
        assert!(r.resolve("put_tempdir").is_none());
        let d = r
            .resolve_for(Wrapper::Odbc, "PUT_TEMPDIR")
            .expect("PUT_TEMPDIR should resolve for Odbc");
        assert_eq!(d.canonical_name, "put_tempdir");
        assert_eq!(d.scopes, &[ParamScope::Connection]);
        assert!(!d.used_at_connect);
        assert!(d.mutable_after_connect);
        assert_eq!(d.value_type, ValueType::String);
        assert!(d.is_visible_to(Wrapper::Odbc));
        assert!(!d.is_visible_to(Wrapper::Python));
        assert!(!d.is_visible_to(Wrapper::Jdbc));
    }

    #[test]
    fn proxy_params_have_correct_metadata() {
        let r = registry();
        for key in [
            "proxy_host",
            "proxy_port",
            "proxy_scheme",
            "proxy_user",
            "proxy_password",
            "no_proxy",
            "proxy",
            "use_proxy_env",
            "allow_empty_proxy",
        ] {
            let d = r
                .resolve(key)
                .unwrap_or_else(|| panic!("expected registry entry for {key}"));
            assert_eq!(d.scopes, &[ParamScope::Connection], "key {key}");
            assert!(d.used_at_connect, "key {key}");
            assert!(!d.mutable_after_connect, "key {key}");
        }
        let port = r.resolve("proxy_port").unwrap();
        assert_eq!(port.value_type, ValueType::Int);
        let pw = r.resolve("proxy_password").unwrap();
        assert!(pw.sensitive, "proxy_password must be marked sensitive");
        let proxy = r.resolve("proxy").unwrap();
        assert!(
            proxy.sensitive,
            "proxy URL must be sensitive (may contain creds)"
        );
        let host = r.resolve("proxy_host").unwrap();
        assert!(!host.sensitive);
        // PROXY must NOT alias proxy_host: their formats differ (URL vs hostname).
        assert_eq!(r.resolve("PROXY").unwrap().canonical_name, "proxy");
    }

    #[test]
    fn resolve_for_prefers_wrapper_scoped_alias() {
        let r = registry();
        let def = r
            .resolve_for(Wrapper::Odbc, "LOGIN_TIMEOUT")
            .expect("ODBC LOGIN_TIMEOUT should resolve");
        assert_eq!(def.canonical_name, "authentication_timeout");
        assert_eq!(
            r.canonical_name_for(Wrapper::Odbc, "LOGIN_TIMEOUT"),
            Some("authentication_timeout")
        );
    }

    #[test]
    fn resolve_for_falls_back_to_canonical_index() {
        let r = registry();
        for wrapper in [Wrapper::Jdbc, Wrapper::Python] {
            let def = r
                .resolve_for(wrapper, "LOGIN_TIMEOUT")
                .unwrap_or_else(|| panic!("{wrapper:?} LOGIN_TIMEOUT should resolve"));
            assert_eq!(
                def.canonical_name, "login_timeout",
                "{wrapper:?} LOGIN_TIMEOUT should map to login_timeout"
            );
        }
        // Global resolve (no wrapper) still gets login_timeout via the canonical
        // name match — `LOGIN_TIMEOUT` is no longer a stored alias.
        assert_eq!(
            r.resolve("LOGIN_TIMEOUT").map(|d| d.canonical_name),
            Some("login_timeout")
        );
        // Canonical names fall back to the global index for every wrapper.
        for wrapper in [Wrapper::Odbc, Wrapper::Jdbc, Wrapper::Python] {
            assert_eq!(
                r.resolve_for(wrapper, "account").map(|d| d.canonical_name),
                Some("account"),
                "{wrapper:?}"
            );
        }
        // `SERVER` is the legacy ODBC DSN spelling and ODBC-only: JDBC carries the
        // host in the URL and the legacy Python connector has no `server` kwarg.
        // The wrapper-agnostic `resolve` never sees it.
        assert_eq!(
            r.resolve_for(Wrapper::Odbc, "SERVER")
                .map(|d| d.canonical_name),
            Some("host")
        );
        for wrapper in [Wrapper::Jdbc, Wrapper::Python] {
            assert!(
                r.resolve_for(wrapper, "SERVER").is_none(),
                "{wrapper:?} must not resolve the ODBC-only SERVER spelling"
            );
        }
        assert!(r.resolve("SERVER").is_none());
    }

    #[test]
    fn wrapper_scoped_alias_names_are_unique_per_wrapper() {
        // Same-name → different-canonical within one registration class is an
        // authoring error. Globals and (wrapper, name) scoped entries live in
        // separate indexes, so one spelling may be both a canonical/global name and
        // a scoped alias for a different param: `LOGIN_TIMEOUT` is the canonical
        // `login_timeout`, and an Odbc-scoped alias of `authentication_timeout`.
        let mut global: HashMap<String, &'static str> = HashMap::new();
        let mut scoped: HashMap<(Wrapper, String), &'static str> = HashMap::new();
        for param in registry().all_params() {
            let canon = param.canonical_name;
            let gkey = canon.to_ascii_lowercase();
            if let Some(prev) = global.insert(gkey, canon) {
                panic!("duplicate canonical registration: {canon:?} and {prev:?}");
            }
            for alias in param.aliases {
                let key = alias.name.to_ascii_lowercase();
                match alias.wrapper {
                    None => {
                        if let Some(prev) = global.insert(key, canon) {
                            assert_eq!(
                                prev, canon,
                                "global alias {:?} maps to both {prev:?} and {canon:?}",
                                alias.name
                            );
                        }
                    }
                    Some(wrapper) => {
                        if let Some(prev) = scoped.insert((wrapper, key), canon) {
                            assert_eq!(
                                prev, canon,
                                "scoped alias {:?} for {wrapper:?} maps to both {prev:?} and {canon:?}",
                                alias.name
                            );
                        }
                    }
                }
            }
        }
    }

    #[test]
    fn is_auth_for_flags_credential_and_oauth_params() {
        let r = registry();
        // Canonical names, ODBC wire spellings, and mixed case all resolve to
        // the same `auth` verdict — core echoes canonical names, wrappers see
        // the SCREAMING_SNAKE DSN aliases.
        for key in [
            "user",
            "USER",
            "UID",
            "password",
            "PWD",
            "authenticator",
            "private_key",
            "PRIV_KEY_BASE64",
            "private_key_file",
            "PRIV_KEY_FILE",
            "private_key_password",
            "PRIV_KEY_FILE_PWD",
            "token",
            "TOKEN",
            // The path is not the credential, but supplying it is how the
            // caller presents one, so a bad path is an auth failure.
            "token_file_path",
            "oauth_client_id",
            "OAUTH_CLIENT_SECRET",
            "oauth_credentials_in_body",
        ] {
            assert!(
                r.is_auth_for(Wrapper::Odbc, key),
                "{key} should be classified as an auth parameter"
            );
        }

        // Every OAuth and WIF parameter is an auth parameter.
        for def in r.all_params() {
            if def.canonical_name.starts_with("oauth_")
                || def.canonical_name.starts_with("workload_identity_")
            {
                assert!(
                    r.is_auth_for(Wrapper::Odbc, def.canonical_name),
                    "{} should be an auth parameter",
                    def.canonical_name
                );
            }
        }

        // Non-credential connection params and unknown keys are not auth.
        for key in [
            "account",
            "host",
            "SERVER",
            "warehouse",
            "crl_check_mode",
            "proxy_password",
            "totally_unknown_key",
        ] {
            assert!(
                !r.is_auth_for(Wrapper::Odbc, key),
                "{key} should not be classified as an auth parameter"
            );
        }
    }
}
