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

use std::collections::HashMap;
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
    pub const PUT_FASTFAIL: ParamKey = ParamKey("put_fastfail");
    pub const GET_FASTFAIL: ParamKey = ParamKey("get_fastfail");
    pub const AUTHENTICATION_TIMEOUT: ParamKey = ParamKey("authentication_timeout");
    pub const OKTA_USERNAME: ParamKey = ParamKey("okta_username");
    pub const DISABLE_SAML_URL_CHECK: ParamKey = ParamKey("disable_saml_url_check");
    pub const DISABLE_PARALLEL_USER_PROMPT: ParamKey = ParamKey("disable_parallel_user_prompt");
    pub const DISABLE_QUERY_CONTEXT_CACHE: ParamKey = ParamKey("disable_query_context_cache");
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
}

/// Default `retry_max_attempts` for general HTTP calls (mirrors the `ParamDef`).
pub const DEFAULT_RETRY_MAX_ATTEMPTS: u32 = 6;

/// Default `put_get_max_attempts` (mirrors the `ParamDef`).
pub const DEFAULT_PUT_GET_MAX_ATTEMPTS: u32 = 6;

/// Default `login_timeout` in seconds (mirrors the `ParamDef`).
pub const DEFAULT_LOGIN_TIMEOUT_SECS: u64 = 120;

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
/// * `aliases![]` — no aliases (the common case; canonical name still resolves
///   case-insensitively).
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

/// Defines a single supported configuration parameter.
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

    /// Human-readable description.
    pub description: &'static str,

    /// If deprecated, the canonical name of the replacement parameter.
    pub deprecated_by: Option<&'static str>,

    /// Which API layer(s) may write this parameter. A parameter may be valid at
    /// more than one level (e.g. `QUERY_TAG` is settable both at the
    /// session/connection level and per-statement).
    pub scopes: &'static [ParamScope],

    /// When true, the resolved connection-seed value participates in login / new session.
    pub used_at_connect: bool,

    /// When false, connection-level setters must reject changes once connected.
    pub mutable_after_connect: bool,
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

static PARAM_DEFS: &[ParamDef] = &[
    // ── Server ──────────────────────────────────────────────────────────
    ParamDef {
        canonical_name: param_names::ACCOUNT.as_str(),
        aliases: aliases![],
        value_type: ValueType::String,
        additional_value_type: None,
        required: Required::Always,
        default: None,
        sensitive: false,
        description: "Snowflake account identifier",
        deprecated_by: None,
        scopes: &[ParamScope::Connection],
        used_at_connect: true,
        mutable_after_connect: false,
    },
    ParamDef {
        canonical_name: param_names::HOST.as_str(),
        // `HOST` is redundant with the canonical name (case-insensitive match).
        // `SERVER` is the ODBC DSN spelling (`Snowflake.h` `SF_HOST_KEY`) and is
        // ODBC-only: the legacy Python connector has no `server` kwarg, and JDBC
        // carries the host in the JDBC URL.
        aliases: aliases![Odbc; "SERVER"],
        value_type: ValueType::String,
        additional_value_type: None,
        required: Required::Never,
        default: None,
        sensitive: false,
        description: "Snowflake server hostname",
        deprecated_by: None,
        scopes: &[ParamScope::Connection],
        used_at_connect: true,
        mutable_after_connect: false,
    },
    ParamDef {
        canonical_name: param_names::PORT.as_str(),
        aliases: aliases![],
        value_type: ValueType::Int,
        additional_value_type: None,
        required: Required::Never,
        default: None,
        sensitive: false,
        description: "Server port number",
        deprecated_by: None,
        scopes: &[ParamScope::Connection],
        used_at_connect: true,
        mutable_after_connect: false,
    },
    ParamDef {
        canonical_name: param_names::PROTOCOL.as_str(),
        aliases: aliases![],
        value_type: ValueType::String,
        additional_value_type: None,
        required: Required::Never,
        default: None,
        sensitive: false,
        description: "Connection protocol (http or https)",
        deprecated_by: None,
        scopes: &[ParamScope::Connection],
        used_at_connect: true,
        mutable_after_connect: false,
    },
    ParamDef {
        canonical_name: param_names::SSL.as_str(),
        aliases: aliases![],
        value_type: ValueType::Bool,
        additional_value_type: None,
        required: Required::Never,
        default: None,
        sensitive: false,
        description: "Enable or disable SSL/TLS (sets protocol to https or http)",
        deprecated_by: Some("protocol"),
        scopes: &[ParamScope::Connection],
        used_at_connect: true,
        mutable_after_connect: false,
    },
    ParamDef {
        canonical_name: param_names::SERVER_URL.as_str(),
        aliases: aliases![],
        value_type: ValueType::String,
        additional_value_type: None,
        required: Required::Never,
        default: None,
        sensitive: false,
        description: "Full server URL (alternative to host/port/protocol)",
        deprecated_by: None,
        scopes: &[ParamScope::Connection],
        used_at_connect: true,
        mutable_after_connect: false,
    },
    ParamDef {
        canonical_name: param_names::PRESERVE_UNDERSCORES_IN_HOSTNAME.as_str(),
        // JDBC-only `allowUnderscoresInHost` property (case-insensitive).
        aliases: aliases![Jdbc; "allowUnderscoresInHost"],
        value_type: ValueType::Bool,
        additional_value_type: None,
        required: Required::Never,
        default: Some(DefaultValue::Bool(false)),
        sensitive: false,
        description: "Preserve underscores in the hostname derived from the account name",
        deprecated_by: None,
        scopes: &[ParamScope::Connection],
        used_at_connect: true,
        mutable_after_connect: false,
    },
    // ── Auth ────────────────────────────────────────────────────────────
    ParamDef {
        canonical_name: param_names::USER.as_str(),
        // ODBC DSN `UID` (`Snowflake.h`). ODBC-only: the legacy Python connector
        // has no `uid` kwarg.
        aliases: aliases![Odbc; "UID"],
        value_type: ValueType::String,
        additional_value_type: None,
        required: Required::Always,
        default: None,
        sensitive: false,
        description: "Login username",
        deprecated_by: None,
        scopes: &[ParamScope::Connection],
        used_at_connect: true,
        mutable_after_connect: false,
    },
    ParamDef {
        canonical_name: param_names::PASSWORD.as_str(),
        // ODBC DSN `PWD` (`Snowflake.h`). ODBC-only: the legacy Python connector
        // has no `pwd` kwarg.
        aliases: aliases![Odbc; "PWD"],
        value_type: ValueType::String,
        additional_value_type: None,
        required: Required::WhenAuthMethod("SNOWFLAKE_PASSWORD"),
        default: None,
        sensitive: true,
        description: "Login password",
        deprecated_by: None,
        scopes: &[ParamScope::Connection],
        used_at_connect: true,
        mutable_after_connect: false,
    },
    ParamDef {
        canonical_name: param_names::AUTHENTICATOR.as_str(),
        aliases: aliases![],
        value_type: ValueType::String,
        additional_value_type: None,
        required: Required::Never,
        default: None,
        sensitive: false,
        description: "Authenticator type for the connection",
        deprecated_by: None,
        scopes: &[ParamScope::Connection],
        used_at_connect: true,
        mutable_after_connect: false,
    },
    ParamDef {
        canonical_name: param_names::PRIVATE_KEY.as_str(),
        // `PRIV_KEY_BASE64` is the legacy ODBC DSN key (`Snowflake.h`
        // `SF_PRIV_KEY_BASE64_KEY`); `private_key_base64` is the JDBC property
        // (`SFSessionProperty.PRIVATE_KEY_BASE64`) — legacy ODBC never accepted
        // the fully-underscored spelling.
        aliases: &[
            Alias::scoped(Wrapper::Odbc, "PRIV_KEY_BASE64"),
            Alias::scoped(Wrapper::Jdbc, "PRIVATE_KEY_BASE64"),
        ],
        value_type: ValueType::String,
        additional_value_type: Some(ValueType::Bytes),
        required: Required::WhenAuthMethod("SNOWFLAKE_JWT"),
        default: None,
        sensitive: true,
        description: "Private key for key-pair authentication (base64-encoded or PEM)",
        deprecated_by: None,
        scopes: &[ParamScope::Connection],
        used_at_connect: true,
        mutable_after_connect: false,
    },
    ParamDef {
        canonical_name: param_names::PRIVATE_KEY_FILE.as_str(),
        // ODBC DSN `PRIV_KEY_FILE`.
        aliases: aliases![Odbc; "PRIV_KEY_FILE"],
        value_type: ValueType::String,
        additional_value_type: None,
        required: Required::Never,
        default: None,
        sensitive: false,
        description: "Path to private key file for key-pair authentication",
        deprecated_by: None,
        scopes: &[ParamScope::Connection],
        used_at_connect: true,
        mutable_after_connect: false,
    },
    ParamDef {
        canonical_name: param_names::PRIVATE_KEY_PASSWORD.as_str(),
        // `PRIV_KEY_FILE_PWD` / `PRIV_KEY_PWD` are the legacy ODBC DSN
        // passphrase keys (`Snowflake.h`). `private_key_pwd` and
        // `private_key_file_pwd` are JDBC properties (`SFSessionProperty`);
        // `private_key_file_pwd` is also a legacy snowflake-connector-python
        // kwarg, and needs `Python` scope for the TOML loader, which
        // canonicalizes through the registry under the Python flavor rather
        // than through the Python wrapper's generated `_ALIAS_MAP`.
        aliases: &[
            Alias::scoped(Wrapper::Odbc, "PRIV_KEY_FILE_PWD"),
            Alias::scoped(Wrapper::Odbc, "PRIV_KEY_PWD"),
            Alias::scoped(Wrapper::Jdbc, "PRIVATE_KEY_PWD"),
            Alias::scoped(Wrapper::Jdbc, "PRIVATE_KEY_FILE_PWD"),
            Alias::scoped(Wrapper::Python, "PRIVATE_KEY_FILE_PWD"),
        ],
        value_type: ValueType::String,
        additional_value_type: None,
        required: Required::Never,
        default: None,
        sensitive: true,
        description: "Passphrase for encrypted private key",
        deprecated_by: None,
        scopes: &[ParamScope::Connection],
        used_at_connect: true,
        mutable_after_connect: false,
    },
    ParamDef {
        canonical_name: param_names::TOKEN.as_str(),
        aliases: aliases![],
        value_type: ValueType::String,
        additional_value_type: None,
        required: Required::WhenAuthMethod("PROGRAMMATIC_ACCESS_TOKEN"),
        default: None,
        sensitive: true,
        description: "Pre-acquired bearer token (PAT, legacy OAUTH, or OIDC WIF). Alternative to token_file_path",
        deprecated_by: None,
        scopes: &[ParamScope::Connection],
        used_at_connect: true,
        mutable_after_connect: false,
    },
    ParamDef {
        canonical_name: param_names::TOKEN_FILE_PATH.as_str(),
        // `tokenFilePath` is legacy snowflake-connector-nodejs' option spelling
        // (`lib/connection/connection_config.js`). Legacy .NET and JDBC read the
        // snake_case `token_file_path`, which matches the canonical name
        // case-insensitively and so needs no alias. This alias is inert for any
        // wrapper that takes the `Default` presets — see
        // `WrapperPresets::default`.
        aliases: aliases![NodeJs; "tokenFilePath"],
        value_type: ValueType::String,
        additional_value_type: None,
        required: Required::Never,
        default: None,
        sensitive: false,
        description: "Path to a file containing a pre-acquired bearer token (PAT, legacy OAUTH, or OIDC WIF). If both token and token_file_path are set, the file contents are used",
        deprecated_by: None,
        scopes: &[ParamScope::Connection],
        used_at_connect: true,
        mutable_after_connect: false,
    },
    ParamDef {
        canonical_name: param_names::SESSION_TOKEN.as_str(),
        aliases: aliases![],
        value_type: ValueType::String,
        additional_value_type: None,
        required: Required::Never,
        default: None,
        sensitive: true,
        description: "Pre-acquired session token for session token authentication",
        deprecated_by: None,
        scopes: &[ParamScope::Connection],
        used_at_connect: true,
        mutable_after_connect: false,
    },
    ParamDef {
        canonical_name: param_names::MASTER_TOKEN.as_str(),
        aliases: aliases![],
        value_type: ValueType::String,
        additional_value_type: None,
        required: Required::Never,
        default: None,
        sensitive: true,
        description: "Pre-acquired master token for session token authentication",
        deprecated_by: None,
        scopes: &[ParamScope::Connection],
        used_at_connect: true,
        mutable_after_connect: false,
    },
    ParamDef {
        canonical_name: param_names::MASTER_VALIDITY_IN_SECONDS.as_str(),
        aliases: aliases![],
        value_type: ValueType::Int,
        additional_value_type: None,
        required: Required::Never,
        default: None,
        sensitive: false,
        description: "Remaining validity in seconds for the master token (session token auth)",
        deprecated_by: None,
        scopes: &[ParamScope::Connection],
        used_at_connect: true,
        mutable_after_connect: false,
    },
    ParamDef {
        canonical_name: param_names::PASSCODE.as_str(),
        aliases: aliases![],
        value_type: ValueType::String,
        additional_value_type: None,
        required: Required::Never,
        default: None,
        sensitive: true,
        description: "MFA passcode for USERNAME_PASSWORD_MFA authentication",
        deprecated_by: None,
        scopes: &[ParamScope::Connection],
        used_at_connect: true,
        mutable_after_connect: false,
    },
    ParamDef {
        canonical_name: param_names::PASSCODE_IN_PASSWORD.as_str(),
        // `PASSCODE_IN_PASSWORD` is the legacy snowflake-connector-python kwarg
        // spelling. It needs `Python` scope for the TOML loader: the canonical
        // camelCase name does *not* match `passcode_in_password`
        // case-insensitively (the underscores differ), so a
        // `config.toml`/`connections.toml` profile would otherwise fail to
        // canonicalize. Legacy ODBC's DSN key is `PASSCODEINPASSWORD` (no
        // separators) and is rewritten wrapper-side in
        // `odbc/src/api/connection.rs`, so no ODBC alias belongs here.
        aliases: aliases![Python; "PASSCODE_IN_PASSWORD"],
        value_type: ValueType::Bool,
        additional_value_type: None,
        required: Required::Never,
        default: Some(DefaultValue::Bool(false)),
        sensitive: false,
        description: "Whether the MFA passcode is appended to the password",
        deprecated_by: None,
        scopes: &[ParamScope::Connection],
        used_at_connect: true,
        mutable_after_connect: false,
    },
    ParamDef {
        canonical_name: param_names::CLIENT_STORE_TEMPORARY_CREDENTIAL.as_str(),
        // JDBC-only camelCase property.
        aliases: aliases![Jdbc; "clientStoreTemporaryCredential"],
        value_type: ValueType::Bool,
        additional_value_type: None,
        required: Required::Never,
        default: Some(DefaultValue::Bool(false)),
        sensitive: false,
        description: "Enable MFA token caching for USERNAME_PASSWORD_MFA authentication",
        deprecated_by: None,
        scopes: &[ParamScope::Connection],
        used_at_connect: true,
        mutable_after_connect: false,
    },
    ParamDef {
        canonical_name: param_names::ENABLE_PUT_GET.as_str(),
        // JDBC `SFSessionProperty.ENABLE_PUT_GET`. `ParameterKeyNormalizer` does
        // not carry this key, so the spelling reaches core verbatim and this
        // alias is what resolves it.
        aliases: aliases![Jdbc; "enablePutGet"],
        value_type: ValueType::Bool,
        additional_value_type: None,
        required: Required::Never,
        default: Some(DefaultValue::Bool(true)),
        sensitive: false,
        description: "JDBC-only. When false, client-side PUT/GET file transfers are disabled",
        deprecated_by: None,
        scopes: &[ParamScope::Connection],
        used_at_connect: true,
        mutable_after_connect: false,
    },
    ParamDef {
        canonical_name: param_names::DISABLE_PARALLEL_USER_PROMPT.as_str(),
        aliases: aliases![],
        value_type: ValueType::Bool,
        additional_value_type: None,
        required: Required::Never,
        default: Some(DefaultValue::Bool(true)),
        sensitive: false,
        description: "When true (default), enables process-global serialization of interactive auth \
                      prompts (external browser, MFA, OAuth) so that only one prompt is shown per \
                      <user, host> when clientStoreTemporaryCredential is enabled. Set to false to \
                      allow each concurrent connection to show its own prompt.",
        deprecated_by: None,
        scopes: &[ParamScope::Connection],
        used_at_connect: true,
        mutable_after_connect: false,
    },
    ParamDef {
        canonical_name: param_names::DISABLE_QUERY_CONTEXT_CACHE.as_str(),
        // Legacy ODBC (libsnowflakeclient `connection.c`) and legacy .NET
        // (`SFSessionProperty`) spell this `DISABLEQUERYCONTEXTCACHE`; legacy JDBC
        // (`SFSessionProperty`) and snowflake-connector-nodejs
        // (`connection_config.js`) spell it `disableQueryContextCache`. The two
        // differ only by case and resolve identically. Legacy Python's
        // `disable_query_context_cache` matches the canonical name, so `Python`
        // is absent here.
        aliases: &[
            Alias::scoped(Wrapper::Odbc, "DISABLEQUERYCONTEXTCACHE"),
            Alias::scoped(Wrapper::DotNet, "DISABLEQUERYCONTEXTCACHE"),
            Alias::scoped(Wrapper::Jdbc, "disableQueryContextCache"),
            Alias::scoped(Wrapper::NodeJs, "disableQueryContextCache"),
        ],
        value_type: ValueType::Bool,
        additional_value_type: None,
        required: Required::Never,
        default: Some(DefaultValue::Bool(false)),
        sensitive: false,
        description: "When true, disables the client-side query context cache. \
                      No context is sent in requests and server-returned context is ignored.",
        deprecated_by: None,
        scopes: &[ParamScope::Connection],
        used_at_connect: false,
        mutable_after_connect: false,
    },
    ParamDef {
        canonical_name: param_names::AUTHENTICATION_TIMEOUT.as_str(),
        // ODBC's LOGIN_TIMEOUT historically means authentication_timeout (it is an
        // auth-retry budget, not a socket timeout); for the other wrappers
        // `LOGIN_TIMEOUT` keeps matching the canonical `login_timeout` parameter
        // below, case-insensitively.
        aliases: &[Alias::scoped(Wrapper::Odbc, "LOGIN_TIMEOUT")],
        value_type: ValueType::Int,
        additional_value_type: None,
        required: Required::Never,
        default: Some(DefaultValue::Int(120)),
        sensitive: false,
        description: "Timeout in seconds for native Okta SSO authentication",
        deprecated_by: None,
        scopes: &[ParamScope::Connection],
        used_at_connect: true,
        mutable_after_connect: false,
    },
    ParamDef {
        canonical_name: param_names::OKTA_USERNAME.as_str(),
        aliases: aliases![],
        value_type: ValueType::String,
        additional_value_type: None,
        required: Required::Never,
        default: None,
        sensitive: false,
        description: "Okta username (defaults to the Snowflake user if omitted)",
        deprecated_by: None,
        scopes: &[ParamScope::Connection],
        used_at_connect: true,
        mutable_after_connect: false,
    },
    ParamDef {
        canonical_name: param_names::DISABLE_SAML_URL_CHECK.as_str(),
        aliases: aliases![],
        value_type: ValueType::Bool,
        additional_value_type: None,
        required: Required::Never,
        default: Some(DefaultValue::Bool(false)),
        sensitive: false,
        description: "Skip the Okta SAML URL host-match safety check",
        deprecated_by: None,
        scopes: &[ParamScope::Connection],
        used_at_connect: true,
        mutable_after_connect: false,
    },
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
    ParamDef {
        canonical_name: param_names::OAUTH_CLIENT_ID.as_str(),
        // `OAUTH_CLIENT_ID` is the canonical spelling (case-insensitive match);
        // the camelCase form is the JDBC-only `SFSessionProperty` key.
        aliases: aliases![Jdbc; "oauthClientId"],
        value_type: ValueType::String,
        additional_value_type: None,
        required: Required::Never,
        default: None,
        sensitive: false,
        description: "OAuth client identifier (LOCAL_APPLICATION when Snowflake is the IdP)",
        deprecated_by: None,
        scopes: &[ParamScope::Connection],
        used_at_connect: true,
        mutable_after_connect: false,
    },
    ParamDef {
        canonical_name: param_names::OAUTH_CLIENT_SECRET.as_str(),
        aliases: aliases![Jdbc; "oauthClientSecret"],
        value_type: ValueType::String,
        additional_value_type: None,
        required: Required::Never,
        default: None,
        sensitive: true,
        description: "OAuth client secret (redacted from logs)",
        deprecated_by: None,
        scopes: &[ParamScope::Connection],
        used_at_connect: true,
        mutable_after_connect: false,
    },
    ParamDef {
        canonical_name: param_names::OAUTH_AUTHORIZATION_URL.as_str(),
        aliases: aliases![Jdbc; "oauthAuthorizationUrl"],
        value_type: ValueType::String,
        additional_value_type: None,
        required: Required::Never,
        default: None,
        sensitive: false,
        description: "IdP authorization endpoint (defaults to https://{host}/oauth/authorize)",
        deprecated_by: None,
        scopes: &[ParamScope::Connection],
        used_at_connect: true,
        mutable_after_connect: false,
    },
    ParamDef {
        canonical_name: param_names::OAUTH_TOKEN_REQUEST_URL.as_str(),
        // `OAUTH_TOKEN_REQUEST_URL` matches the canonical name case-insensitively,
        // which is also the legacy Python kwarg and the legacy ODBC DSN key
        // (`Snowflake.h` `SF_OAUTH_TOKEN_REQUEST_URL_KEY`). Only the camelCase
        // JDBC property needs an alias; the shorter `OAUTH_TOKEN_URL` was UD-only
        // leniency (no such kwarg in the legacy connector) and is gone.
        aliases: aliases![Jdbc; "oauthTokenRequestUrl"],
        value_type: ValueType::String,
        additional_value_type: None,
        required: Required::WhenAuthMethod("OAUTH_CLIENT_CREDENTIALS"),
        default: None,
        sensitive: false,
        description: "IdP token endpoint (CC only; defaults to https://{host}/oauth/token-request for AC)",
        deprecated_by: None,
        scopes: &[ParamScope::Connection],
        used_at_connect: true,
        mutable_after_connect: false,
    },
    ParamDef {
        canonical_name: param_names::OAUTH_REDIRECT_URI.as_str(),
        aliases: aliases![Jdbc; "oauthRedirectUri"],
        value_type: ValueType::String,
        additional_value_type: None,
        required: Required::Never,
        default: None,
        sensitive: false,
        description: "Loopback redirect URI advertised to the IdP (defaults to http://127.0.0.1:<random>)",
        deprecated_by: None,
        scopes: &[ParamScope::Connection],
        used_at_connect: true,
        mutable_after_connect: false,
    },
    ParamDef {
        canonical_name: param_names::OAUTH_SCOPE.as_str(),
        aliases: aliases![Jdbc; "oauthScope"],
        value_type: ValueType::String,
        additional_value_type: None,
        required: Required::Never,
        default: None,
        sensitive: false,
        description: "OAuth scope (space-separated; defaults to session:role:<role>)",
        deprecated_by: None,
        scopes: &[ParamScope::Connection],
        used_at_connect: true,
        mutable_after_connect: false,
    },
    ParamDef {
        canonical_name: param_names::OAUTH_ENABLE_SINGLE_USE_REFRESH_TOKENS.as_str(),
        aliases: aliases![Jdbc; "oauthEnableSingleUseRefreshTokens"],
        value_type: ValueType::Bool,
        additional_value_type: None,
        required: Required::Never,
        default: Some(DefaultValue::Bool(false)),
        sensitive: false,
        description: "Request single-use refresh-token rotation (Snowflake-IdP only)",
        deprecated_by: None,
        scopes: &[ParamScope::Connection],
        used_at_connect: true,
        mutable_after_connect: false,
    },
    ParamDef {
        canonical_name: param_names::OAUTH_DISABLE_PKCE.as_str(),
        aliases: aliases![],
        value_type: ValueType::Bool,
        additional_value_type: None,
        required: Required::Never,
        default: Some(DefaultValue::Bool(false)),
        sensitive: false,
        description: "Disable PKCE S256 challenge for OAUTH_AUTHORIZATION_CODE (Python-compatible escape hatch)",
        deprecated_by: None,
        scopes: &[ParamScope::Connection],
        used_at_connect: true,
        mutable_after_connect: false,
    },
    ParamDef {
        canonical_name: param_names::OAUTH_ENABLE_DPOP.as_str(),
        aliases: aliases![],
        value_type: ValueType::Bool,
        additional_value_type: None,
        required: Required::Never,
        default: Some(DefaultValue::Bool(false)),
        sensitive: false,
        description: "Enable RFC 9449 DPoP proof-of-possession (JDBC-compatible)",
        deprecated_by: None,
        scopes: &[ParamScope::Connection],
        used_at_connect: true,
        mutable_after_connect: false,
    },
    ParamDef {
        canonical_name: param_names::OAUTH_CREDENTIALS_IN_BODY.as_str(),
        aliases: aliases![],
        value_type: ValueType::Bool,
        additional_value_type: None,
        required: Required::Never,
        default: Some(DefaultValue::Bool(false)),
        sensitive: false,
        description: "Send client_id/client_secret in the OAUTH_CLIENT_CREDENTIALS token request body (client_secret_post) instead of the HTTP Basic header",
        deprecated_by: None,
        scopes: &[ParamScope::Connection],
        used_at_connect: true,
        mutable_after_connect: false,
    },
    ParamDef {
        canonical_name: param_names::OAUTH_DISABLE_CONSOLE_LOGIN.as_str(),
        aliases: aliases![],
        value_type: ValueType::Bool,
        additional_value_type: None,
        required: Required::Never,
        default: Some(DefaultValue::Bool(false)),
        sensitive: false,
        description: "Disable EXTERNALBROWSER console-login (JDBC parity; does not gate OAuth)",
        deprecated_by: None,
        scopes: &[ParamScope::Connection],
        used_at_connect: true,
        mutable_after_connect: false,
    },
    // ── Session ─────────────────────────────────────────────────────────
    ParamDef {
        canonical_name: param_names::DATABASE.as_str(),
        aliases: aliases![],
        value_type: ValueType::String,
        additional_value_type: None,
        required: Required::Never,
        default: None,
        sensitive: false,
        description: "Default database to use",
        deprecated_by: None,
        scopes: &[ParamScope::Session],
        used_at_connect: true,
        mutable_after_connect: true,
    },
    ParamDef {
        canonical_name: param_names::SCHEMA.as_str(),
        aliases: aliases![],
        value_type: ValueType::String,
        additional_value_type: None,
        required: Required::Never,
        default: None,
        sensitive: false,
        description: "Default schema to use",
        deprecated_by: None,
        scopes: &[ParamScope::Session],
        used_at_connect: true,
        mutable_after_connect: true,
    },
    ParamDef {
        canonical_name: param_names::WAREHOUSE.as_str(),
        aliases: aliases![],
        value_type: ValueType::String,
        additional_value_type: None,
        required: Required::Never,
        default: None,
        sensitive: false,
        description: "Default warehouse to use",
        deprecated_by: None,
        scopes: &[ParamScope::Session],
        used_at_connect: true,
        mutable_after_connect: true,
    },
    ParamDef {
        canonical_name: param_names::ROLE.as_str(),
        aliases: aliases![],
        value_type: ValueType::String,
        additional_value_type: None,
        required: Required::Never,
        default: None,
        sensitive: false,
        description: "Default role to use",
        deprecated_by: None,
        scopes: &[ParamScope::Session],
        used_at_connect: true,
        mutable_after_connect: true,
    },
    ParamDef {
        canonical_name: param_names::SECONDARY_ROLES.as_str(),
        aliases: &[],
        value_type: ValueType::String,
        additional_value_type: None,
        required: Required::Never,
        default: None,
        sensitive: false,
        description: "Secondary-roles activation mode sent at login (e.g. ALL or NONE)",
        deprecated_by: None,
        scopes: &[ParamScope::Connection],
        used_at_connect: true,
        mutable_after_connect: false,
    },
    // ── TLS ─────────────────────────────────────────────────────────────
    ParamDef {
        canonical_name: param_names::CUSTOM_ROOT_STORE_PATH.as_str(),
        // No alias: legacy ODBC had no custom-root-store DSN key (only `SSL`), and
        // the `TLS_`-prefixed spelling had no users — the canonical name resolves
        // for every wrapper case-insensitively.
        aliases: aliases![],
        value_type: ValueType::String,
        additional_value_type: None,
        required: Required::Never,
        default: None,
        sensitive: false,
        description: "Path to custom root certificate store",
        deprecated_by: None,
        scopes: &[ParamScope::Connection],
        used_at_connect: true,
        mutable_after_connect: false,
    },
    ParamDef {
        canonical_name: param_names::VERIFY_HOSTNAME.as_str(),
        // No alias: no legacy TLS-verification DSN key existed, and the
        // `TLS_VERIFY_HOSTNAME` spelling had no users.
        aliases: aliases![],
        value_type: ValueType::Bool,
        additional_value_type: None,
        required: Required::Never,
        default: Some(DefaultValue::Bool(true)),
        sensitive: false,
        description: "Whether to verify the server hostname in TLS",
        deprecated_by: None,
        scopes: &[ParamScope::Connection],
        used_at_connect: true,
        mutable_after_connect: false,
    },
    ParamDef {
        canonical_name: param_names::VERIFY_CERTIFICATES.as_str(),
        // No alias: see `verify_hostname` above.
        aliases: aliases![],
        value_type: ValueType::Bool,
        additional_value_type: None,
        required: Required::Never,
        default: Some(DefaultValue::Bool(true)),
        sensitive: false,
        description: "Whether to verify TLS certificates",
        deprecated_by: None,
        scopes: &[ParamScope::Connection],
        used_at_connect: true,
        mutable_after_connect: false,
    },
    ParamDef {
        canonical_name: param_names::TLS_SKIP_VERIFY.as_str(),
        aliases: aliases![],
        value_type: ValueType::Bool,
        additional_value_type: None,
        required: Required::Never,
        default: Some(DefaultValue::Bool(false)),
        sensitive: false,
        description: "Skip all TLS verification with a single switch: disables both certificate and hostname checks (and, since certificate verification is off, CRL revocation checks are bypassed too). Insecure; intended for testing only",
        deprecated_by: None,
        scopes: &[ParamScope::Connection],
        used_at_connect: true,
        mutable_after_connect: false,
    },
    // TLS protocol-version window.
    ParamDef {
        canonical_name: param_names::MIN_TLS_VERSION.as_str(),
        aliases: aliases![],
        value_type: ValueType::String,
        additional_value_type: None,
        required: Required::Never,
        default: Some(DefaultValue::String("tls12")),
        sensitive: false,
        description: "Minimum TLS protocol version to negotiate (tls12 or tls13)",
        deprecated_by: None,
        scopes: &[ParamScope::Connection],
        used_at_connect: true,
        mutable_after_connect: false,
    },
    ParamDef {
        canonical_name: param_names::MAX_TLS_VERSION.as_str(),
        aliases: aliases![],
        value_type: ValueType::String,
        additional_value_type: None,
        required: Required::Never,
        default: Some(DefaultValue::String("tls13")),
        sensitive: false,
        description: "Maximum TLS protocol version to negotiate (tls12 or tls13)",
        deprecated_by: None,
        scopes: &[ParamScope::Connection],
        used_at_connect: true,
        mutable_after_connect: false,
    },
    // ── CRL ─────────────────────────────────────────────────────────────
    ParamDef {
        canonical_name: param_names::CRL_CHECK_MODE.as_str(),
        // UD-ODBC's own DSN spellings for CRL checking (legacy snowflake-odbc
        // spelled this family `CRL_CHECK` / `CRL_ADVISORY`, `Snowflake.h:197-202`;
        // UD has not adopted those names). Wired wrapper-side for value
        // normalization and exercised by `odbc_tests/tests/e2e/tls/crl_enabled.cpp`.
        // Python uses the `cert_revocation_check_mode` legacy kwarg, rewritten
        // wrapper-side by `_LEGACY_REWRITES`.
        aliases: aliases![Odbc; "CRL_MODE", "CRL_ENABLED"],
        value_type: ValueType::String,
        additional_value_type: None,
        required: Required::Never,
        default: Some(DefaultValue::String("DISABLED")),
        sensitive: false,
        description: "Certificate revocation check mode (DISABLED, ENABLED, ADVISORY)",
        deprecated_by: None,
        scopes: &[ParamScope::Connection],
        used_at_connect: true,
        mutable_after_connect: false,
    },
    ParamDef {
        canonical_name: param_names::CRL_ENABLE_DISK_CACHING.as_str(),
        aliases: aliases![],
        value_type: ValueType::Bool,
        additional_value_type: None,
        required: Required::Never,
        default: Some(DefaultValue::Bool(true)),
        sensitive: false,
        description: "Enable disk caching for CRL responses",
        deprecated_by: None,
        scopes: &[ParamScope::Connection],
        used_at_connect: true,
        mutable_after_connect: false,
    },
    ParamDef {
        canonical_name: param_names::CRL_ENABLE_MEMORY_CACHING.as_str(),
        aliases: aliases![],
        value_type: ValueType::Bool,
        additional_value_type: None,
        required: Required::Never,
        default: Some(DefaultValue::Bool(true)),
        sensitive: false,
        description: "Enable in-memory caching for CRL responses",
        deprecated_by: None,
        scopes: &[ParamScope::Connection],
        used_at_connect: true,
        mutable_after_connect: false,
    },
    ParamDef {
        canonical_name: param_names::CRL_CACHE_DIR.as_str(),
        aliases: aliases![],
        value_type: ValueType::String,
        additional_value_type: None,
        required: Required::Never,
        default: None,
        sensitive: false,
        description: "Directory for CRL cache files",
        deprecated_by: None,
        scopes: &[ParamScope::Connection],
        used_at_connect: true,
        mutable_after_connect: false,
    },
    ParamDef {
        canonical_name: param_names::CRL_MAX_DOWNLOAD_SIZE.as_str(),
        aliases: aliases![],
        value_type: ValueType::Int,
        additional_value_type: None,
        required: Required::Never,
        default: Some(DefaultValue::Int(20 * 1024 * 1024)),
        sensitive: false,
        description: "Maximum CRL download size in bytes before the download is aborted",
        deprecated_by: None,
        scopes: &[ParamScope::Connection],
        used_at_connect: true,
        mutable_after_connect: false,
    },
    ParamDef {
        canonical_name: param_names::CRL_VALIDITY_TIME.as_str(),
        aliases: aliases![],
        value_type: ValueType::Int,
        additional_value_type: None,
        required: Required::Never,
        default: Some(DefaultValue::Int(86400)),
        sensitive: false,
        description: "Maximum age in seconds of a cached CRL before it is re-fetched",
        deprecated_by: None,
        scopes: &[ParamScope::Connection],
        used_at_connect: true,
        mutable_after_connect: false,
    },
    ParamDef {
        canonical_name: param_names::CRL_ON_DISK_CACHE_REMOVAL_DELAY.as_str(),
        aliases: aliases![],
        value_type: ValueType::Int,
        additional_value_type: None,
        required: Required::Never,
        default: Some(DefaultValue::Int(604800)),
        sensitive: false,
        description: "Delay in seconds after a CRL's nextUpdate before it is purged from the on-disk cache",
        deprecated_by: None,
        scopes: &[ParamScope::Connection],
        used_at_connect: true,
        mutable_after_connect: false,
    },
    ParamDef {
        canonical_name: param_names::CRL_CACHE_CLEANUP_INTERVAL.as_str(),
        aliases: aliases![],
        value_type: ValueType::Int,
        additional_value_type: None,
        required: Required::Never,
        default: Some(DefaultValue::Int(3600)),
        sensitive: false,
        description: "Interval in seconds between background CRL cache cleanup passes",
        deprecated_by: None,
        scopes: &[ParamScope::Connection],
        used_at_connect: true,
        mutable_after_connect: false,
    },
    ParamDef {
        canonical_name: param_names::CRL_CACHE_START_CLEANUP.as_str(),
        aliases: aliases![],
        value_type: ValueType::Bool,
        additional_value_type: None,
        required: Required::Never,
        default: Some(DefaultValue::Bool(false)),
        sensitive: false,
        description: "Run the background CRL cache cleanup task",
        deprecated_by: None,
        scopes: &[ParamScope::Connection],
        used_at_connect: true,
        mutable_after_connect: false,
    },
    ParamDef {
        canonical_name: param_names::CRL_UNSAFE_SKIP_FILE_PERMISSIONS_CHECK.as_str(),
        aliases: aliases![],
        value_type: ValueType::Bool,
        additional_value_type: None,
        required: Required::Never,
        default: Some(DefaultValue::Bool(false)),
        sensitive: false,
        description: "Skip verification that on-disk CRL cache files and directory are owner-only (0600/0700)",
        deprecated_by: None,
        scopes: &[ParamScope::Connection],
        used_at_connect: true,
        mutable_after_connect: false,
    },
    ParamDef {
        canonical_name: param_names::CRL_ALLOW_CERTIFICATES_WITHOUT_CRL_URL.as_str(),
        aliases: aliases![],
        value_type: ValueType::Bool,
        additional_value_type: None,
        required: Required::Never,
        default: Some(DefaultValue::Bool(false)),
        sensitive: false,
        description: "Allow certificates that do not include a CRL distribution URL",
        deprecated_by: None,
        scopes: &[ParamScope::Connection],
        used_at_connect: true,
        mutable_after_connect: false,
    },
    ParamDef {
        canonical_name: param_names::CRL_HTTP_TIMEOUT.as_str(),
        aliases: aliases![],
        value_type: ValueType::Int,
        additional_value_type: None,
        required: Required::Never,
        default: Some(DefaultValue::Int(10)),
        sensitive: false,
        description: "HTTP timeout in seconds for CRL endpoint requests",
        deprecated_by: None,
        scopes: &[ParamScope::Connection],
        used_at_connect: true,
        mutable_after_connect: false,
    },
    ParamDef {
        canonical_name: param_names::CRL_CONNECTION_TIMEOUT.as_str(),
        aliases: aliases![],
        value_type: ValueType::Int,
        additional_value_type: None,
        required: Required::Never,
        default: Some(DefaultValue::Int(10)),
        sensitive: false,
        description: "Connection timeout in seconds for CRL endpoints",
        deprecated_by: None,
        scopes: &[ParamScope::Connection],
        used_at_connect: true,
        mutable_after_connect: false,
    },
    // ── Client ──────────────────────────────────────────────────────────
    ParamDef {
        canonical_name: param_names::CONNECTION_NAME.as_str(),
        aliases: aliases![],
        value_type: ValueType::String,
        additional_value_type: None,
        required: Required::Never,
        default: None,
        sensitive: false,
        description: "Named connection to load from TOML configuration files",
        deprecated_by: None,
        scopes: &[ParamScope::Connection],
        used_at_connect: false,
        mutable_after_connect: false,
    },
    ParamDef {
        canonical_name: param_names::LOG_MAX_QUERY_LENGTH.as_str(),
        aliases: aliases![],
        value_type: ValueType::Int,
        additional_value_type: None,
        required: Required::Never,
        default: Some(DefaultValue::Int(80)),
        sensitive: false,
        description: "Maximum number of characters of a query string to include in log messages",
        deprecated_by: None,
        scopes: &[ParamScope::Connection],
        used_at_connect: false,
        mutable_after_connect: false,
    },
    ParamDef {
        canonical_name: param_names::LOG_QUERY_TEXT.as_str(),
        aliases: aliases![],
        value_type: ValueType::Bool,
        additional_value_type: Some(ValueType::String),
        required: Required::Never,
        default: Some(DefaultValue::Bool(false)),
        sensitive: false,
        description: "Include the (truncated) SQL text in INFO query logs",
        deprecated_by: None,
        scopes: &[ParamScope::Connection],
        used_at_connect: false,
        mutable_after_connect: false,
    },
    ParamDef {
        canonical_name: param_names::LOG_QUERY_PARAMETERS.as_str(),
        aliases: aliases![],
        value_type: ValueType::Bool,
        additional_value_type: Some(ValueType::String),
        required: Required::Never,
        default: Some(DefaultValue::Bool(false)),
        sensitive: false,
        description: "Include the (truncated) JSON bindings in INFO query logs (requires log_query_text)",
        deprecated_by: None,
        scopes: &[ParamScope::Connection],
        used_at_connect: false,
        mutable_after_connect: false,
    },
    // ── Logout ────────────────────────────────────────────────────────
    ParamDef {
        canonical_name: param_names::SERVER_SESSION_KEEP_ALIVE.as_str(),
        aliases: aliases![],
        value_type: ValueType::Bool,
        additional_value_type: None,
        required: Required::Never,
        default: None,
        sensitive: false,
        description: "Control server session lifecycle: true=keep alive, false=always logout, null=auto-detect",
        deprecated_by: None,
        scopes: &[ParamScope::Connection],
        used_at_connect: false,
        mutable_after_connect: false,
    },
    ParamDef {
        canonical_name: param_names::ENABLE_SERVER_SESSION_KEEP_ALIVE_AUTO_DETECTION.as_str(),
        aliases: aliases![],
        value_type: ValueType::Bool,
        additional_value_type: None,
        required: Required::Never,
        default: None,
        sensitive: false,
        description: "Enable auto-detection of async queries before logout (SNOW-2314152)",
        deprecated_by: None,
        scopes: &[ParamScope::Connection],
        used_at_connect: false,
        mutable_after_connect: false,
    },
    ParamDef {
        canonical_name: param_names::LOGOUT_ERROR_STRATEGY.as_str(),
        aliases: aliases![],
        value_type: ValueType::String,
        additional_value_type: None,
        required: Required::Never,
        default: None,
        sensitive: false,
        description: "Error handling strategy for logout: 'best_effort' or 'strict'",
        deprecated_by: None,
        scopes: &[ParamScope::Connection],
        used_at_connect: false,
        mutable_after_connect: true,
    },
    ParamDef {
        canonical_name: param_names::LOGOUT_TOTAL_TIMEOUT_SECONDS.as_str(),
        aliases: aliases![],
        value_type: ValueType::Int,
        additional_value_type: None,
        required: Required::Never,
        default: None,
        sensitive: false,
        description: "Total timeout budget for logout operation including retries",
        deprecated_by: None,
        scopes: &[ParamScope::Connection],
        used_at_connect: false,
        mutable_after_connect: true,
    },
    ParamDef {
        canonical_name: param_names::LOGOUT_MAX_ATTEMPTS.as_str(),
        aliases: aliases![],
        value_type: ValueType::Int,
        additional_value_type: None,
        required: Required::Never,
        default: None,
        sensitive: false,
        description: "Maximum total attempts for logout (1 = no retry, 3 = 2 retries)",
        deprecated_by: None,
        scopes: &[ParamScope::Connection],
        used_at_connect: false,
        mutable_after_connect: true,
    },
    ParamDef {
        canonical_name: param_names::LOGOUT_REQUEST_TIMEOUT_SECONDS.as_str(),
        aliases: aliases![],
        value_type: ValueType::Int,
        additional_value_type: None,
        required: Required::Never,
        default: None,
        sensitive: false,
        description: "Per-request socket timeout for individual logout attempts",
        deprecated_by: None,
        scopes: &[ParamScope::Connection],
        used_at_connect: false,
        mutable_after_connect: true,
    },
    ParamDef {
        canonical_name: param_names::RETRY_MAX_ATTEMPTS.as_str(),
        aliases: aliases![],
        value_type: ValueType::Int,
        additional_value_type: None,
        required: Required::Never,
        default: Some(DefaultValue::Int(DEFAULT_RETRY_MAX_ATTEMPTS as i64)),
        sensitive: false,
        description: "Maximum total attempts for general HTTP calls (login, query, logout). 1 = no retry",
        deprecated_by: None,
        scopes: &[ParamScope::Connection],
        used_at_connect: false,
        mutable_after_connect: false,
    },
    ParamDef {
        canonical_name: param_names::RETRY_EXTRA_STATUS_CODES.as_str(),
        aliases: aliases![],
        value_type: ValueType::String,
        additional_value_type: None,
        required: Required::Never,
        default: None,
        sensitive: false,
        description: "Additional HTTP status codes (comma-separated) to retry on general HTTP and PUT/GET calls, beyond the built-in 408/429/307/308/5xx set",
        deprecated_by: None,
        scopes: &[ParamScope::Connection],
        used_at_connect: false,
        mutable_after_connect: false,
    },
    ParamDef {
        canonical_name: param_names::PUT_GET_MAX_ATTEMPTS.as_str(),
        aliases: aliases![],
        value_type: ValueType::Int,
        additional_value_type: None,
        required: Required::Never,
        default: Some(DefaultValue::Int(DEFAULT_PUT_GET_MAX_ATTEMPTS as i64)),
        sensitive: false,
        description: "Maximum total attempts for a single PUT/GET file transfer (1 = no retry)",
        deprecated_by: None,
        scopes: &[ParamScope::Connection],
        used_at_connect: false,
        mutable_after_connect: true,
    },
    // ── Retry backoff curve (shared by HTTP and PUT/GET pipelines) ──────
    ParamDef {
        canonical_name: param_names::RETRY_BACKOFF_BASE_MS.as_str(),
        aliases: aliases![],
        value_type: ValueType::Int,
        additional_value_type: None,
        required: Required::Never,
        default: Some(DefaultValue::Int(DEFAULT_RETRY_BACKOFF_BASE_MS as i64)),
        sensitive: false,
        description: "Initial exponential-backoff delay in milliseconds between retry attempts",
        deprecated_by: None,
        scopes: &[ParamScope::Connection],
        used_at_connect: false,
        mutable_after_connect: false,
    },
    ParamDef {
        canonical_name: param_names::RETRY_BACKOFF_CAP_MS.as_str(),
        aliases: aliases![],
        value_type: ValueType::Int,
        additional_value_type: None,
        required: Required::Never,
        default: Some(DefaultValue::Int(DEFAULT_RETRY_BACKOFF_CAP_MS as i64)),
        sensitive: false,
        description: "Maximum exponential-backoff delay in milliseconds between retry attempts",
        deprecated_by: None,
        scopes: &[ParamScope::Connection],
        used_at_connect: false,
        mutable_after_connect: false,
    },
    ParamDef {
        canonical_name: param_names::RETRY_BACKOFF_FACTOR.as_str(),
        aliases: aliases![],
        value_type: ValueType::Double,
        additional_value_type: None,
        required: Required::Never,
        default: Some(DefaultValue::Double(DEFAULT_RETRY_BACKOFF_FACTOR)),
        sensitive: false,
        description: "Multiplier applied to the backoff delay after each retry attempt",
        deprecated_by: None,
        scopes: &[ParamScope::Connection],
        used_at_connect: false,
        mutable_after_connect: false,
    },
    ParamDef {
        canonical_name: param_names::RETRY_BACKOFF_JITTER.as_str(),
        aliases: aliases![],
        value_type: ValueType::String,
        additional_value_type: None,
        required: Required::Never,
        default: Some(DefaultValue::String(DEFAULT_RETRY_BACKOFF_JITTER)),
        sensitive: false,
        description: "Backoff jitter strategy: 'none', 'full', or 'decorrelated'",
        deprecated_by: None,
        scopes: &[ParamScope::Connection],
        used_at_connect: false,
        mutable_after_connect: false,
    },
    // ── Timeout configuration ─────────────────────────────────────────
    ParamDef {
        canonical_name: param_names::LOGIN_TIMEOUT.as_str(),
        // `LOGIN_TIMEOUT` matches this canonical case-insensitively for every
        // wrapper except ODBC, where it is scoped to `authentication_timeout`
        // (see that param's `Alias::scoped(Wrapper::Odbc, "LOGIN_TIMEOUT")`).
        aliases: aliases![],
        value_type: ValueType::Int,
        additional_value_type: None,
        required: Required::Never,
        default: Some(DefaultValue::Int(DEFAULT_LOGIN_TIMEOUT_SECS as i64)),
        sensitive: false,
        description: "Wall-clock timeout in seconds for the entire login operation including retries (0 = no timeout)",
        deprecated_by: None,
        scopes: &[ParamScope::Connection],
        used_at_connect: true,
        mutable_after_connect: false,
    },
    ParamDef {
        canonical_name: param_names::QUERY_TIMEOUT.as_str(),
        aliases: aliases![],
        value_type: ValueType::Int,
        additional_value_type: None,
        required: Required::Never,
        default: Some(DefaultValue::Int(DEFAULT_QUERY_TIMEOUT_SECS as i64)),
        sensitive: false,
        description: "Wall-clock timeout in seconds for query execution including retries (0 = no timeout)",
        deprecated_by: None,
        scopes: &[ParamScope::Connection],
        used_at_connect: true,
        mutable_after_connect: false,
    },
    ParamDef {
        canonical_name: param_names::REQUEST_TIMEOUT.as_str(),
        aliases: aliases![],
        value_type: ValueType::Int,
        additional_value_type: None,
        required: Required::Never,
        default: Some(DefaultValue::Int(DEFAULT_REQUEST_TIMEOUT_SECS as i64)),
        sensitive: false,
        description: "Wall-clock timeout in seconds for all other operations (close session, heartbeat, etc.) including retries (0 = no timeout)",
        deprecated_by: None,
        scopes: &[ParamScope::Connection],
        used_at_connect: false,
        mutable_after_connect: false,
    },
    ParamDef {
        canonical_name: param_names::RETRY_TIMEOUT.as_str(),
        aliases: aliases![],
        value_type: ValueType::Int,
        additional_value_type: None,
        required: Required::Never,
        default: None,
        sensitive: false,
        description: "Per-request timeout in seconds for a single HTTP attempt within a retry loop (0 or absent = no per-request timeout)",
        deprecated_by: None,
        scopes: &[ParamScope::Connection],
        used_at_connect: true,
        mutable_after_connect: false,
    },
    ParamDef {
        canonical_name: param_names::CONNECT_TIMEOUT.as_str(),
        aliases: aliases![],
        value_type: ValueType::Int,
        additional_value_type: None,
        required: Required::Never,
        default: None,
        sensitive: false,
        description: "TCP connect timeout in seconds for the HTTP client (0 or absent = system default)",
        deprecated_by: None,
        scopes: &[ParamScope::Connection],
        used_at_connect: true,
        mutable_after_connect: false,
    },
    ParamDef {
        canonical_name: param_names::UNSAFE_SKIP_CONFIG_FILE_PERMISSIONS_CHECK.as_str(),
        aliases: aliases![],
        value_type: ValueType::Bool,
        additional_value_type: None,
        required: Required::Never,
        default: Some(DefaultValue::Bool(false)),
        sensitive: false,
        description: "When true, skip file permission checks on config.toml and connections.toml \
                      during connection setup. Use in environments where permissions cannot be \
                      controlled (CI runners, containers). Unix-only; ignored on Windows",
        deprecated_by: None,
        scopes: &[ParamScope::Connection],
        used_at_connect: true,
        mutable_after_connect: false,
    },
    ParamDef {
        canonical_name: param_names::UNSAFE_FILE_WRITE.as_str(),
        aliases: aliases![],
        value_type: ValueType::Bool,
        additional_value_type: None,
        required: Required::Never,
        default: Some(DefaultValue::Bool(false)),
        sensitive: false,
        description: "When true, GET downloads use the process umask permissions instead of owner-only \
                      (0600). Unix-only; ignored on Windows",
        deprecated_by: None,
        scopes: &[ParamScope::Connection],
        used_at_connect: false,
        mutable_after_connect: true,
    },
    ParamDef {
        canonical_name: param_names::CLIENT_APP_ID.as_str(),
        aliases: aliases![],
        value_type: ValueType::String,
        additional_value_type: None,
        required: Required::Never,
        default: None,
        sensitive: false,
        description: "Driver identity sent as CLIENT_APP_ID in the login request (e.g. PythonConnector, SnowSQL)",
        deprecated_by: None,
        scopes: &[ParamScope::Connection],
        used_at_connect: false,
        mutable_after_connect: false,
    },
    ParamDef {
        canonical_name: param_names::CLIENT_APP_VERSION.as_str(),
        aliases: aliases![],
        value_type: ValueType::String,
        additional_value_type: None,
        required: Required::Never,
        default: None,
        sensitive: false,
        description: "Driver version sent as CLIENT_APP_VERSION in the login request",
        deprecated_by: None,
        scopes: &[ParamScope::Connection],
        used_at_connect: false,
        mutable_after_connect: false,
    },
    ParamDef {
        canonical_name: param_names::APPLICATION.as_str(),
        aliases: aliases![],
        value_type: ValueType::String,
        additional_value_type: None,
        required: Required::Never,
        default: None,
        sensitive: false,
        description: "User-facing application name sent as CLIENT_ENVIRONMENT.APPLICATION (falls back to client_app_id)",
        deprecated_by: None,
        scopes: &[ParamScope::Connection],
        used_at_connect: false,
        mutable_after_connect: false,
    },
    // ── Statement ──────────────────────────────────────────────────────
    ParamDef {
        canonical_name: param_names::ASYNC_EXECUTION.as_str(),
        aliases: aliases![],
        value_type: ValueType::Bool,
        additional_value_type: None,
        required: Required::Never,
        default: Some(DefaultValue::Bool(false)),
        sensitive: false,
        description: "Execute queries asynchronously",
        deprecated_by: None,
        scopes: &[ParamScope::Statement],
        used_at_connect: false,
        mutable_after_connect: true,
    },
    ParamDef {
        canonical_name: param_names::MULTI_STATEMENT_COUNT.as_str(),
        aliases: aliases![],
        value_type: ValueType::Int,
        additional_value_type: None,
        required: Required::Never,
        default: None,
        sensitive: false,
        description: "Exact number of statements in a multi-statement query",
        deprecated_by: None,
        scopes: &[ParamScope::Statement],
        used_at_connect: false,
        mutable_after_connect: true,
    },
    ParamDef {
        canonical_name: param_names::QUERY_TAG.as_str(),
        aliases: aliases![],
        value_type: ValueType::String,
        additional_value_type: None,
        required: Required::Never,
        default: None,
        sensitive: false,
        description: "String label attached to queries and surfaced in QUERY_HISTORY. \
                      Settable at the session level (connection option or session override, \
                      forwarded as a login session parameter) and overridable per-statement.",
        deprecated_by: None,
        // A session parameter that may also be overridden per-statement.
        scopes: &[ParamScope::Session, ParamScope::Statement],
        used_at_connect: false,
        mutable_after_connect: true,
    },
    ParamDef {
        canonical_name: param_names::SKIP_UPLOAD_ON_CONTENT_MATCH.as_str(),
        aliases: aliases![],
        value_type: ValueType::Bool,
        additional_value_type: None,
        required: Required::Never,
        default: Some(DefaultValue::Bool(false)),
        sensitive: false,
        description: "Skip re-uploading a PUT object when the remote stored digest (S3 x-amz-meta-sfc-digest / Azure x-ms-meta-sfcdigest / GCS x-goog-meta-sfc-digest) equals the local SHA-256. Optimization for racing concurrent uploaders; only meaningful when overwrite=true. Set per-statement via statement_set_options before each execute. Client-only, never forwarded to GS.",
        deprecated_by: None,
        scopes: &[ParamScope::Statement],
        used_at_connect: false,
        mutable_after_connect: true,
    },
    ParamDef {
        canonical_name: param_names::PUT_FASTFAIL.as_str(),
        aliases: aliases![],
        value_type: ValueType::Bool,
        additional_value_type: None,
        required: Required::Never,
        // No registry default: unset must resolve to `None` so the dispatch
        // site can fall back to `WrapperPresets::put_get_fastfail_default`
        // (true for Python/JDBC, false for ODBC) instead of a fixed value.
        default: None,
        sensitive: false,
        description: "Controls whether a PUT batch stops at the first failing file (true, fail-fast) or attempts every file and reports failures as ERROR-status rows in the result set (false, collect-all). Defaults to the active wrapper's preset when unset. Mirrors old ODBC's PUT_FASTFAIL connection attribute. Set per-statement via statement_set_options before each execute. Client-only, never forwarded to GS.",
        deprecated_by: None,
        scopes: &[
            ParamScope::Connection,
            ParamScope::Session,
            ParamScope::Statement,
        ],
        used_at_connect: false,
        mutable_after_connect: true,
    },
    ParamDef {
        canonical_name: param_names::GET_FASTFAIL.as_str(),
        aliases: aliases![],
        value_type: ValueType::Bool,
        additional_value_type: None,
        required: Required::Never,
        // See PUT_FASTFAIL above: `None` is load-bearing, not an oversight.
        default: None,
        sensitive: false,
        description: "Controls whether a GET batch stops at the first failing file (true, fail-fast) or attempts every file and reports failures as ERROR-status rows in the result set (false, collect-all). Defaults to the active wrapper's preset when unset. Mirrors old ODBC's GET_FASTFAIL connection attribute. Set per-statement via statement_set_options before each execute. Client-only, never forwarded to GS.",
        deprecated_by: None,
        scopes: &[
            ParamScope::Connection,
            ParamScope::Session,
            ParamScope::Statement,
        ],
        used_at_connect: false,
        mutable_after_connect: true,
    },
    // ── Prefetch ───────────────────────────────────────────────────────
    ParamDef {
        canonical_name: param_names::CLIENT_PREFETCH_THREADS.as_str(),
        aliases: aliases![],
        value_type: ValueType::Int,
        additional_value_type: None,
        required: Required::Never,
        default: Some(DefaultValue::Int(4)),
        sensitive: false,
        description: "Number of concurrent chunk prefetch threads for result set downloading",
        deprecated_by: None,
        scopes: &[ParamScope::Session],
        used_at_connect: true,
        mutable_after_connect: true,
    },
    ParamDef {
        canonical_name: param_names::CLIENT_MEMORY_LIMIT.as_str(),
        aliases: aliases![],
        value_type: ValueType::Int,
        additional_value_type: None,
        required: Required::Never,
        default: Some(DefaultValue::Int(1536)),
        sensitive: false,
        description: "Memory budget in MB for chunk prefetch buffer (0 = unlimited)",
        deprecated_by: None,
        scopes: &[ParamScope::Session],
        used_at_connect: false,
        mutable_after_connect: true,
    },
    // ── Session keep-alive ─────────────────────────────────────────────
    ParamDef {
        canonical_name: param_names::CLIENT_SESSION_KEEP_ALIVE.as_str(),
        aliases: aliases![],
        value_type: ValueType::Bool,
        additional_value_type: None,
        required: Required::Never,
        default: Some(DefaultValue::Bool(false)),
        sensitive: false,
        description: "Keep the session alive with periodic heartbeat requests",
        deprecated_by: None,
        scopes: &[ParamScope::Session],
        used_at_connect: true,
        mutable_after_connect: false,
    },
    ParamDef {
        canonical_name: param_names::CLIENT_SESSION_KEEP_ALIVE_HEARTBEAT_FREQUENCY.as_str(),
        aliases: aliases![],
        value_type: ValueType::Int,
        additional_value_type: None,
        required: Required::Never,
        default: None,
        sensitive: false,
        description: "Heartbeat frequency in seconds (clamped to interval master_token_validity/16..master_token_validity/4)",
        deprecated_by: None,
        scopes: &[ParamScope::Session],
        used_at_connect: true,
        mutable_after_connect: false,
    },
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
    ParamDef {
        canonical_name: param_names::USE_S3_REGIONAL_URL.as_str(),
        aliases: aliases![Python; "ENABLE_STAGE_S3_PRIVATELINK_FOR_US_EAST_1"],
        value_type: ValueType::Bool,
        additional_value_type: None,
        required: Required::Never,
        default: Some(DefaultValue::Bool(false)),
        sensitive: false,
        description: "Force the S3 regional endpoint for PUT/GET (PrivateLink-to-S3)",
        deprecated_by: None,
        scopes: &[ParamScope::Session],
        used_at_connect: false,
        mutable_after_connect: true,
    },
    ParamDef {
        canonical_name: param_names::VALIDATE_DEFAULT_PARAMETERS.as_str(),
        aliases: aliases![],
        value_type: ValueType::Bool,
        additional_value_type: None,
        required: Required::Never,
        default: Some(DefaultValue::Bool(false)),
        sensitive: false,
        description: "Validate that the default database, schema, and warehouse exist on the server at connect time",
        deprecated_by: None,
        scopes: &[ParamScope::Connection],
        used_at_connect: true,
        mutable_after_connect: false,
    },
    // ── Proxy ──────────────────────────────────────────────────────────
    ParamDef {
        canonical_name: param_names::PROXY_HOST.as_str(),
        // The legacy ODBC `PROXY` DSN key uses a different *format* (full URL
        // with embedded creds), so it is registered as a distinct canonical
        // param `proxy` rather than aliased here. `build_proxy_config` parses
        // the URL and merges it with the fields below.
        aliases: aliases![],
        value_type: ValueType::String,
        additional_value_type: None,
        required: Required::Never,
        default: None,
        sensitive: false,
        description: "Proxy server hostname",
        deprecated_by: None,
        scopes: &[ParamScope::Connection],
        used_at_connect: true,
        mutable_after_connect: false,
    },
    ParamDef {
        canonical_name: param_names::PROXY_PORT.as_str(),
        aliases: aliases![],
        value_type: ValueType::Int,
        additional_value_type: None,
        required: Required::Never,
        default: None,
        sensitive: false,
        description: "Proxy server port",
        deprecated_by: None,
        scopes: &[ParamScope::Connection],
        used_at_connect: true,
        mutable_after_connect: false,
    },
    ParamDef {
        canonical_name: param_names::PROXY_USER.as_str(),
        aliases: aliases![],
        value_type: ValueType::String,
        additional_value_type: None,
        required: Required::Never,
        default: None,
        sensitive: false,
        description: "Proxy server username for Basic auth",
        deprecated_by: None,
        scopes: &[ParamScope::Connection],
        used_at_connect: true,
        mutable_after_connect: false,
    },
    ParamDef {
        canonical_name: param_names::PROXY_PASSWORD.as_str(),
        aliases: aliases![],
        value_type: ValueType::String,
        additional_value_type: None,
        required: Required::Never,
        default: None,
        sensitive: true,
        description: "Proxy server password for Basic auth",
        deprecated_by: None,
        scopes: &[ParamScope::Connection],
        used_at_connect: true,
        mutable_after_connect: false,
    },
    ParamDef {
        canonical_name: param_names::NO_PROXY.as_str(),
        // No alias: legacy ODBC's DSN key is `NO_PROXY` (`Snowflake.h`
        // `SF_NO_PROXY_KEY`) and legacy Python's kwarg is `no_proxy`, both of
        // which match the canonical name case-insensitively. The separator-less
        // `NOPROXY` was UD-only leniency and is no longer accepted.
        aliases: aliases![],
        value_type: ValueType::String,
        additional_value_type: None,
        required: Required::Never,
        default: None,
        sensitive: false,
        description: "Comma-separated list of hosts to bypass the proxy for",
        deprecated_by: None,
        scopes: &[ParamScope::Connection],
        used_at_connect: true,
        mutable_after_connect: false,
    },
    // ── Workload Identity Federation (WIF) ────────────────────────────
    ParamDef {
        canonical_name: param_names::WORKLOAD_IDENTITY_PROVIDER.as_str(),
        aliases: aliases![],
        value_type: ValueType::String,
        additional_value_type: None,
        required: Required::WhenAuthMethod("WORKLOAD_IDENTITY"),
        default: None,
        sensitive: false,
        description: "Cloud provider for WIF attestation (AWS, AZURE, GCP, OIDC)",
        deprecated_by: None,
        scopes: &[ParamScope::Connection],
        used_at_connect: true,
        mutable_after_connect: false,
    },
    ParamDef {
        canonical_name: param_names::WORKLOAD_IDENTITY_ENTRA_RESOURCE.as_str(),
        aliases: aliases![],
        value_type: ValueType::String,
        additional_value_type: None,
        required: Required::Never,
        default: None,
        sensitive: false,
        description: "Azure Entra resource URI for managed-identity token (Azure only; defaults to api://fd3f753b-eed3-462c-b6a7-a4b5bb650aad)",
        deprecated_by: None,
        scopes: &[ParamScope::Connection],
        used_at_connect: true,
        mutable_after_connect: false,
    },
    ParamDef {
        canonical_name: param_names::WORKLOAD_IDENTITY_IMPERSONATION_PATH.as_str(),
        aliases: aliases![],
        value_type: ValueType::String,
        additional_value_type: None,
        required: Required::Never,
        default: None,
        sensitive: false,
        description: "Comma-separated impersonation chain for WIF (AWS role ARNs or GCP service account emails)",
        deprecated_by: None,
        scopes: &[ParamScope::Connection],
        used_at_connect: true,
        mutable_after_connect: false,
    },
    ParamDef {
        canonical_name: param_names::WORKLOAD_IDENTITY_AWS_USE_OUTBOUND_TOKEN.as_str(),
        aliases: aliases![],
        value_type: ValueType::Bool,
        additional_value_type: None,
        required: Required::Never,
        default: Some(DefaultValue::Bool(false)),
        sensitive: false,
        description: "Use outbound STS GetWebIdentityToken for AWS WIF (default: pre-signed GetCallerIdentity)",
        deprecated_by: None,
        scopes: &[ParamScope::Connection],
        used_at_connect: true,
        mutable_after_connect: false,
    },
    // Legacy ODBC PROXY URL form (parsed and merged with the fields above).
    ParamDef {
        canonical_name: param_names::PROXY.as_str(),
        aliases: aliases![],
        value_type: ValueType::String,
        additional_value_type: None,
        required: Required::Never,
        default: None,
        sensitive: true,
        description: "Proxy URL ([scheme://][user:pass@]host[:port]); legacy ODBC `PROXY` form",
        deprecated_by: None,
        scopes: &[ParamScope::Connection],
        used_at_connect: true,
        mutable_after_connect: false,
    },
    ParamDef {
        canonical_name: param_names::USE_PROXY_ENV.as_str(),
        // Legacy ODBC DSN `ProxyWithEnv` (`Snowflake.h`), uppercased by the
        // connection-string parser. ODBC-only: the legacy Python connector has
        // no equivalent kwarg (it consulted the proxy env vars unconditionally).
        aliases: aliases![Odbc; "PROXYWITHENV"],
        value_type: ValueType::Bool,
        additional_value_type: None,
        required: Required::Never,
        default: Some(DefaultValue::Bool(false)),
        sensitive: false,
        description: "Honour HTTP_PROXY/HTTPS_PROXY/NO_PROXY env vars when no explicit proxy is set",
        deprecated_by: None,
        scopes: &[ParamScope::Connection],
        used_at_connect: true,
        mutable_after_connect: false,
    },
    ParamDef {
        canonical_name: param_names::ALLOW_EMPTY_PROXY.as_str(),
        // Legacy ODBC DSN `AllowEmptyProxy` (`Snowflake.h`), uppercased by the
        // connection-string parser. ODBC-only: no legacy Python equivalent.
        aliases: aliases![Odbc; "ALLOWEMPTYPROXY"],
        value_type: ValueType::Bool,
        additional_value_type: None,
        required: Required::Never,
        default: Some(DefaultValue::Bool(true)),
        sensitive: false,
        description: "Empty PROXY value explicitly disables proxy (overrides env)",
        deprecated_by: None,
        scopes: &[ParamScope::Connection],
        used_at_connect: true,
        mutable_after_connect: false,
    },
    ParamDef {
        canonical_name: param_names::ENABLE_CONNECTION_DIAG.as_str(),
        aliases: aliases![],
        value_type: ValueType::Bool,
        additional_value_type: None,
        required: Required::Never,
        // No registry default: the consumer uses `.unwrap_or(false)`.  Omitting
        // the default keeps the Python dataclass field at `None` so that a
        // TOML profile setting `enable_connection_diag = true` is not silently
        // overridden by a Python-side `False` default passed as an explicit
        // Layer-1 option.
        default: None,
        sensitive: false,
        description: "Run connectivity diagnostics during connect and write a report",
        deprecated_by: None,
        scopes: &[ParamScope::Connection],
        used_at_connect: true,
        mutable_after_connect: false,
    },
    ParamDef {
        canonical_name: param_names::CONNECTION_DIAG_LOG_PATH.as_str(),
        aliases: aliases![],
        value_type: ValueType::String,
        additional_value_type: None,
        required: Required::Never,
        default: None,
        sensitive: false,
        description: "Directory where the diagnostic report file is written (defaults to system tmpdir)",
        deprecated_by: None,
        scopes: &[ParamScope::Connection],
        used_at_connect: true,
        mutable_after_connect: false,
    },
    ParamDef {
        canonical_name: param_names::CONNECTION_DIAG_ALLOWLIST_PATH.as_str(),
        aliases: aliases![],
        value_type: ValueType::String,
        additional_value_type: None,
        required: Required::Never,
        default: None,
        sensitive: false,
        description: "Path to a pre-fetched allowlist.json; if absent the driver fetches it via system$allowlist()",
        deprecated_by: None,
        scopes: &[ParamScope::Connection],
        used_at_connect: true,
        mutable_after_connect: false,
    },
];

impl ParamDef {
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
}

/// The registry singleton. Built once at startup, immutable thereafter.
pub struct ParamRegistry {
    params: &'static [ParamDef],
    /// Case-insensitive map: lowercased canonical + global alias → index into `params`.
    alias_index: HashMap<String, usize>,
    /// Case-insensitive map: (wrapper, lowercased scoped alias) → index into `params`.
    wrapper_alias_index: HashMap<(Wrapper, String), usize>,
}

impl ParamRegistry {
    fn new(params: &'static [ParamDef]) -> Self {
        let mut alias_index = HashMap::new();
        let mut wrapper_alias_index = HashMap::new();
        for (i, param) in params.iter().enumerate() {
            alias_index.insert(param.canonical_name.to_ascii_lowercase(), i);
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
        }
    }

    /// Resolve a global alias or canonical name to its `ParamDef`.
    ///
    /// Accepts any type that can be viewed as a string — `ParamKey`, `&str`,
    /// or `String` — so callers with a typed key can pass it directly without
    /// calling `.as_str()`.  Lookup is case-insensitive. Scoped aliases are
    /// not visible here; use [`Self::resolve_for`].
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

    /// Return all registered parameter definitions.
    pub fn all_params(&self) -> &[ParamDef] {
        self.params
    }

    /// Check if a key is known as a canonical name or global alias.
    pub fn is_known(&self, key: &str) -> bool {
        self.alias_index.contains_key(&key.to_ascii_lowercase())
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
        // SCREAMING_SNAKE spellings that match a canonical name
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
            // UD-ODBC's own CRL DSN keys (legacy spelled the family `CRL_CHECK`).
            ("CRL_MODE", "crl_check_mode", &[Odbc]),
            ("CRL_ENABLED", "crl_check_mode", &[Odbc]),
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
    fn resolve_canonical_names() {
        let r = registry();
        for param in r.all_params() {
            assert!(
                r.resolve(param.canonical_name).is_some(),
                "canonical name {:?} should resolve",
                param.canonical_name
            );
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
        // `is_known` reflects the wrapper-agnostic index (canonicals + any
        // remaining global aliases). Canonical names resolve case-insensitively;
        // wrapper-scoped wire spellings (e.g. `SERVER`) are deliberately not
        // "known" without wrapper context.
        assert!(r.is_known("account"));
        assert!(r.is_known("ACCOUNT"));
        assert!(r.is_known("host"));
        assert!(r.is_known("HOST"));
        assert!(!r.is_known("SERVER"));
        assert!(!r.is_known("unknown_key"));
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
    fn proxy_params_have_correct_metadata() {
        let r = registry();
        for key in [
            "proxy_host",
            "proxy_port",
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
}
