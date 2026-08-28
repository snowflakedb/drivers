#![allow(clippy::result_large_err)]
pub mod async_exec;
mod auth;
mod browser;
mod error_context;
pub(crate) use error_context::SnowflakeErrorContext;
mod external_browser;
pub mod heartbeat;
pub mod logout;
mod native_okta;
mod oauth;
pub mod prompt_lock;
pub mod workload_identity;
/// Re-export of the browser-launcher closure type so that
/// `crate::config::rest_parameters::OAuthAuthorizationCodeConfig` can
/// carry a `Arc<dyn Fn() -> BrowserLaunchFn + Send + Sync>` factory
/// without reaching into the private `oauth` module hierarchy.
pub(crate) use oauth::BrowserLaunchFn;
/// Re-exported under `cfg(any(test, feature = "test-utils"))` so e2e
/// tests can derive the OAuth token-cache key host without
/// reimplementing the Python-style `urlparse(token_request_url).hostname`
/// fallback chain. Production builds do not expose this helper.
#[cfg(any(test, feature = "test-utils"))]
pub use oauth::host_from_token_url;
pub mod query_request;
pub mod query_response;
pub mod sql_state;
pub mod telemetry;

use std::collections::HashMap;

use crate::auth::{AuthError, Credentials, create_credentials};
use crate::config::rest_parameters::ClientInfo;
use crate::config::rest_parameters::{LoginMethod, LoginParameters, QueryParameters};
use crate::config::retry::RetryPolicy;
use crate::crl::worker::SharedCrlWorker;
use crate::http::retry::{HttpContext, HttpError, TransportSnafu, execute_with_retry};
use crate::logging::url_for_log;
use crate::rest::snowflake::auth::{
    AuthRequest, AuthRequestClientCapabilities, AuthRequestClientEnvironment, AuthRequestData,
    AuthResponse, authenticator,
};
use crate::rest::snowflake::external_browser::{
    DefaultBrowserOpener, external_browser_authenticate,
};
use crate::rest::snowflake::native_okta::fetch_native_okta_saml;
use crate::sensitive::SensitiveString;
use crate::tls::client::create_tls_client_with_proxy;
use crate::tls::error::TlsError;
use crate::token_cache::{CacheKey, TokenCache, TokenType, normalize_identifier, normalize_url};
use reqwest::{self, Method, StatusCode, header};
use serde::de::Deserialize as _;
use serde_json;
use serde_json::value::RawValue;
use snafu::{Location, OptionExt, ResultExt, Snafu};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use tracing;
use url::Url;
use uuid::Uuid;

pub const STATEMENT_ASYNC_EXECUTION_OPTION: &str = "async_execution";
pub(crate) const QUERY_REQUEST_PATH: &str = "/queries/v1/query-request";
const ABORT_REQUEST_PATH: &str = "/queries/v1/abort-request";
const TOKEN_REQUEST_PATH: &str = "/session/token-request";

/// Send an HTTP request with retry and return `(StatusCode, body_text)`.
///
/// Shared by `native_okta` and `external_browser` authentication flows.
async fn request_text_with_retry(
    build: impl Fn() -> reqwest::RequestBuilder,
    ctx: &HttpContext,
    policy: &RetryPolicy,
) -> Result<(StatusCode, String), HttpError> {
    execute_with_retry(build, ctx, policy, |resp| async move {
        let status = resp.status();
        let text = resp.text().await.context(TransportSnafu)?;
        Ok((status, text))
    })
    .await
}

// ─── Snowflake GS protocol error codes ───────────────────────────────────────
/// GS error code returned when a running query has been canceled (server-side
/// abort, statement timeout, or `SQLCancel`). The query-request / result
/// response carries this code so the ODBC wrapper can classify the failure as a
/// cancellation (ODBC `HY008`) rather than a generic error — see
/// `odbc/src/api/error.rs`. Matches the reference driver's
/// `Statement::S_QUERY_CANCELED`.
pub const QUERY_CANCELED: i32 = 604;
/// GS error code returned when a session no longer exists on the server.
/// Logout callers treat this as success — the goal (an invalidated session) is achieved.
pub const SESSION_GONE: i32 = 390111;
/// GS error code returned when the session token has expired.
/// The caller must use the master token to obtain a fresh session token and retry.
pub const SESSION_TOKEN_EXPIRED: i32 = 390112;
/// GS error code returned when the master token could not be found on the
/// server. Same terminal handling as [`MASTER_TOKEN_EXPIRED`]. Matches
/// legacy's `MASTER_TOKEN_NOTFOUND_GS_CODE` (`network.py`); JDBC has the
/// equivalent case under the same numeric value.
pub const MASTER_TOKEN_NOT_FOUND: i32 = 390113;
/// GS error code returned when the master token has expired.
/// Full re-authentication is required; the session can never be renewed.
pub const MASTER_TOKEN_EXPIRED: i32 = 390114;
/// GS error code returned when the master token is invalid. Same terminal
/// handling as [`MASTER_TOKEN_EXPIRED`]. Matches legacy's
/// `MASTER_TOKEN_INVALD_GS_CODE` (sic — legacy's own constant misspells
/// "invalid") (`network.py`).
pub const MASTER_TOKEN_INVALID: i32 = 390115;
/// GS codes that mean the master token can never be renewed — not found,
/// expired, or invalid.
const MASTER_TOKEN_TERMINAL_CODES: [i32; 3] = [
    MASTER_TOKEN_NOT_FOUND,
    MASTER_TOKEN_EXPIRED,
    MASTER_TOKEN_INVALID,
];
/// GS error code returned when the OAuth access token presented at login is
/// invalid. Treated cross-driver as a signal to evict the cached access
/// token and replay the OAuth flow.
pub const OAUTH_ACCESS_TOKEN_INVALID: i32 = 390303;
/// GS error code returned when the OAuth access token presented at login has
/// expired. Same eviction-and-retry behavior as
/// [`OAUTH_ACCESS_TOKEN_INVALID`].
pub const OAUTH_ACCESS_TOKEN_EXPIRED: i32 = 390318;
/// GS error codes that indicate the cached OAuth access token (and any
/// DPoP-bundled cache entry) must be evicted, after which the login is
/// retried once. Mirrors JDBC/Go's `refreshOAuthTokenErrorCodes` set.
const OAUTH_REFRESH_ERROR_CODES: [i32; 2] =
    [OAUTH_ACCESS_TOKEN_INVALID, OAUTH_ACCESS_TOKEN_EXPIRED];
/// GS error codes that reject the presented credentials outright (bad
/// username/password, invalid JWT). These warrant SQLSTATE `28000` (invalid
/// authorization) rather than the generic connection-failure SQLSTATE.
/// Mirrors `CREDENTIAL_REJECTION_GS_CODES` in the legacy Python connector's
/// `network.py` (SNOW-3775156), which is where the raw-code passthrough this
/// list supports originated.
pub const CREDENTIAL_REJECTION_GS_CODES: [i32; 9] = [
    390100, // AUTHORIZATION_FAILURE
    390144, // JWT_TOKEN_INVALID
    394300, // JWT_TOKEN_INVALID
    394301, // JWT_TOKEN_EXPIRED
    394302, // JWT_TOKEN_NOT_YET_VALID
    394303, // JWT_TOKEN_INVALID_EXPIRATION_TIME
    394304, // JWT_TOKEN_INVALID_PUBLIC_KEY_FINGERPRINT_MISMATCH
    394305, // JWT_TOKEN_INVALID_ALGORITHM
    394306, // JWT_TOKEN_INVALID_FORMAT
];
/// ANSI SQLSTATE for "invalid authorization specification", used for
/// [`CREDENTIAL_REJECTION_GS_CODES`]. Mirrors `SQLSTATE_AUTHORIZATION_FAILURE`
/// in the legacy Python connector's `sqlstate.py`.
pub const SQLSTATE_AUTHORIZATION_FAILURE: &str = "28000";
/// ANSI SQLSTATE for "connection exception: connection does not exist" —
/// used for session-token expiry (`390112`) and for terminal, non-renewable
/// authentication states (master-token expiry, reauth-shaped login failures)
/// that are not a credential rejection. The credentials themselves were not
/// rejected, so `SQLSTATE_AUTHORIZATION_FAILURE` (class 28) would
/// misclassify it. Mirrors `SQLSTATE_CONNECTION_WAS_NOT_ESTABLISHED`
/// in the legacy Python connector's `sqlstate.py`.
pub const SQLSTATE_CONNECTION_WAS_NOT_ESTABLISHED: &str = "08001";
/// ODBC/ANSI SQLSTATE for a driver-enforced query or cancel timeout (`HYT00`).
pub const SQLSTATE_TIMEOUT_EXPIRED: &str = "HYT00";
/// Sentinel for a login-failure `code` the server omitted or sent as a
/// non-numeric value — not a real GS error code. Produced at the `code`
/// extraction site feeding [`LoginSnafu`] below.
pub const GS_CODE_UNAVAILABLE: i32 = -1;

/// Session tokens returned from login, used for authentication and refresh
#[derive(Debug, Clone)]
pub struct SessionTokens {
    /// Token used to authenticate API requests
    pub session_token: SensitiveString,
    /// Token used to refresh an expired session token
    pub master_token: SensitiveString,
    /// Server-assigned session ID
    pub session_id: i64,
    /// When the session token expires
    pub session_expires_at: Option<std::time::Instant>,
    /// When the master token expires (after this, full re-auth is needed)
    pub master_expires_at: Option<std::time::Instant>,
    /// Configured master-token TTL as returned by the server (`masterValidityInSeconds`).
    /// Unlike the remaining time derived from `master_expires_at`, this does not shrink
    /// as the token ages, so it is the right input for heartbeat-cadence computation.
    pub master_validity: Option<std::time::Duration>,
}

/// Result of a successful login to Snowflake
#[derive(Debug)]
pub struct LoginResult {
    /// Session tokens for authentication and refresh
    pub tokens: SessionTokens,
    /// Session parameters returned by the server
    pub session_parameters: Option<HashMap<String, String>>,
    /// Server-echoed database name from sessionInfo
    pub database_name: Option<String>,
    /// Server-echoed schema name from sessionInfo
    pub schema_name: Option<String>,
    /// Server-echoed warehouse name from sessionInfo
    pub warehouse_name: Option<String>,
    /// Server-echoed role name from sessionInfo
    pub role_name: Option<String>,
    /// Snowflake server version reported
    pub server_version: Option<String>,
}

impl SessionTokens {
    /// Check if the master token is expired or about to expire
    pub fn is_master_expired(&self) -> bool {
        self.master_expires_at
            .map(|exp| exp < std::time::Instant::now())
            .unwrap_or(false)
    }

    /// Check if the session token is expired or about to expire
    pub fn is_session_expired(&self) -> bool {
        self.session_expires_at
            .map(|exp| exp < std::time::Instant::now())
            .unwrap_or(false)
    }

    /// Get remaining validity for the master token
    pub fn master_valid_for(&self) -> Option<std::time::Duration> {
        self.master_expires_at
            .and_then(|exp| exp.checked_duration_since(std::time::Instant::now()))
    }
}

/// Response from the session token refresh endpoint
#[derive(Debug, serde::Deserialize)]
struct RefreshSessionResponse {
    data: Option<RefreshSessionData>,
    message: Option<String>,
    code: Option<String>,
    success: bool,
}

#[derive(Debug, serde::Deserialize)]
struct RefreshSessionData {
    #[serde(rename = "sessionToken")]
    session_token: SensitiveString,
    #[serde(rename = "masterToken")]
    master_token: SensitiveString,
    #[serde(rename = "sessionId")]
    session_id: i64,
    #[serde(
        rename = "validityInSecondsST",
        deserialize_with = "auth::deserialize_seconds_as_duration",
        default
    )]
    validity: Option<std::time::Duration>,
    #[serde(
        rename = "validityInSecondsMT",
        deserialize_with = "auth::deserialize_seconds_as_duration",
        default
    )]
    master_validity: Option<std::time::Duration>,
}

/// Response from the token request endpoint (ISSUE/RENEW).
/// Unlike `RefreshSessionResponse`, fields like `masterToken` and `sessionId`
/// may be absent depending on the request type.
#[derive(Debug, serde::Deserialize)]
struct TokenRequestResponse {
    data: Option<TokenRequestData>,
    message: Option<String>,
    code: Option<String>,
    success: bool,
}

#[derive(Debug, serde::Deserialize)]
struct TokenRequestData {
    #[serde(rename = "sessionToken")]
    session_token: SensitiveString,
    #[serde(
        rename = "validityInSecondsST",
        deserialize_with = "auth::deserialize_seconds_as_duration",
        default
    )]
    validity: Option<std::time::Duration>,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum QueryExecutionMode {
    #[default]
    Blocking,
    Async,
}

/// Rarely-varied knobs for a single query execution.
///
/// Every field defaults to the common case — default retry policy, blocking
/// (sync) execution, and a freshly-minted `requestId` — so the overwhelming
/// majority of callers pass `QueryOptions::default()`. Only the
/// statement-execute path overrides these (to run async and/or pre-register a
/// `requestId` for cross-thread cancel), which it does with struct-update
/// syntax: `QueryOptions { request_id: Some(id), ..Default::default() }`.
#[derive(Clone, Debug, Default)]
pub struct QueryOptions {
    /// HTTP-level retry policy. Defaults to [`RetryPolicy::default`].
    pub retry_policy: RetryPolicy,
    /// Sync (blocking) vs. async execution. Defaults to
    /// [`QueryExecutionMode::Blocking`].
    pub execution_mode: QueryExecutionMode,
    /// Caller-supplied `requestId`. `None` mints a fresh id inside the query
    /// function — the right choice for callers that don't need to know it in
    /// advance. The statement-execute path passes `Some(id)` because it needs the
    /// same id afterwards: the abort-request it fires on cancellation or on a
    /// client-side timeout is keyed on the `requestId` the query was sent with.
    pub request_id: Option<uuid::Uuid>,
}

#[derive(Clone)]
pub struct QueryInput<'a> {
    pub sql: String,
    pub bindings: Option<&'a RawValue>,
    pub bind_stage: Option<String>,
    pub describe_only: Option<bool>,
    pub query_parameters: Option<HashMap<String, serde_json::Value>>,
}

impl<'a> QueryInput<'a> {
    pub fn new(sql: impl Into<String>) -> Self {
        QueryInput {
            sql: sql.into(),
            bindings: None,
            bind_stage: None,
            describe_only: None,
            query_parameters: None,
        }
    }
}

/// Build the optional `sql` and `bindings` fields used in query log lines,
/// honoring the `log_query_text` / `log_query_parameters` opt-ins and the
/// existing `log_max_query_length` truncation.
///
/// - `(None, None)` when `log_query_text` is `false`.
/// - `(Some(prefix), None)` when only `log_query_text` is `true`.
/// - `(Some(prefix), Some(bindings_prefix))` when both flags are `true`;
///   `bindings_prefix` is the empty string when no bindings are attached.
///
/// Returning `None` lets callers pass the result straight to `tracing` macros
/// where `Option::None` fields are skipped automatically.
pub(crate) fn query_log_fields(
    params: &QueryParameters,
    input: &QueryInput<'_>,
) -> (Option<String>, Option<String>) {
    if !params.log_query_text {
        return (None, None);
    }
    let sql = input
        .sql
        .chars()
        .take(params.log_max_query_length)
        .collect::<String>();
    let bindings = params.log_query_parameters.then(|| {
        input
            .bindings
            .map(|raw| {
                raw.get()
                    .chars()
                    .take(params.log_max_query_length)
                    .collect::<String>()
            })
            .unwrap_or_default()
    });
    (Some(sql), bindings)
}

pub fn user_agent(client_info: &ClientInfo) -> String {
    let base = format!(
        "{}/{} ({}-{})",
        client_info.client_app_id,
        client_info.version,
        client_info.os,
        std::env::consts::ARCH
    );
    match (&client_info.runtime_name, &client_info.runtime_version) {
        (Some(name), Some(ver)) => {
            // Sanitize runtime name: replace spaces with underscores so the
            // User-Agent token is safe for parsers that split on whitespace
            // (e.g. Java's `java.vm.name` = "OpenJDK 64-Bit Server VM").
            let safe_name = name.replace(' ', "_");
            format!("{base} {safe_name}/{ver}")
        }
        _ => base,
    }
}

/// Strip non-numeric suffixes from a version string so the server accepts it.
///
/// `CLIENT_APP_VERSION` must be a dotted numeric version for feature gates to
/// remain enabled, so this helper truncates each dot-separated segment at its
/// first non-digit and drops everything from the first segment that has no
/// leading digit at all. That last part keeps PEP 440 dev/post releases, where
/// the suffix is its own segment, from turning into a bogus extra component
/// (`"5.0.0.dev0"` must not become `"5.0.0.0"`). Examples: `"5.0.0dev"` →
/// `"5.0.0"`, `"5.0.0.dev0"` → `"5.0.0"`, `"2.21.8.1"` → `"2.21.8.1"`.
fn strip_version_suffix(version: &str) -> String {
    let stripped = version
        .split('.')
        .map(|seg| -> String { seg.chars().take_while(|c| c.is_ascii_digit()).collect() })
        .take_while(|numeric| !numeric.is_empty())
        .collect::<Vec<_>>()
        .join(".");

    if stripped.is_empty() {
        "0".to_owned()
    } else {
        stripped
    }
}

fn base_auth_request_data(login_parameters: &LoginParameters) -> AuthRequestData {
    AuthRequestData {
        account_name: login_parameters.account_name.clone(),
        client_app_id: login_parameters.client_info.client_app_id.clone(),
        client_app_version: strip_version_suffix(&login_parameters.client_info.version),
        client_app_version_full: login_parameters.client_info.version.clone(),
        client_capabilities: AuthRequestClientCapabilities {
            smk_id_as_string: true,
        },
        client_environment: AuthRequestClientEnvironment {
            application: login_parameters.client_info.application.clone(),
            os: login_parameters.client_info.os.clone(),
            os_version: login_parameters.client_info.os_version.clone(),
            ocsp_mode: login_parameters.client_info.ocsp_mode.clone(),
            platforms: login_parameters.client_info.platforms.clone(),
            runtime_version: login_parameters.client_info.runtime_version.clone(),
            runtime_name: login_parameters.client_info.runtime_name.clone(),
            compiler: login_parameters.client_info.compiler.clone(),
            os_details: login_parameters.client_info.os_details.clone(),
            release_type: login_parameters.client_info.release_type.clone(),
        },
        ..Default::default()
    }
}

/// GS error code returned when a cached id_token presented at login is
/// invalid or stale. Matches legacy's `ID_TOKEN_INVALID_LOGIN_REQUEST_GS_CODE`
/// (`network.py`).
const ID_TOKEN_INVALID_LOGIN_REQUEST: i32 = 390195;

const EXT_AUTHN_ERROR_CODES: [i32; 8] = [
    390120, // EXT_AUTHN_DENIED
    390122, // EXT_AUTHN_NOT_ENROLLED
    390123, // EXT_AUTHN_LOCKED
    390126, // EXT_AUTHN_TIMEOUT
    390127, // EXT_AUTHN_INVALID
    390129, // EXT_AUTHN_EXCEPTION
    390132, // EXT_AUTHN_DUO_PUSH_DISABLED
    ID_TOKEN_INVALID_LOGIN_REQUEST,
];

/// GS codes that mean a cached credential was rejected. Necessary but not
/// sufficient: see [`driver_can_reacquire_credential`].
///
/// Deliberately narrower than all of [`EXT_AUTHN_ERROR_CODES`]: the other
/// seven are MFA-flow failures (denied, not enrolled, locked, timeout,
/// invalid, exception, DUO push disabled), not a dead cached credential — a
/// locked account is not fixed by opening a new connection, so surfacing
/// `ReauthenticationRequest` for them would be actively misleading. They
/// stay plain login failures. Matches legacy's set in `auth/_auth.py`.
fn code_is_reauth_shaped(code: i32) -> bool {
    code == ID_TOKEN_INVALID_LOGIN_REQUEST || OAUTH_REFRESH_ERROR_CODES.contains(&code)
}

/// True when the driver can re-drive credential acquisition itself; false
/// when the credential was supplied by the caller verbatim or there's no
/// re-drive mechanism. Not "is a human involved": browser auth is `true`
/// (driver drives the whole flow) but MFA is `false` (evicting the cached
/// token and replaying is cache invalidation, not reauthentication — the
/// user must satisfy the second factor again). Mirrors legacy's three real
/// `reauthenticate()` implementations (`auth/idtoken.py`, `auth/webbrowser.py`,
/// `auth/_oauth_base.py`) vs. its six `{"success": False}` stubs.
fn driver_can_reacquire_credential(m: &LoginMethod) -> bool {
    match m {
        LoginMethod::ExternalBrowser { .. }
        | LoginMethod::OAuthAuthorizationCode(_)
        | LoginMethod::OAuthClientCredentials(_) => true,

        LoginMethod::Password { .. }
        | LoginMethod::NativeOkta(_)
        | LoginMethod::PrivateKey { .. }
        | LoginMethod::Pat { .. }
        | LoginMethod::UserPasswordMfa { .. }
        | LoginMethod::OAuthAccessToken { .. }
        | LoginMethod::SessionToken { .. }
        | LoginMethod::WorkloadIdentity(_) => false,
        // no `_` arm — a new LoginMethod MUST be classified here
    }
}

fn is_reauthentication_required(code: i32, m: &LoginMethod) -> bool {
    code_is_reauth_shaped(code) && driver_can_reacquire_credential(m)
}

/// Sets the DUO second-factor fields on the login request.
/// Matches the behavior of the old JDBC, .NET, and ODBC drivers:
/// always sends `EXT_AUTHN_DUO_METHOD`, defaulting to `"push"` when
/// no passcode is provided.
fn set_duo_authn_fields(
    data: &mut AuthRequestData,
    passcode_in_password: bool,
    passcode: Option<SensitiveString>,
) {
    data.ext_authn_duo_method = Some(if passcode.is_some() || passcode_in_password {
        "passcode".to_string()
    } else {
        "push".to_string()
    });
    if !passcode_in_password {
        data.passcode = passcode;
    }
}

async fn try_get_cached_token(
    server_url: &str,
    username: &str,
    role: &str,
    token_type: TokenType,
    token_cache: Option<std::sync::Arc<dyn TokenCache>>,
) -> Option<SensitiveString> {
    let cache = token_cache?;
    let key = CacheKey {
        token_type,
        idp: String::new(),
        snowflake: normalize_url(server_url),
        username: normalize_identifier(username),
        role: normalize_identifier(role),
    };
    let result = tokio::task::spawn_blocking(move || cache.get_token(&key)).await;
    match result {
        Ok(Ok(Some(token))) if !token.is_empty() => {
            tracing::info!(%token_type, "Found cached token");
            Some(token.into())
        }
        Ok(Ok(_)) => None,
        Ok(Err(e)) => {
            tracing::warn!(%token_type, error = %e, "Failed to retrieve cached token");
            None
        }
        Err(e) => {
            tracing::warn!(%token_type, error = %e, "Cache retrieval task panicked");
            None
        }
    }
}

async fn store_token_in_cache(
    server_url: &str,
    username: &str,
    role: &str,
    token_type: TokenType,
    token_value: &str,
    token_cache: Option<std::sync::Arc<dyn TokenCache>>,
) {
    let Some(cache) = token_cache else {
        tracing::debug!(%token_type, "No token cache available");
        return;
    };
    let key = CacheKey {
        token_type,
        idp: String::new(),
        snowflake: normalize_url(server_url),
        username: normalize_identifier(username),
        role: normalize_identifier(role),
    };
    let token_value = token_value.to_string();
    let result = tokio::task::spawn_blocking(move || cache.add_token(&key, &token_value)).await;
    match result {
        Ok(Ok(())) => {
            tracing::info!(%token_type, "Cached token for future use");
        }
        Ok(Err(e)) => {
            tracing::warn!(%token_type, error = %e, "Failed to cache token");
        }
        Err(e) => {
            tracing::warn!(%token_type, error = %e, "Cache store task panicked");
        }
    }
}

async fn remove_token_from_cache(
    server_url: &str,
    username: &str,
    role: &str,
    token_type: TokenType,
    token_cache: Option<std::sync::Arc<dyn TokenCache>>,
) {
    let Some(cache) = token_cache else {
        return;
    };
    let key = CacheKey {
        token_type,
        idp: String::new(),
        snowflake: normalize_url(server_url),
        username: normalize_identifier(username),
        role: normalize_identifier(role),
    };
    let result = tokio::task::spawn_blocking(move || cache.remove_token(&key)).await;
    match result {
        Ok(Ok(())) => {
            tracing::info!(%token_type, "Removed cached token");
        }
        Ok(Err(e)) => {
            tracing::warn!(%token_type, error = %e, "Failed to remove cached token");
        }
        Err(e) => {
            tracing::warn!(%token_type, error = %e, "Cache removal task panicked");
        }
    }
}

/// Evict the cached OAuth access token (and DPoP-bundled entry, when
/// present) for an Authorization Code login. Used by the
/// `390303 / 390318` retry block in [`snowflake_login_with_client`]:
/// after eviction the next call to `auth_request_data` will run the
/// refresh-token leg or, if that also fails, the full interactive flow.
///
/// The `idp_url` is derived through [`oauth::derive_idp_url`] — the same helper
/// the storing path uses — so `normalize_url` sees identical input on both
/// sides and produces byte-exact cache keys even for URLs with explicit default
/// ports (e.g. `:443`), and neither path can drift from the other
/// (SNOW-3780375). The `snowflake_url` is always the Snowflake server URL.
async fn evict_oauth_access_token_for_authorization_code(
    cfg: &crate::config::rest_parameters::OAuthAuthorizationCodeConfig,
    server_url: &str,
    role: &str,
    token_cache: Option<std::sync::Arc<dyn TokenCache>>,
) {
    let parsed_server_url = match Url::parse(server_url) {
        Ok(url) => url,
        Err(_) => {
            tracing::warn!("Cannot evict cached OAuth access token: server_url is not a valid URL");
            return;
        }
    };
    let idp_url = match oauth::derive_idp_url(cfg, &parsed_server_url) {
        Ok(idp_url) => idp_url,
        Err(_) => {
            tracing::warn!(
                "Cannot evict cached OAuth access token: unable to derive IdP token URL from server_url"
            );
            return;
        }
    };
    tracing::debug!(
        idp_host_path = %url::Url::parse(&idp_url)
            .map(|u| format!("{}{}", u.host_str().unwrap_or(""), u.path()))
            .unwrap_or_default(),
        "Evicting cached OAuth access token"
    );
    oauth::remove_oauth_access_token(
        &idp_url,
        server_url,
        &cfg.username,
        role,
        token_cache.clone(),
    )
    .await;
    oauth::remove_oauth_dpop_bundled(&idp_url, server_url, &cfg.username, role, token_cache).await;
}

pub async fn auth_request_data(
    client: &reqwest::Client,
    login_parameters: &LoginParameters,
    session_parameters: Option<&HashMap<String, String>>,
    token_cache: Option<std::sync::Arc<dyn TokenCache>>,
    prompt_locks: Option<&std::sync::Arc<prompt_lock::PromptLockMap>>,
    retry_policy: &RetryPolicy,
) -> Result<AuthRequestData, RestError> {
    let mut data = base_auth_request_data(login_parameters);
    data.spcs_token = login_parameters.spcs_token.clone();

    if let Some(secondary_roles) = login_parameters.secondary_roles.as_deref()
        && !secondary_roles.is_empty()
    {
        data.secondary_roles = Some(secondary_roles.to_uppercase());
    }

    if let Some(params) = session_parameters {
        let json_params = params
            .iter()
            .map(|(k, v)| (k.clone(), serde_json::Value::String(v.clone())))
            .collect();
        data.session_parameters = Some(json_params);
    }

    match &login_parameters.login_method {
        LoginMethod::NativeOkta(okta_config) => {
            let saml_html =
                fetch_native_okta_saml(client, login_parameters, retry_policy, okta_config)
                    .await
                    .context(NativeOktaSnafu)?;

            data.login_name = Some(okta_config.username.clone());
            data.authenticator = Some(okta_config.okta_url.to_string());
            data.raw_saml_response = Some(saml_html.into());
        }
        LoginMethod::ExternalBrowser {
            username,
            authentication_timeout_secs,
            client_store_temporary_credential,
        } => {
            data.login_name = Some(username.clone());
            data.authenticator = Some(authenticator::EXTERNAL_BROWSER.to_string());

            if *client_store_temporary_credential {
                data.session_parameters
                    .get_or_insert_with(HashMap::new)
                    .insert(
                        "CLIENT_STORE_TEMPORARY_CREDENTIAL".to_string(),
                        serde_json::Value::Bool(true),
                    );
            }

            let cached_id_token = if *client_store_temporary_credential {
                // `build_cache_key` drops `role` for ID tokens; empty is intentional.
                try_get_cached_token(
                    &login_parameters.server_url,
                    username,
                    "",
                    TokenType::IdToken,
                    token_cache.clone(),
                )
                .await
            } else {
                None
            };

            if let Some(cached_token) = cached_id_token {
                tracing::info!("Using cached SSO ID token for external browser login");
                data.authenticator = Some(authenticator::ID_TOKEN.to_string());
                data.token = Some(cached_token);
                data.token_from_cache_used = true;
            } else {
                let result = external_browser_authenticate(
                    client,
                    login_parameters,
                    username,
                    *authentication_timeout_secs,
                    &DefaultBrowserOpener,
                    retry_policy,
                )
                .await
                .context(ExternalBrowserSnafu)?;

                data.token = Some(result.token);
                data.proof_key = Some(result.proof_key);
                data.consent_cache_id_token = result.consent_cache_id_token;
            }
        }
        // Authorization Code orchestration runs the PKCE/state/loopback flow
        // (and any cache hits / refresh-token exchange) before forwarding the
        // resulting access token to Snowflake under AUTHENTICATOR=OAUTH.
        // The body always uses uppercase OAUTH — never the user-supplied
        // authenticator string verbatim — and tags the request with
        // OAUTH_TYPE=OAUTH_AUTHORIZATION_CODE so GS knows which flow
        // produced the token. LOGIN_NAME is always set.
        LoginMethod::OAuthAuthorizationCode(cfg) => {
            let acquired = oauth::run_oauth_authorization_code(
                client,
                &login_parameters.server_url,
                cfg,
                login_parameters.role.as_deref().unwrap_or(""),
                token_cache.clone(),
                login_parameters.disable_parallel_user_prompt,
                prompt_locks,
            )
            .await
            .context(OAuthFlowSnafu)?;
            data.login_name = Some(cfg.username.clone());
            data.token = Some(acquired.access_token);
            data.authenticator = Some(authenticator::OAUTH.to_string());
            data.oauth_type = Some("OAUTH_AUTHORIZATION_CODE".to_string());
            // `dpop_jwk_json` is `Option<String>`: `Some` when DPoP was
            // enabled, `None` otherwise, so the assignment is implicitly
            // conditional. The JWK is carried through login data so the
            // driver can build a DPoP proof header on the Snowflake login
            // request; the server validates it statelessly against the
            // thumbprint (`jkt`) already embedded in the access token
            // (RFC 9449).
            data.dpop_jwk_json = acquired.dpop_jwk_json;
        }
        // Client Credentials is external-IdP only and tokens are
        // intentionally not cached. On Snowflake error codes
        // 390303/390318 the retry block in `snowflake_login_with_client`
        // skips the AC eviction step and just replays the flow so the IdP
        // token endpoint is re-hit.
        LoginMethod::OAuthClientCredentials(cfg) => {
            let acquired = oauth::acquire_client_credentials(client, cfg)
                .await
                .context(OAuthFlowSnafu)?;
            data.login_name = Some(cfg.username.clone());
            data.token = Some(acquired.access_token);
            data.authenticator = Some(authenticator::OAUTH.to_string());
            data.oauth_type = Some("OAUTH_CLIENT_CREDENTIALS".to_string());
            // See AC branch above for why dpop_jwk_json is carried here.
            data.dpop_jwk_json = acquired.dpop_jwk_json;
        }
        LoginMethod::WorkloadIdentity(cfg) => {
            // Verify the host is a recognized Snowflake endpoint before
            // fetching cloud credentials. See workload_identity::host_allowlist.
            workload_identity::ensure_allowed_host(&login_parameters.server_url)
                .context(WorkloadIdentityAttestationSnafu)?;
            let attestation = workload_identity::create_attestation(client, cfg)
                .await
                .context(WorkloadIdentityAttestationSnafu)?;
            data.authenticator = Some(authenticator::WORKLOAD_IDENTITY.to_string());
            data.provider = Some(attestation.provider.to_string());
            data.token = Some(attestation.token);
        }
        _ => match create_credentials(login_parameters)
            .await
            .context(AuthenticationSnafu)?
        {
            Credentials::Password {
                username,
                password,
                passcode_in_password,
                passcode,
            } => {
                data.login_name = Some(username);
                data.password = Some(password);
                set_duo_authn_fields(&mut data, passcode_in_password, passcode);
            }
            Credentials::Jwt { username, token } => {
                data.login_name = Some(username);
                data.token = Some(token);
                data.authenticator = Some(authenticator::SNOWFLAKE_JWT.to_string());
            }
            Credentials::Pat { username, token } => {
                // PAT encodes the principal; omit LOGIN_NAME when empty so
                // Snowflake resolves the user from the token itself.
                if !username.is_empty() {
                    data.login_name = Some(username);
                }
                data.token = Some(token);
                data.authenticator = Some(authenticator::PROGRAMMATIC_ACCESS_TOKEN.to_string());
            }
            // Legacy pre-acquired access token: forward unchanged (analysis
            // §6 / §10.1). LOGIN_NAME is always set (§14 #10) — never the
            // .NET-only `loginName=""` quirk — and OAUTH_TYPE is omitted to
            // distinguish the legacy flow from AC/CC.
            Credentials::OAuth {
                username,
                access_token,
            } => {
                data.login_name = Some(username);
                data.token = Some(access_token);
                data.authenticator = Some(authenticator::OAUTH.to_string());
            }
            Credentials::UserPasswordMfa {
                username,
                password,
                passcode_in_password,
                passcode,
            } => {
                let store_temp_cred = matches!(
                    &login_parameters.login_method,
                    LoginMethod::UserPasswordMfa {
                        client_store_temporary_credential: true,
                        ..
                    }
                );

                let cached_mfa_token = if store_temp_cred {
                    try_get_cached_token(
                        &login_parameters.server_url,
                        &username,
                        "",
                        TokenType::MfaToken,
                        token_cache.clone(),
                    )
                    .await
                } else {
                    None
                };

                data.login_name = Some(username);
                data.password = Some(password);
                data.authenticator = Some(authenticator::USERNAME_PASSWORD_MFA.to_string());

                if let Some(cached_token) = cached_mfa_token {
                    data.token = Some(cached_token);
                    data.token_from_cache_used = true;
                } else {
                    set_duo_authn_fields(&mut data, passcode_in_password, passcode.clone());
                    if store_temp_cred {
                        // Reference connector sends this inside SESSION_PARAMETERS, not as a
                        // top-level login field — the server ignores the top-level form.
                        data.session_parameters
                            .get_or_insert_with(HashMap::new)
                            .insert(
                                "CLIENT_REQUEST_MFA_TOKEN".to_string(),
                                serde_json::Value::Bool(true),
                            );
                    }
                }
            }
        },
    }
    Ok(data)
}

async fn send_login_request(
    client: &reqwest::Client,
    login_parameters: &LoginParameters,
    login_request: &AuthRequest,
    policy: &RetryPolicy,
) -> Result<AuthResponse, RestError> {
    use crate::http::retry::{HttpContext, execute_with_retry};

    let login_url = format!("{}/session/v1/login-request", login_parameters.server_url);
    tracing::info!(login_url = %login_url, "Making Snowflake login request");

    let user_agent = user_agent(&login_parameters.client_info);

    // Drift C.5: when the OAuth flow handed us a DPoP JWK alongside the
    // access token, sign a DPoP proof JWT for the Snowflake login URL on
    // every send (including retries — `proof_jwt` includes a fresh `jti`
    // and `iat` per RFC 9449 §4.2). The key is parsed once up front so a
    // malformed JWK fails the login fast instead of inside the retry
    // closure. Snowflake's GS does not issue `use_dpop_nonce` for login,
    // so we don't replicate the OAuth-token-endpoint nonce retry here
    // (matches JDBC `SessionUtil.java:746-750`).
    let dpop_signer: Option<DPoPSigner> =
        if let Some(jwk_json) = login_request.data.dpop_jwk_json.as_deref() {
            let key = oauth::dpop::DPoPKey::from_jwk_json(jwk_json).context(OAuthFlowSnafu)?;
            let url = Url::parse(&login_url).context(UrlJoinSnafu {
                path: "/session/v1/login-request",
            })?;
            Some(DPoPSigner {
                key: std::sync::Arc::new(key),
                url: std::sync::Arc::new(url),
            })
        } else {
            None
        };

    let build_request = || {
        let mut builder = client
            .post(&login_url)
            .query(&[
                (
                    "databaseName",
                    login_parameters.database.as_deref().unwrap_or_default(),
                ),
                (
                    "schemaName",
                    login_parameters.schema.as_deref().unwrap_or_default(),
                ),
                (
                    "warehouse",
                    login_parameters.warehouse.as_deref().unwrap_or_default(),
                ),
                (
                    "roleName",
                    login_parameters.role.as_deref().unwrap_or_default(),
                ),
            ])
            .json(login_request)
            .header("accept", "application/snowflake")
            .header("User-Agent", &user_agent)
            .header("Authorization", "Snowflake Token=\"None\"")
            .timeout(Duration::from_secs(30));
        if let Some(signer) = dpop_signer.as_ref() {
            // Signing is infallible once `from_jwk_json` succeeded above
            // (only openssl primitive failures could surface here, which
            // would have already failed the validation step).
            let proof = oauth::dpop::proof_jwt(&signer.key, "POST", &signer.url, None)
                .expect("DPoP proof generation must succeed for a pre-validated key");
            builder = builder.header("DPoP", proof.reveal());
        }
        builder
    };

    let ctx = HttpContext::new(Method::POST, "/session/v1/login-request").allow_post_retry();

    let response = execute_with_retry(build_request, &ctx, policy, |r| async move { Ok(r) })
        .await
        .context(HttpRetrySnafu {
            context: "login request",
            ids: QueryIds::default(),
        })?;

    read_response_json::<auth::AuthResponseMain>(response).await
}

/// Drift C.5: per-request DPoP signing context for `send_login_request`.
/// Holds an `Arc`-shared key and login URL so the `build_request`
/// closure (called once per retry attempt) can stamp a fresh proof JWT
/// without moving values out of the surrounding scope.
struct DPoPSigner {
    key: std::sync::Arc<oauth::dpop::DPoPKey>,
    url: std::sync::Arc<Url>,
}

#[tracing::instrument(
    skip(login_parameters, session_parameters, crl_worker),
    fields(account_name, login_name)
)]
pub async fn snowflake_login(
    login_parameters: &LoginParameters,
    session_parameters: Option<&HashMap<String, String>>,
    crl_worker: SharedCrlWorker,
) -> Result<LoginResult, RestError> {
    let client = build_tls_http_client(&login_parameters.client_info, crl_worker)?;
    let policy = RetryPolicy::default();
    snowflake_login_with_client(
        &client,
        login_parameters,
        session_parameters,
        None,
        None,
        &policy,
    )
    .await
}

#[tracing::instrument(
    skip(
        client,
        login_parameters,
        session_parameters,
        token_cache,
        retry_policy
    ),
    fields(account_name, login_name)
)]
pub async fn snowflake_login_with_client(
    client: &reqwest::Client,
    login_parameters: &LoginParameters,
    session_parameters: Option<&HashMap<String, String>>,
    token_cache: Option<std::sync::Arc<dyn TokenCache>>,
    prompt_locks: Option<&std::sync::Arc<prompt_lock::PromptLockMap>>,
    retry_policy: &RetryPolicy,
) -> Result<LoginResult, RestError> {
    tracing::info!("Starting Snowflake login process");

    // Record key fields in the span
    tracing::Span::current().record("account_name", &login_parameters.account_name);

    // Optional settings
    tracing::debug!(
        account_name = %login_parameters.account_name,
        server_url = %login_parameters.server_url,
        database = ?login_parameters.database,
        schema = ?login_parameters.schema,
        warehouse = ?login_parameters.warehouse,
        "Extracted connection settings"
    );

    // Session token bypass: validate the pre-acquired tokens via RENEW, which
    // also returns the server-assigned session ID needed for telemetry routing.
    if let LoginMethod::SessionToken {
        session_token,
        master_token,
        master_validity_in_seconds,
    } = &login_parameters.login_method
    {
        tracing::info!("Session token authentication: validating tokens via token-request RENEW");
        let master_validity = master_validity_in_seconds.map(std::time::Duration::from_secs);
        let temp_tokens = SessionTokens {
            session_token: session_token.clone(),
            master_token: master_token.clone(),
            session_id: 0, // unknown until refresh_session returns the real id
            session_expires_at: None,
            master_expires_at: master_validity.map(|d| std::time::Instant::now() + d),
            master_validity,
        };
        let tokens = refresh_session(
            client,
            &login_parameters.server_url,
            &login_parameters.client_info,
            &temp_tokens,
        )
        .await?;
        tracing::info!(
            session_id = tokens.session_id,
            "Session token authentication succeeded"
        );
        return Ok(LoginResult {
            tokens,
            session_parameters: None,
            database_name: None,
            schema_name: None,
            warehouse_name: None,
            role_name: None,
            server_version: None,
        });
    }

    // For interactive auth methods (external browser and MFA) that write a
    // token to the cache, acquire a per-<user, host> prompt-lock so that only
    // one connection in a pool drives the interactive step.  Waiters block
    // here, then re-read the cache inside `auth_request_data` (the existing
    // cache lookups serve as the post-lock double-check).  The lock is held
    // across `auth_request_data` + `send_login_request` + the EXT_AUTHN retry
    // block so the token is fully persisted before waiters proceed.
    // OAuth Authorization Code is serialized inside `run_oauth_authorization_code`.
    let _prompt_guard: Option<prompt_lock::PromptGuard> = if let Some(locks) = prompt_locks {
        match &login_parameters.login_method {
            LoginMethod::ExternalBrowser {
                username,
                client_store_temporary_credential: true,
                ..
            } if prompt_lock::is_eligible(
                true,
                login_parameters.disable_parallel_user_prompt,
                username,
            ) =>
            {
                tracing::debug!(%username, "Acquiring external-browser prompt lock");
                // ID-token keys hash only `snowflake` + `username`; `idp` and
                // `role` are excluded by `build_cache_key`, so leave them empty.
                let lock_key = CacheKey {
                    token_type: TokenType::IdToken,
                    idp: String::new(),
                    snowflake: normalize_url(&login_parameters.server_url),
                    username: normalize_identifier(username),
                    role: String::new(),
                };
                Some(prompt_lock::acquire(locks, &lock_key).await)
            }
            LoginMethod::UserPasswordMfa {
                username,
                client_store_temporary_credential: true,
                ..
            } if prompt_lock::is_eligible(
                true,
                login_parameters.disable_parallel_user_prompt,
                username,
            ) =>
            {
                tracing::debug!(%username, "Acquiring MFA prompt lock");
                let lock_key = CacheKey {
                    token_type: TokenType::MfaToken,
                    idp: String::new(),
                    snowflake: normalize_url(&login_parameters.server_url),
                    username: normalize_identifier(username),
                    role: String::new(),
                };
                Some(prompt_lock::acquire(locks, &lock_key).await)
            }
            _ => None,
        }
    } else {
        None
    };

    // Build the login request data (handles all auth methods including Okta SAML exchange).
    // For prompt-locked callers the existing cache lookups inside this function
    // (lines for ID token / MFA token) serve as the post-lock double-check.
    let login_request_data = auth_request_data(
        client,
        login_parameters,
        session_parameters,
        token_cache.clone(),
        prompt_locks,
        retry_policy,
    )
    .await?;
    tracing::Span::current().record("login_name", &login_request_data.login_name);
    let login_request = AuthRequest {
        data: login_request_data,
    };

    tracing::debug!(
        authenticator = ?login_request.data.authenticator,
        login_name = ?login_request.data.login_name,
        "Login request prepared (secrets redacted)"
    );

    // Send the actual login request
    let mut auth_response =
        send_login_request(client, login_parameters, &login_request, retry_policy).await?;

    // Revoke cached token and retry if cached token caused failure
    if !auth_response.success {
        let code = auth_response
            .code
            .as_deref()
            .and_then(|c| c.parse::<i32>().ok())
            .unwrap_or(-1);

        // Cached token (ID token or MFA) rejected with an EXT_AUTHN error:
        // evict it and retry via the normal interactive flow.
        if login_request.data.token_from_cache_used && EXT_AUTHN_ERROR_CODES.contains(&code) {
            if let Some((username, role, token_type)) = match &login_parameters.login_method {
                LoginMethod::ExternalBrowser { username, .. } => Some((
                    username.as_str(),
                    login_parameters.role.as_deref().unwrap_or(""),
                    TokenType::IdToken,
                )),
                LoginMethod::UserPasswordMfa { username, .. } => {
                    Some((username.as_str(), "", TokenType::MfaToken))
                }
                _ => None,
            } {
                tracing::warn!(
                    code,
                    %token_type,
                    "Cached token rejected, evicting and retrying"
                );
                remove_token_from_cache(
                    &login_parameters.server_url,
                    username,
                    role,
                    token_type,
                    token_cache.clone(),
                )
                .await;
                let retry_data = auth_request_data(
                    client,
                    login_parameters,
                    session_parameters,
                    token_cache.clone(),
                    prompt_locks,
                    retry_policy,
                )
                .await?;
                let retry_request = AuthRequest { data: retry_data };
                auth_response =
                    send_login_request(client, login_parameters, &retry_request, retry_policy)
                        .await?;
            }
        }
        // OAuth refresh-on-failure: when GS rejects the OAuth access token
        // with 390303 / 390318, replay the login once. For Authorization Code
        // we first evict the cached access token (and any DPoP-bundled entry)
        // so the replay exercises the refresh-token leg or, failing that, the
        // interactive flow. For Client Credentials there is no cache to evict
        // (CC tokens are not persisted), so the replay re-hits the IdP token
        // endpoint to fetch a fresh access token. Cross-driver consensus:
        // JDBC, ODBC, .NET, Go all retry both flows. Legacy `OAuthAccessToken`
        // bubbles the error since the caller supplies the token directly.
        else if OAUTH_REFRESH_ERROR_CODES.contains(&code) {
            let mut should_retry = false;
            match &login_parameters.login_method {
                LoginMethod::OAuthAuthorizationCode(cfg) => {
                    tracing::debug!(
                        code = code,
                        oauth_type = "OAUTH_AUTHORIZATION_CODE",
                        "OAuth access token cache eviction triggered by Snowflake error code {code}"
                    );
                    evict_oauth_access_token_for_authorization_code(
                        cfg,
                        &login_parameters.server_url,
                        login_parameters.role.as_deref().unwrap_or(""),
                        token_cache.clone(),
                    )
                    .await;
                    should_retry = true;
                }
                LoginMethod::OAuthClientCredentials(_) => {
                    // No cache to evict for CC (tokens are not persisted);
                    // the replay re-acquires from the IdP token endpoint.
                    tracing::debug!(
                        code = code,
                        oauth_type = "OAUTH_CLIENT_CREDENTIALS",
                        "Re-acquiring OAuth client-credentials access token after Snowflake error code {code}"
                    );
                    should_retry = true;
                }
                _ => {}
            }
            if should_retry {
                tracing::debug!("Retrying login after OAuth refresh");
                let retry_data = auth_request_data(
                    client,
                    login_parameters,
                    session_parameters,
                    token_cache.clone(),
                    prompt_locks,
                    retry_policy,
                )
                .await?;
                let retry_request = AuthRequest { data: retry_data };
                auth_response =
                    send_login_request(client, login_parameters, &retry_request, retry_policy)
                        .await?;
            }
        }
    }

    // If retry failed or unrecoverable, evict tokens from cache and fail
    if !auth_response.success {
        let message = auth_response
            .message
            .unwrap_or_else(|| "Unknown error".to_string());
        tracing::error!(message = %message, "Snowflake login failed");
        let code = auth_response
            .code
            .as_deref()
            .and_then(|c| c.parse::<i32>().ok())
            .unwrap_or(GS_CODE_UNAVAILABLE);
        if EXT_AUTHN_ERROR_CODES.contains(&code) {
            let evictable = match &login_parameters.login_method {
                LoginMethod::UserPasswordMfa { username, .. } => {
                    Some((username.as_str(), "", TokenType::MfaToken))
                }
                LoginMethod::ExternalBrowser { username, .. } => Some((
                    username.as_str(),
                    login_parameters.role.as_deref().unwrap_or(""),
                    TokenType::IdToken,
                )),
                _ => None,
            };
            if let Some((username, role, token_type)) = evictable {
                tracing::warn!(code, %token_type, "Evicting cached token after terminal login failure");
                remove_token_from_cache(
                    &login_parameters.server_url,
                    username,
                    role,
                    token_type,
                    token_cache.clone(),
                )
                .await;
            }
        }
        let reauthentication_required =
            is_reauthentication_required(code, &login_parameters.login_method);
        LoginSnafu {
            message,
            code,
            reauthentication_required,
        }
        .fail()?;
    }

    tracing::debug!("Login successful, extracting session tokens");

    // If success - cache response tokens (MFA or ID token) when caching is enabled.
    // Also, for IdToken, respect IdP consent: skip caching when explicitly denied.
    let cacheable_token: Option<(&str, &str, TokenType, &SensitiveString)> =
        match &login_parameters.login_method {
            LoginMethod::UserPasswordMfa {
                username,
                client_store_temporary_credential: true,
                ..
            } => auth_response
                .data
                .mfa_token
                .as_ref()
                .map(|t| (username.as_str(), "", TokenType::MfaToken, t)),
            LoginMethod::ExternalBrowser {
                username,
                client_store_temporary_credential: true,
                ..
            } if login_request.data.consent_cache_id_token != Some(false) => {
                // `build_cache_key` drops `role` for ID tokens; empty is intentional.
                auth_response
                    .data
                    .id_token
                    .as_ref()
                    .map(|t| (username.as_str(), "", TokenType::IdToken, t))
            }
            _ => None,
        };
    if let Some((username, role, token_type, token)) = cacheable_token {
        store_token_in_cache(
            &login_parameters.server_url,
            username,
            role,
            token_type,
            token.reveal(),
            token_cache,
        )
        .await;
    }

    // Extract tokens and session id from response
    let session_token = auth_response
        .data
        .token
        .context(MissingResponseFieldSnafu {
            field: "session token",
        })?;

    let master_token = auth_response
        .data
        .master_token
        .context(MissingResponseFieldSnafu {
            field: "master token",
        })?;

    let session_id = auth_response
        .data
        .session_id
        .context(MissingResponseFieldSnafu {
            field: "session ID",
        })?;

    let now = std::time::Instant::now();
    let session_expires_at = auth_response.data.validity.map(|d| now + d);
    let master_expires_at = auth_response.data.master_validity.map(|d| now + d);

    // Extract session parameters from auth response
    let session_params = auth_response.data._parameters.map(|params| {
        params
            .iter()
            .filter_map(|param| {
                // Convert JSON value to string
                let value_str = match &param._value {
                    serde_json::Value::String(s) => Some(s.clone()),
                    serde_json::Value::Number(n) => Some(n.to_string()),
                    serde_json::Value::Bool(b) => Some(b.to_string()),
                    serde_json::Value::Null => None,
                    other => {
                        tracing::debug!(
                            param_name = %param._name,
                            param_value = ?other,
                            "Unexpected JSON type for session parameter, skipping"
                        );
                        None
                    }
                };

                value_str.map(|v| (param._name.to_uppercase(), v))
            })
            .collect::<HashMap<String, String>>()
    });

    // Extract server-echoed sessionInfo names separately so they can be
    // stored on the connection as `final_session_names` (not mixed into
    // session parameters).
    let (database_name, schema_name, warehouse_name, role_name) =
        match &auth_response.data.session_info {
            Some(info) => (
                info.database_name.clone(),
                info.schema_name.clone(),
                info.warehouse_name.clone(),
                info.role_name.clone(),
            ),
            None => (None, None, None, None),
        };

    let server_version = auth_response.data.server_version.clone();

    tracing::info!(
        session_id,
        session_validity_secs = auth_response.data.validity.map(|d| d.as_secs()),
        master_validity_secs = auth_response.data.master_validity.map(|d| d.as_secs()),
        session_params_count = session_params.as_ref().map(|p| p.len()),
        server_version = server_version.as_deref(),
        "Snowflake login completed successfully"
    );
    Ok(LoginResult {
        tokens: SessionTokens {
            session_token,
            master_token,
            session_id,
            session_expires_at,
            master_expires_at,
            master_validity: auth_response.data.master_validity,
        },
        session_parameters: session_params,
        database_name,
        schema_name,
        warehouse_name,
        role_name,
        server_version,
    })
}

/// Refresh an expired session token using the master token.
///
/// When a session token expires (indicated by HTTP 401), this function can be called
/// to obtain new tokens without requiring a full re-authentication.
#[tracing::instrument(skip(client, client_info, tokens))]
pub async fn refresh_session(
    client: &reqwest::Client,
    server_url: &str,
    client_info: &ClientInfo,
    tokens: &SessionTokens,
) -> Result<SessionTokens, RestError> {
    tracing::info!(session_id = tokens.session_id, "Refreshing session token");

    let refresh_url = Url::parse(server_url)
        .and_then(|base| base.join(TOKEN_REQUEST_PATH))
        .context(UrlJoinSnafu {
            path: TOKEN_REQUEST_PATH,
        })?;

    // Build request body per gosnowflake: {"oldSessionToken": "...", "requestType": "RENEW"}
    let body = serde_json::json!({
        "oldSessionToken": tokens.session_token.reveal(),
        "requestType": "RENEW"
    });

    let request = client
        .post(refresh_url)
        .query(&[
            ("requestId", uuid::Uuid::new_v4().to_string()),
            ("request_guid", uuid::Uuid::new_v4().to_string()),
        ])
        // Authenticate with master token, not session token
        .header(
            header::AUTHORIZATION,
            format!("Snowflake Token=\"{}\"", tokens.master_token.reveal()),
        )
        .header(header::ACCEPT, "application/json")
        .header("User-Agent", user_agent(client_info))
        .json(&body)
        .build()
        .context(RequestConstructionSnafu {
            request: "session refresh",
        })?;

    let response = client.execute(request).await.context(CommunicationSnafu {
        context: "Failed to execute session refresh request",
    })?;

    let status = response.status();
    if !status.is_success() {
        tracing::error!(status = %status, "Session refresh request failed");
        return SessionRefreshSnafu { status }.fail();
    }

    let refresh_response =
        response
            .json::<RefreshSessionResponse>()
            .await
            .context(CommunicationSnafu {
                context: "Failed to parse session refresh response",
            })?;

    if !refresh_response.success {
        let message = refresh_response
            .message
            .unwrap_or_else(|| "Unknown error".to_string());
        let code = refresh_response
            .code
            .as_deref()
            .and_then(|c| c.parse::<i32>().ok())
            .unwrap_or(-1);
        tracing::error!(code, message = %message, "Session refresh failed");
        // GS 390113/390114/390115 on the refresh endpoint all mean the master
        // token can never be renewed (not found, expired, or invalid).
        // Surface the discriminable MasterTokenTerminal variant, carrying the
        // real code, so callers can mark the connection expired, mirroring
        // the query-response path in read_response_json.
        if MASTER_TOKEN_TERMINAL_CODES.contains(&code) {
            return MasterTokenTerminalSnafu { code }.fail();
        }
        return SessionRefreshFailedSnafu { message, code }.fail();
    }

    let data = refresh_response.data.context(MissingResponseFieldSnafu {
        field: "session refresh data",
    })?;

    let now = std::time::Instant::now();
    let session_expires_at = data.validity.map(|d| now + d);
    let master_expires_at = data.master_validity.map(|d| now + d);

    tracing::info!(
        session_id = data.session_id,
        session_validity_secs = data.validity.map(|d| d.as_secs()),
        master_validity_secs = data.master_validity.map(|d| d.as_secs()),
        "Session refreshed successfully"
    );

    Ok(SessionTokens {
        session_token: data.session_token,
        master_token: data.master_token,
        session_id: data.session_id,
        session_expires_at,
        master_expires_at,
        master_validity: data.master_validity,
    })
}

/// Result of a token request (ISSUE or RENEW).
pub struct TokenRequestResult {
    pub session_token: SensitiveString,
    /// Validity in seconds as returned by the server.
    /// `None` when the server omits the validity field.
    pub validity_in_seconds: Option<i64>,
}

impl std::fmt::Debug for TokenRequestResult {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TokenRequestResult")
            .field("session_token", &"[REDACTED]")
            .field("validity_in_seconds", &self.validity_in_seconds)
            .finish()
    }
}

/// Send a token request (ISSUE or RENEW) to the Snowflake server.
///
/// This reuses the same endpoint and authentication as `refresh_session`
/// but allows specifying the request type and returns minimal structured data.
pub async fn token_request(
    client: &reqwest::Client,
    server_url: &str,
    client_info: &ClientInfo,
    tokens: &SessionTokens,
    request_type: &str,
) -> Result<TokenRequestResult, RestError> {
    let token_url = Url::parse(server_url)
        .and_then(|base| base.join(TOKEN_REQUEST_PATH))
        .context(UrlJoinSnafu {
            path: TOKEN_REQUEST_PATH,
        })?;

    let body = serde_json::json!({
        "oldSessionToken": tokens.session_token.reveal(),
        "requestType": request_type,
    });

    let request = client
        .post(token_url)
        .query(&[
            ("requestId", uuid::Uuid::new_v4().to_string()),
            ("request_guid", uuid::Uuid::new_v4().to_string()),
        ])
        .header(
            header::AUTHORIZATION,
            format!("Snowflake Token=\"{}\"", tokens.master_token.reveal()),
        )
        .header(header::ACCEPT, "application/json")
        .header("User-Agent", user_agent(client_info))
        .json(&body)
        .build()
        .context(RequestConstructionSnafu {
            request: "token request",
        })?;

    let response = client.execute(request).await.context(CommunicationSnafu {
        context: "Failed to execute token request",
    })?;

    let status = response.status();
    if !status.is_success() {
        return TokenRequestHttpSnafu {
            operation: request_type.to_string(),
            status,
        }
        .fail();
    }

    let token_response =
        response
            .json::<TokenRequestResponse>()
            .await
            .context(CommunicationSnafu {
                context: "Failed to parse token request response",
            })?;

    if !token_response.success {
        let message = token_response
            .message
            .unwrap_or_else(|| "Unknown error".to_string());
        let code = token_response
            .code
            .as_deref()
            .and_then(|c| c.parse::<i32>().ok())
            .unwrap_or(-1);
        return TokenRequestFailedSnafu {
            operation: request_type.to_string(),
            message,
            code,
        }
        .fail();
    }

    let data = token_response.data.context(MissingResponseFieldSnafu {
        field: "token request data",
    })?;

    Ok(TokenRequestResult {
        session_token: data.session_token,
        validity_in_seconds: data.validity.and_then(|d| i64::try_from(d.as_secs()).ok()),
    })
}

#[tracing::instrument(
    skip(query_parameters, session_token, query_input, crl_worker),
    fields(sql)
)]
pub async fn snowflake_query<'a>(
    query_parameters: QueryParameters,
    session_token: impl AsRef<str>,
    query_input: QueryInput<'a>,
    options: QueryOptions,
    crl_worker: SharedCrlWorker,
) -> Result<query_response::Response, RestError> {
    let client = build_tls_http_client(&query_parameters.client_info, crl_worker)?;
    snowflake_query_with_client(
        &client,
        query_parameters,
        session_token,
        query_input,
        options,
    )
    .await
}

/// Execute a query with a caller-supplied HTTP client and [`QueryOptions`].
///
/// See [`QueryOptions`] for the rarely-varied knobs (retry policy, execution
/// mode, caller-supplied `requestId`); every field defaults to the common
/// case, so most callers pass `QueryOptions::default()`.
#[tracing::instrument(
    skip(client, query_parameters, session_token, query_input, options),
    fields(sql)
)]
pub async fn snowflake_query_with_client<'a>(
    client: &reqwest::Client,
    query_parameters: QueryParameters,
    session_token: impl AsRef<str>,
    query_input: QueryInput<'a>,
    options: QueryOptions,
) -> Result<query_response::Response, RestError> {
    let QueryOptions {
        retry_policy,
        execution_mode,
        request_id,
    } = options;
    let request_id = request_id.unwrap_or_else(uuid::Uuid::new_v4);
    let session_token = session_token.as_ref();

    // Async mode path (legacy, opt-in)
    if matches!(execution_mode, QueryExecutionMode::Async) {
        let response = execute_async_with_fallback(
            client,
            &query_parameters,
            session_token,
            query_input,
            &retry_policy,
            request_id,
        )
        .await?;
        return Ok(response);
    }

    // Sync mode (default): use requestId-based retry for connection failures
    execute_sync_query(
        client,
        &query_parameters,
        session_token,
        &query_input,
        request_id,
        &retry_policy,
    )
    .await
}

/// Execute query in async mode with fallback to sync for error 612.
/// Returns the response and the client-generated request UUID used on the wire.
async fn execute_async_with_fallback<'a>(
    client: &reqwest::Client,
    query_parameters: &QueryParameters,
    session_token: &str,
    query_input: QueryInput<'a>,
    retry_policy: &RetryPolicy,
    request_id: uuid::Uuid,
) -> Result<query_response::Response, RestError> {
    match snowflake_query_async_style(
        client,
        query_parameters,
        session_token,
        &query_input,
        retry_policy,
        request_id,
    )
    .await
    {
        Ok(response) => return Ok(response),
        Err(RestError::AsyncPollResultNotFound {
            is_first_poll: true,
            ..
        }) => {
            // Error 612 "Result not found" on first poll - fall through to sync retry.
        }
        Err(
            e @ RestError::AsyncPollResultNotFound {
                is_first_poll: false,
                ..
            },
        ) => {
            // guarded with: query_log_text, query_log_parameters
            let (sql, bindings) = query_log_fields(query_parameters, &query_input);
            tracing::error!(
                request_id = ?request_id,
                sql = sql,
                bindings = bindings,
                "Error 612 after prior successful polls; not retrying"
            );
            return Err(e);
        }
        Err(e) => return Err(e),
    }

    // Fallback to sync after 612
    let response = execute_sync_query(
        client,
        query_parameters,
        session_token,
        &query_input,
        request_id,
        retry_policy,
    )
    .await?;

    // Log based on actual command type after sync completes (we always get here via 612)
    let is_file_transfer = response
        .data
        .command
        .as_deref()
        .map(|c| c.eq_ignore_ascii_case("UPLOAD") || c.eq_ignore_ascii_case("DOWNLOAD"))
        .unwrap_or(false);
    if is_file_transfer {
        tracing::info!(
            command = response.data.command.as_deref(),
            "Retried async 612 with sync; confirmed file transfer"
        );
    } else {
        tracing::warn!(
            command = response.data.command.as_deref(),
            "Retried async 612 with sync; unexpected non-file-transfer query"
        );
    }

    Ok(response)
}

/// Build [`RestError::QueryFailed`] from a Snowflake query-response envelope.
pub(super) fn query_failed_from_response(
    response: query_response::Response,
    ids: &QueryIds,
) -> RestError {
    let message = response
        .message
        .unwrap_or_else(|| "Unknown error".to_owned());
    let code = response.code.as_deref().and_then(|c| c.parse::<i32>().ok());
    QueryFailedSnafu {
        message,
        code,
        sql_state: response.data.sql_state,
        ids: ids.clone(),
    }
    .build()
}

/// Map a Snowflake query response into a `Result`, converting
/// `response.success == false` into `RestError::QueryFailed`.
pub(super) fn into_query_result(
    response: query_response::Response,
    ids: &QueryIds,
) -> Result<query_response::Response, RestError> {
    if !response.success {
        return Err(query_failed_from_response(response, ids));
    }
    Ok(response)
}

/// Execute a single sync query request with HTTP-level retries.
///
/// The `requestId` is stable across every HTTP attempt inside
/// `execute_with_retry` so that Snowflake can dedupe replays via its usual
/// request-id machinery. The first attempt is sent as a fresh request; every
/// replay (attempt ≥ 2) additionally carries `retry=true`, which is the
/// Snowflake-documented hint for "look up this requestId in the dedup
/// table". If the retry budget is exhausted the error surfaces as
/// [`RestError::HttpRetry`].
async fn execute_sync_query<'a>(
    client: &reqwest::Client,
    query_parameters: &QueryParameters,
    session_token: &str,
    query_input: &QueryInput<'a>,
    request_id: uuid::Uuid,
    retry_policy: &RetryPolicy,
) -> Result<query_response::Response, RestError> {
    use crate::http::retry::{HttpContext, execute_with_retry};

    // guarded with: log_query_text, log_query_parameters
    let (sql, bindings) = query_log_fields(query_parameters, query_input);
    tracing::info!(
        request_id = %request_id,
        sql = sql,
        bindings = bindings,
        "Executing sync query"
    );

    let query_request = query_request::Request {
        sql_text: query_input.sql.clone(),
        async_exec: false,
        sequence_id: 1,
        query_submission_time: SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis() as i64,
        is_internal: false,
        describe_only: query_input.describe_only,
        parameters: query_input.query_parameters.clone(),
        bindings: query_input.bindings,
        bind_stage: query_input.bind_stage.clone(),
        query_context: query_request::QueryContext { entries: None },
    };

    let query_url = Url::parse(query_parameters.server_url.as_str())
        .and_then(|base| base.join(QUERY_REQUEST_PATH))
        .context(UrlJoinSnafu {
            path: QUERY_REQUEST_PATH,
        })?;

    // Base query parameters. `retry=true` is added for every HTTP replay
    // inside `execute_with_retry` below (attempt ≥ 2) — it is always safe
    // per Snowflake docs, and when the server has already seen this
    // `requestId` it improves dedupe accuracy.
    let base_query_params = vec![
        ("requestId", request_id.to_string()),
        ("request_guid", uuid::Uuid::new_v4().to_string()),
    ];

    let send_start = Instant::now();
    let attempt_counter = std::sync::atomic::AtomicU32::new(0);
    let build_request = || {
        let n = attempt_counter.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        let mut params = base_query_params.clone();
        if n >= 1 {
            params.push(("retry", "true".to_string()));
        }
        apply_json_content_type(apply_query_headers(
            client.post(query_url.clone()),
            &query_parameters.client_info,
            session_token,
        ))
        .query(&params)
        .json(&query_request)
    };

    let ctx = HttpContext::new(Method::POST, QUERY_REQUEST_PATH).allow_post_retry();

    let response = execute_with_retry(build_request, &ctx, retry_policy, |r| async move { Ok(r) })
        .await
        .context(HttpRetrySnafu {
            context: "query request",
            ids: QueryIds {
                request_id: Some(request_id),
                query_id: None,
            },
        })?;

    let query_response = read_response_json::<query_response::Data>(response).await?;

    let elapsed_ms = send_start.elapsed().as_secs_f64() * 1000.0;
    tracing::debug!(
        elapsed_ms,
        request_id = %request_id,
        query_id = query_response.data.query_id.as_deref().unwrap_or_default(),
        "Sync query response received"
    );

    let ids = QueryIds {
        request_id: Some(request_id),
        query_id: query_response.data.query_id.clone(),
    };
    let query_response = if async_exec::should_poll_for_completion(&query_response) {
        tracing::debug!(request_id = %request_id, "detached query - polling for completion");
        async_exec::poll_detached_query(
            client,
            query_parameters,
            session_token,
            &query_response,
            retry_policy,
            &ids,
        )
        .await?
    } else {
        query_response
    };

    into_query_result(query_response, &ids)
}

/// New blocking facade that uses the async engine under the hood.
#[tracing::instrument(
    skip(client, query_parameters, session_token, query_input),
    fields(sql)
)]
pub async fn snowflake_query_async_style<'a, S: AsRef<str>>(
    client: &reqwest::Client,
    query_parameters: &QueryParameters,
    session_token: S,
    query_input: &QueryInput<'a>,
    retry_policy: &RetryPolicy,
    request_id: uuid::Uuid,
) -> Result<query_response::Response, RestError> {
    async_exec::execute_blocking_with_async(
        client,
        query_parameters,
        session_token.as_ref(),
        query_input,
        request_id,
        retry_policy,
    )
    .await
}

/// Fetch the result of a previously executed query by its Snowflake Query ID.
///
/// Issues `GET /queries/{query_id}/result` using the connection's session token,
/// validates the response, and returns the parsed query response on success.
/// Returns `RestError` so callers can use `RefreshContext` for token refresh.
#[tracing::instrument(skip(client, query_parameters, session_token))]
pub async fn snowflake_get_query_result(
    client: &reqwest::Client,
    query_parameters: &QueryParameters,
    session_token: &str,
    query_id: &str,
    retry_policy: &RetryPolicy,
) -> Result<query_response::Response, RestError> {
    tracing::info!(query_id = query_id, "Fetching query result");

    let result_url = format!(
        "{}/queries/{}/result",
        query_parameters.server_url, query_id
    );
    let ids = QueryIds {
        request_id: None,
        query_id: Some(query_id.to_owned()),
    };
    let query_response = async_exec::poll_query_status(
        client,
        &query_parameters.client_info,
        session_token,
        &result_url,
        retry_policy,
        &ids,
    )
    .await?;

    into_query_result(query_response, &ids)
}

/// Result of a query status check via the monitoring endpoint.
#[derive(Debug)]
pub struct QueryStatusResult {
    pub status_name: String,
    pub error_code: Option<i32>,
    pub error_message: Option<String>,
    pub end_time: i64,
    pub start_time: i64,
    pub total_duration: i32,
    pub query_id: String,
    pub session_id: i64,
    pub sql_text: String,
    pub warehouse_id: i64,
    pub warehouse_name: Option<String>,
    pub warehouse_external_size: Option<String>,
    pub warehouse_server_type: Option<String>,
    pub state: String,
}

const MONITORING_QUERIES_PATH: &str = "/monitoring/queries/";

/// Check the status of a query by its ID via the `/monitoring/queries/{query_id}` endpoint.
#[tracing::instrument(skip(client, client_info, session_token))]
pub async fn get_query_status(
    client: &reqwest::Client,
    server_url: &str,
    client_info: &ClientInfo,
    session_token: &SensitiveString,
    query_id: &str,
    retry_policy: &RetryPolicy,
) -> Result<QueryStatusResult, RestError> {
    use crate::http::retry::{HttpContext, execute_with_retry};

    let mut url = Url::parse(server_url)
        .and_then(|base| base.join(MONITORING_QUERIES_PATH))
        .context(UrlJoinSnafu {
            path: MONITORING_QUERIES_PATH,
        })?;

    {
        let url_str = url.to_string();
        url.path_segments_mut()
            .map_err(|()| InvalidUrlSnafu { url: url_str }.build())?
            .push(query_id);
    }

    let token_str = session_token.reveal();
    let build_request = || {
        apply_query_headers(client.get(url.clone()), client_info, token_str.as_ref()).query(&[
            ("requestId", uuid::Uuid::new_v4().to_string()),
            ("request_guid", uuid::Uuid::new_v4().to_string()),
        ])
    };

    let ctx = HttpContext::new(Method::GET, MONITORING_QUERIES_PATH);
    let ids = QueryIds {
        request_id: None,
        query_id: Some(query_id.to_owned()),
    };
    let response = execute_with_retry(build_request, &ctx, retry_policy, |r| async move { Ok(r) })
        .await
        .with_context(|_| HttpRetrySnafu {
            context: "query status",
            ids: ids.clone(),
        })?;

    let body: QueryStatusResponse =
        read_response_json::<Option<QueryStatusResponseData>>(response).await?;

    if !body.success {
        let message = body.message.unwrap_or_else(|| "Unknown error".to_owned());
        let code = body.code.as_deref().and_then(|c| c.parse::<i32>().ok());
        return QueryFailedSnafu {
            message,
            code,
            sql_state: None::<String>,
            ids,
        }
        .fail();
    }

    let data = body.data.context(MissingResponseFieldSnafu {
        field: "data in monitoring response",
    })?;

    let query_entry = data
        .queries
        .into_iter()
        .next()
        .context(MissingResponseFieldSnafu {
            field: "queries[0] in monitoring response",
        })?;

    let error_code = query_entry
        .error_code
        .as_deref()
        .and_then(|c| c.parse::<i32>().ok())
        .and_then(|c| if c == 0 { None } else { Some(c) });

    let error_message = if error_code.is_some() {
        query_entry.error_message.filter(|m| !m.is_empty())
    } else {
        None
    };

    Ok(QueryStatusResult {
        status_name: query_entry.status,
        error_code,
        error_message,
        end_time: query_entry.end_time,
        start_time: query_entry.start_time,
        total_duration: query_entry.total_duration,
        query_id: query_entry.id,
        session_id: query_entry.session_id,
        sql_text: query_entry.sql_text,
        warehouse_id: query_entry.warehouse_id,
        warehouse_name: query_entry.warehouse_name,
        warehouse_external_size: query_entry.warehouse_external_size,
        warehouse_server_type: query_entry.warehouse_server_type,
        state: query_entry.state,
    })
}

type QueryStatusResponse = SnowflakeResponse<Option<QueryStatusResponseData>>;

#[derive(Debug, serde::Deserialize)]
struct QueryStatusResponseData {
    queries: Vec<QueryStatusEntry>,
}

#[derive(Debug, serde::Deserialize)]
struct QueryStatusEntry {
    status: String,
    #[serde(
        rename = "errorCode",
        default,
        deserialize_with = "deserialize_string_or_int"
    )]
    error_code: Option<String>,
    #[serde(rename = "errorMessage")]
    error_message: Option<String>,
    #[serde(rename = "endTime", default)]
    end_time: i64,
    #[serde(rename = "startTime", default)]
    start_time: i64,
    #[serde(rename = "totalDuration", default)]
    total_duration: i32,
    #[serde(default)]
    id: String,
    #[serde(rename = "sessionId", default)]
    session_id: i64,
    #[serde(rename = "sqlText", default)]
    sql_text: String,
    #[serde(rename = "warehouseId", default)]
    warehouse_id: i64,
    #[serde(rename = "warehouseName")]
    warehouse_name: Option<String>,
    #[serde(rename = "warehouseExternalSize")]
    warehouse_external_size: Option<String>,
    #[serde(rename = "warehouseServerType")]
    warehouse_server_type: Option<String>,
    #[serde(default)]
    state: String,
}

/// Snowflake returns `errorCode` as either a JSON string (`"002003"`) or an
/// integer (`0`). This deserializer accepts both and normalises to `Option<String>`.
fn deserialize_string_or_int<'de, D>(deserializer: D) -> Result<Option<String>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    use serde::Deserialize;
    let value = Option::<serde_json::Value>::deserialize(deserializer)?;
    match value {
        Some(serde_json::Value::String(s)) => Ok(Some(s)),
        Some(serde_json::Value::Number(n)) => Ok(Some(n.to_string())),
        Some(serde_json::Value::Null) | None => Ok(None),
        Some(other) => Err(serde::de::Error::custom(format!(
            "expected string or number for errorCode, got {other}"
        ))),
    }
}

/// Outcome of a [`snowflake_abort_query`] call. A server-declined abort
/// (the query was not running — e.g. already completed, or never started) is
/// an expected outcome, not an error — only genuine failures (bad handle,
/// transport, session) propagate as `Err`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AbortOutcome {
    /// The query was running and the abort was acknowledged.
    Aborted,
    /// The query was not running (e.g. already completed, or never
    /// started); nothing to abort.
    NotRunning,
}

/// Abort a running query by its Snowflake Query ID.
///
/// Issues `POST /queries/{query_id}/abort-request` with an empty JSON body.
/// Returns `Ok(AbortOutcome::Aborted)` when the server acknowledges the abort
/// (`success: true`), or `Ok(AbortOutcome::NotRunning)` when it declines
/// (the query was not running — e.g. already completed, or never started) —
/// this is a normal outcome, not an error. Transport, parse, and
/// session-token errors still propagate as `Err`.
#[tracing::instrument(skip(client, query_parameters, session_token))]
pub async fn snowflake_abort_query(
    client: &reqwest::Client,
    query_parameters: &QueryParameters,
    session_token: &str,
    query_id: &str,
) -> Result<AbortOutcome, RestError> {
    let abort_url = format!(
        "{}/queries/{}/abort-request",
        query_parameters.server_url, query_id
    );

    let request = apply_json_content_type(apply_query_headers(
        client.post(&abort_url),
        &query_parameters.client_info,
        session_token,
    ))
    .json(&serde_json::json!({}))
    .build()
    .context(RequestConstructionSnafu {
        request: "abort_query",
    })?;

    let response = client.execute(request).await.context(CommunicationSnafu {
        context: "Failed to execute abort query request",
    })?;

    let abort_response: query_response::AbortQueryResponse = read_response_json(response).await?;

    Ok(if abort_response.success {
        AbortOutcome::Aborted
    } else {
        AbortOutcome::NotRunning
    })
}

/// Cancel a running query by its client-generated `requestId` (not its
/// server-assigned `queryId`).
///
/// Issues `POST /queries/v1/abort-request` with the JSON body
/// `{ "sqlText": <sql>, "requestId": <request_id> }`. The `requestId` in the
/// body identifies the in-flight query to abort; the `requestId` query
/// parameter is this abort request's own id (a fresh uuid), consistent with
/// every other Snowflake REST call.
///
/// The endpoint returns HTTP 200 for business-logic outcomes — the real result
/// is in the JSON envelope. Mirrors [`snowflake_abort_query`]: returns
/// `Ok(AbortOutcome::Aborted)` when the server acknowledges the abort
/// (`success: true`, meaning the request was *processed*, not a guarantee the
/// query stopped), or `Ok(AbortOutcome::NotRunning)` when it declines
/// (`success: false` — the query was not running) — a normal outcome, not an
/// error. Session-token expiry (`390112`) and other transport/parse failures
/// still propagate as `Err` (`read_response_json` maps `390112` to
/// `SessionExpired` centrally so `with_valid_session` can renew-and-retry).
#[tracing::instrument(skip(client, query_parameters, session_token, sql_text))]
pub async fn snowflake_cancel_query(
    client: &reqwest::Client,
    query_parameters: &QueryParameters,
    session_token: &str,
    request_id: &str,
    sql_text: &str,
) -> Result<AbortOutcome, RestError> {
    let abort_url = Url::parse(query_parameters.server_url.as_str())
        .and_then(|base| base.join(ABORT_REQUEST_PATH))
        .context(UrlJoinSnafu {
            path: ABORT_REQUEST_PATH,
        })?;

    tracing::info!(
        method = %Method::POST,
        host = abort_url.host_str().unwrap_or("<none>"),
        path = abort_url.path(),
        "outbound HTTP call"
    );

    let client_start_time = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis().to_string())
        .unwrap_or_else(|_| "0".to_owned());

    let request = apply_json_content_type(apply_query_headers(
        client.post(abort_url),
        &query_parameters.client_info,
        session_token,
    ))
    .query(&[
        ("requestId", uuid::Uuid::new_v4().to_string()),
        ("request_guid", uuid::Uuid::new_v4().to_string()),
        ("clientStartTime", client_start_time),
        ("retryCount", "0".to_owned()),
    ])
    .json(&serde_json::json!({
        "sqlText": sql_text,
        "requestId": request_id,
    }))
    .build()
    .context(RequestConstructionSnafu {
        request: "cancel_query",
    })?;

    let response = client.execute(request).await.context(CommunicationSnafu {
        context: "Failed to execute cancel query request",
    })?;

    tracing::info!(status = response.status().as_u16(), "HTTP response");

    let cancel_response: query_response::AbortQueryResponse = read_response_json(response).await?;

    // `success: false` here is a business-logic decline (the query was not
    // running), not a failure — session-token expiry (390112) has already been
    // mapped to `SessionExpired` by `read_response_json`.
    Ok(if cancel_response.success {
        AbortOutcome::Aborted
    } else {
        AbortOutcome::NotRunning
    })
}

/// Standard Snowflake JSON response envelope: `{success, code, message, data: T}`.
///
/// Every REST endpoint parsed by [`read_response_json`] returns this shape; the
/// generic `T` is the endpoint-specific payload. Keeping the envelope uniform
/// lets `read_response_json` inspect `success` + `code` centrally and map
/// body-level `390112` (session-token expired) to [`RestError::SessionExpired`]
/// for the single-flight `RefreshContext` refresh path — without each caller
/// having to re-implement that check.
#[derive(Debug, serde::Deserialize)]
#[serde(bound(deserialize = "T: serde::de::Deserialize<'de> + Default"))]
pub struct SnowflakeResponse<T> {
    pub success: bool,
    #[serde(default)]
    pub code: Option<String>,
    #[serde(default)]
    pub message: Option<String>,
    /// GS sends an explicit `"data": null` for some responses (e.g.
    /// ROLLBACK/COMMIT with no active transaction); absent and explicit-null
    /// collapse to `T::default()`, matching how the Python and Node drivers
    /// handle it.
    #[serde(default, deserialize_with = "deserialize_null_as_default")]
    pub data: T,
}

fn deserialize_null_as_default<'de, D, T>(deserializer: D) -> Result<T, D::Error>
where
    D: serde::Deserializer<'de>,
    T: serde::de::Deserialize<'de> + Default,
{
    Ok(Option::<T>::deserialize(deserializer)?.unwrap_or_default())
}

pub(crate) async fn read_response_json<T>(
    response: reqwest::Response,
) -> Result<SnowflakeResponse<T>, RestError>
where
    T: serde::de::DeserializeOwned + Default,
{
    let response_status = response.status();
    let response_text = response.text().await;

    if !response_status.is_success() {
        // Return SessionExpired so caller can refresh and retry
        if response_status == reqwest::StatusCode::UNAUTHORIZED {
            return SessionExpiredSnafu.fail();
        }
        // Do not embed the raw body in the error message, as this JSON may contain sensitive data.
        let body = response_text.unwrap_or("Unknown error".to_string());
        let message = format!("Unexpected response, body length {}.", body.len());
        return ResponseStatusSnafu {
            status: response_status,
            message,
        }
        .fail()
        .context(InvalidSnowflakeResponseSnafu);
    }

    let response_text = response_text
        .context(ResponseTextSnafu)
        .context(InvalidSnowflakeResponseSnafu)?;

    tracing::debug!(response_len = response_text.len(), "Received HTTP response");
    let parsed: SnowflakeResponse<T> = serde_json::from_str(&response_text)
        .context(ResponseFormatSnafu)
        .context(InvalidSnowflakeResponseSnafu)?;

    // 2xx with `success:false, code:"390112"` means the session token expired.
    // Surface it as SessionExpired so the RefreshContext can refresh and retry,
    // matching the HTTP 401 branch above.
    if !parsed.success
        && parsed.code.as_deref().and_then(|c| c.parse::<i32>().ok()) == Some(SESSION_TOKEN_EXPIRED)
    {
        return SessionExpiredSnafu.fail();
    }

    // 2xx with `success:false, code:"390113"/"390114"/"390115"` means the
    // master token can never be renewed. Surface it so RefreshContext can set
    // `is_master_token_expired = true` and propagate `MasterTokenTerminal` to
    // the caller, carrying the real code.
    if !parsed.success
        && let Some(code) = parsed.code.as_deref().and_then(|c| c.parse::<i32>().ok())
        && MASTER_TOKEN_TERMINAL_CODES.contains(&code)
    {
        return MasterTokenTerminalSnafu { code }.fail();
    }

    Ok(parsed)
}

#[track_caller]
fn build_tls_http_client(
    client_info: &ClientInfo,
    crl_worker: SharedCrlWorker,
) -> Result<reqwest::Client, RestError> {
    create_tls_client_with_proxy(
        client_info.tls_config.clone(),
        Some(&client_info.proxy_config),
        crl_worker,
    )
    .context(CrlValidationSnafu)
}

pub(crate) fn authorization_header(session_token: &str) -> header::HeaderValue {
    let value = format!("Snowflake Token=\"{session_token}\"");
    header::HeaderValue::from_str(&value).expect("authorization header construction must succeed")
}

pub(crate) fn json_header_value() -> header::HeaderValue {
    header::HeaderValue::from_static("application/json")
}

pub(crate) fn apply_query_headers(
    builder: reqwest::RequestBuilder,
    client_info: &ClientInfo,
    session_token: &str,
) -> reqwest::RequestBuilder {
    builder
        .header(header::AUTHORIZATION, authorization_header(session_token))
        .header(header::ACCEPT, json_header_value())
        .header("User-Agent", user_agent(client_info))
}

pub(crate) fn apply_json_content_type(builder: reqwest::RequestBuilder) -> reqwest::RequestBuilder {
    builder.header(header::CONTENT_TYPE, json_header_value())
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct QueryIds {
    pub request_id: Option<Uuid>,
    pub query_id: Option<String>,
}

#[derive(Debug, Snafu, error_trace::ErrorTrace)]
pub enum RestError {
    #[snafu(display("{operation} timed out after {budget:?}"))]
    #[snafu(visibility(pub(crate)))]
    OperationTimeout {
        operation: String,
        budget: std::time::Duration,
        /// Empty on login timeout; filled on the statement-poll path.
        ids: QueryIds,
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("Authentication failed"))]
    Authentication {
        source: AuthError,
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("Native Okta SSO failed"))]
    NativeOkta {
        source: native_okta::NativeOktaError,
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("External browser SSO failed"))]
    ExternalBrowser {
        source: external_browser::ExternalBrowserError,
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("OAuth flow failed"))]
    OAuthFlow {
        source: oauth::OAuthError,
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("Workload Identity Federation attestation failed: {source}"))]
    WorkloadIdentityAttestation {
        source: workload_identity::AttestationError,
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("Invalid Snowflake response"))]
    InvalidSnowflakeResponse {
        source: SnowflakeResponseError,
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("Session expired - reauthentication required"))]
    SessionExpired {
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("{}", master_token_terminal_detail(Some(*code))))]
    MasterTokenTerminal {
        /// The real GS code (390113/390114/390115) that triggered this. Always
        /// present here — this variant is only constructed at the server
        /// round-trip detection sites.
        code: i32,
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("Failed to communicate with Snowflake"))]
    Communication {
        context: String,
        source: reqwest::Error,
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("Failed to build request: {request}"))]
    RequestConstruction {
        request: String,
        source: reqwest::Error,
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("TLS client creation failed"))]
    CrlValidation {
        source: TlsError,
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("Login error: {message}, code: {code}"))]
    LoginError {
        message: String,
        code: i32,
        /// True when `code` is reauth-shaped AND the driver can re-drive the
        /// credential-acquisition flow itself for this login method — the
        /// conjunction of both predicates, per [`is_reauthentication_required`].
        reauthentication_required: bool,
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("Failed to build Snowflake URL for {path}: {source}"))]
    UrlJoin {
        path: &'static str,
        source: url::ParseError,
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("Session refresh HTTP request failed with status {status}"))]
    SessionRefresh {
        status: reqwest::StatusCode,
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("Session refresh failed: {message} (code: {code})"))]
    SessionRefreshFailed {
        message: String,
        code: i32,
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("Token request ({operation}) HTTP request failed with status {status}"))]
    TokenRequestHttp {
        operation: String,
        status: reqwest::StatusCode,
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("Token request ({operation}) failed: {message} (code: {code})"))]
    TokenRequestFailed {
        operation: String,
        message: String,
        code: i32,
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("Heartbeat failed: {message} (code: {code})"))]
    Heartbeat {
        message: String,
        code: i32,
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("Missing response field: {field}"))]
    MissingResponseField {
        field: &'static str,
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("{message}"))]
    QueryFailed {
        message: String,
        /// Snowflake server error code (e.g. 1003 for syntax error).
        code: Option<i32>,
        /// ANSI SQL state code (e.g. "42000" for syntax error).
        sql_state: Option<String>,
        ids: QueryIds,
        #[snafu(implicit)]
        location: Location,
    },
    /// Error 612 from async polling — triggers automatic retry with sync
    /// mode only on the first poll. If we've already made progress, don't retry.
    #[snafu(display("Async poll returned error 612 (result not found)"))]
    AsyncPollResultNotFound {
        /// True if this was the first poll attempt (safe to retry with sync).
        is_first_poll: bool,
        ids: QueryIds,
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("Async query response missing getResultUrl; cannot poll for completion"))]
    MissingResultUrl {
        ids: QueryIds,
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("Async query did not report a queryId"))]
    MissingQueryId {
        ids: QueryIds,
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("HTTP request failed after retries: {context}"))]
    HttpRetry {
        context: &'static str,
        /// Empty on login and logout; filled on query/poll HTTP failures.
        ids: QueryIds,
        source: crate::http::retry::HttpError,
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("Logout failed: {message} (code: {code})"))]
    Logout {
        message: String,
        code: i32,
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("Invalid URL ({url_safe})", url_safe = url_for_log(url)))]
    InvalidUrl {
        url: String,
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("Failed to encode telemetry payload: {reason}"))]
    PayloadEncode {
        reason: String,
        #[snafu(implicit)]
        location: Location,
    },
}

#[derive(Debug, Snafu, error_trace::ErrorTrace)]
pub enum SnowflakeResponseError {
    #[snafu(display("Failed to parse Snowflake response {source}"))]
    ResponseFormat {
        source: serde_json::Error,
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("Failed to read Snowflake response text"))]
    ResponseText {
        source: reqwest::Error,
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("Snowflake responded with error status: {status}, message: {message}"))]
    ResponseStatus {
        status: reqwest::StatusCode,
        message: String,
        #[snafu(implicit)]
        location: Location,
    },
}

/// Single wording source for "master token can never be renewed", shared by
/// [`RestError::MasterTokenTerminal`]'s `Display`, the `ApiError` layer's
/// equivalent variant, and the proto `AuthenticationError.detail` text.
/// `client_api.py`'s `_append_detail` dedupes by exact substring, so any
/// paraphrase between these sites prints the GS code multiple times in the
/// user-visible message. `code` is `None` only for a client-side-predicted
/// expiry with no server round-trip — never fabricate a code for that case.
pub(crate) fn master_token_terminal_detail(code: Option<i32>) -> String {
    match code {
        Some(code) => {
            format!(
                "Master token can never be renewed - full re-authentication required (GS code {code})"
            )
        }
        None => "Master token can never be renewed - full re-authentication required".to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::rest_parameters::test_fixtures::test_client_info;
    use crate::token_cache::{
        CacheKey, TokenCache, TokenCacheError, TokenType, build_cache_key, normalize_identifier,
        normalize_url,
    };
    use std::collections::HashMap;
    use std::sync::Mutex;

    #[test]
    fn is_reauthentication_required_covers_id_token_and_oauth_codes() {
        let browser = LoginMethod::ExternalBrowser {
            username: "testuser".to_string(),
            authentication_timeout_secs: 120,
            client_store_temporary_credential: true,
        };
        assert!(is_reauthentication_required(
            ID_TOKEN_INVALID_LOGIN_REQUEST,
            &browser
        ));

        let oauth_access = LoginMethod::OAuthAccessToken {
            username: "testuser".to_string(),
            token: "token".into(),
        };
        for code in [OAUTH_ACCESS_TOKEN_INVALID, OAUTH_ACCESS_TOKEN_EXPIRED] {
            // OAuth v1 (raw access token, caller-supplied) must stay excluded:
            // legacy's positive `isinstance(auth_instance, AuthByOAuthBase)`
            // gate excludes it too.
            assert!(!is_reauthentication_required(code, &oauth_access));
        }
    }

    #[test]
    fn is_reauthentication_required_excludes_ordinary_login_failures() {
        let browser = LoginMethod::ExternalBrowser {
            username: "testuser".to_string(),
            authentication_timeout_secs: 120,
            client_store_temporary_credential: true,
        };
        assert!(!is_reauthentication_required(390100, &browser));
    }

    #[test]
    fn is_reauthentication_required_excludes_mfa_even_for_id_token_code() {
        // Cache-invalidation (evict + replay) is not reauthentication: the
        // driver reacquires nothing, the user must satisfy the second factor
        // again. Legacy's `AuthByUsrPwdMfa.reauthenticate()` returns
        // `{"success": False}` — no self-driven recovery.
        let mfa = LoginMethod::UserPasswordMfa {
            username: "testuser".to_string(),
            password: "testpass".into(),
            passcode_in_password: false,
            passcode: None,
            client_store_temporary_credential: true,
        };
        assert!(!is_reauthentication_required(
            ID_TOKEN_INVALID_LOGIN_REQUEST,
            &mfa
        ));
    }

    struct StubTokenCache {
        store: Mutex<HashMap<String, String>>,
    }

    impl StubTokenCache {
        fn new() -> Self {
            Self {
                store: Mutex::new(HashMap::new()),
            }
        }

        /// Inserts a token keyed by the same `CacheKey` that `try_get_cached_token` and
        /// friends derive from `(server_url, username, role, token_type)`. `server_url`
        /// is a full URL (e.g. `"https://host.example.com"`) that is passed directly into
        /// `normalize_url`, matching how production helpers pass the full server URL.
        /// MFA and ID token flows use `idp: String::new()` per spec.
        fn with_token(
            server_url: &str,
            username: &str,
            role: &str,
            token_type: TokenType,
            value: &str,
        ) -> Self {
            let cache = Self::new();
            let key = CacheKey {
                token_type,
                idp: String::new(),
                snowflake: normalize_url(server_url),
                username: normalize_identifier(username),
                role: normalize_identifier(role),
            };
            cache
                .add_token(&key, value)
                .expect("test: add_token should succeed");
            cache
        }
    }

    impl TokenCache for StubTokenCache {
        fn add_token(&self, key: &CacheKey, token_value: &str) -> Result<(), TokenCacheError> {
            self.store
                .lock()
                .expect("test: lock poisoned")
                .insert(build_cache_key(key), token_value.to_string());
            Ok(())
        }

        fn remove_token(&self, key: &CacheKey) -> Result<(), TokenCacheError> {
            self.store
                .lock()
                .expect("test: lock poisoned")
                .remove(&build_cache_key(key));
            Ok(())
        }

        fn get_token(&self, key: &CacheKey) -> Result<Option<String>, TokenCacheError> {
            Ok(self
                .store
                .lock()
                .expect("test: lock poisoned")
                .get(&build_cache_key(key))
                .cloned())
        }
    }

    fn key_for(server_url: &str, username: &str, role: &str, token_type: TokenType) -> CacheKey {
        CacheKey {
            token_type,
            idp: String::new(),
            snowflake: normalize_url(server_url),
            username: normalize_identifier(username),
            role: normalize_identifier(role),
        }
    }

    fn test_login_params() -> LoginParameters {
        LoginParameters {
            account_name: "testaccount".to_string(),
            login_method: LoginMethod::Password {
                username: "testuser".to_string(),
                password: "testpass".into(),
                passcode_in_password: false,
                passcode: None,
            },
            server_url: "https://testaccount.snowflakecomputing.com".to_string(),
            database: None,
            schema: None,
            warehouse: None,
            role: None,
            secondary_roles: None,
            client_info: test_client_info(),
            session_parameters: None,
            spcs_token: None,
            disable_parallel_user_prompt: false,
        }
    }

    mod token_cache_helpers_tests {
        use super::*;

        async fn assert_get_store_remove_for(token_type: TokenType) {
            use std::sync::Arc;
            const SERVER: &str = "https://host.example.com";

            // try_get: returns cached token on hit
            let cache: Arc<dyn TokenCache> = Arc::new(StubTokenCache::with_token(
                SERVER, "alice", "", token_type, "tok_val",
            ));
            let result =
                try_get_cached_token(SERVER, "alice", "", token_type, Some(cache.clone())).await;
            assert_eq!(result.unwrap().reveal(), "tok_val");

            // try_get: returns None on cache miss
            let empty: Arc<dyn TokenCache> = Arc::new(StubTokenCache::new());
            assert!(
                try_get_cached_token(SERVER, "alice", "", token_type, Some(empty.clone()))
                    .await
                    .is_none()
            );

            // try_get: returns None when no cache provided
            assert!(
                try_get_cached_token(SERVER, "alice", "", token_type, None)
                    .await
                    .is_none()
            );

            // try_get: returns None for invalid URL
            assert!(
                try_get_cached_token("not-a-url", "alice", "", token_type, Some(empty.clone()))
                    .await
                    .is_none()
            );

            // try_get: returns None for empty cached value
            let empty_val: Arc<dyn TokenCache> = Arc::new(StubTokenCache::with_token(
                SERVER, "alice", "", token_type, "",
            ));
            assert!(
                try_get_cached_token(SERVER, "alice", "", token_type, Some(empty_val))
                    .await
                    .is_none()
            );

            // store + get round-trip
            let cache = Arc::new(StubTokenCache::new());
            store_token_in_cache(
                SERVER,
                "alice",
                "",
                token_type,
                "new_tok",
                Some(cache.clone() as Arc<dyn TokenCache>),
            )
            .await;
            let stored = cache
                .get_token(&key_for(SERVER, "alice", "", token_type))
                .unwrap();
            assert_eq!(stored.as_deref(), Some("new_tok"));

            // store: no panic when no cache
            store_token_in_cache(SERVER, "alice", "", token_type, "tok", None).await;

            // store: no panic for invalid URL
            store_token_in_cache(
                "not-a-url",
                "alice",
                "",
                token_type,
                "tok",
                Some(Arc::new(StubTokenCache::new()) as Arc<dyn TokenCache>),
            )
            .await;

            // remove evicts token
            let cache = Arc::new(StubTokenCache::with_token(
                SERVER,
                "alice",
                "",
                token_type,
                "to_remove",
            ));
            remove_token_from_cache(
                SERVER,
                "alice",
                "",
                token_type,
                Some(cache.clone() as Arc<dyn TokenCache>),
            )
            .await;
            assert!(
                cache
                    .get_token(&key_for(SERVER, "alice", "", token_type))
                    .unwrap()
                    .is_none()
            );

            // remove: no panic when no cache
            remove_token_from_cache(SERVER, "alice", "", token_type, None).await;

            // remove: no panic for invalid URL
            remove_token_from_cache(
                "not-a-url",
                "alice",
                "",
                token_type,
                Some(Arc::new(StubTokenCache::new()) as Arc<dyn TokenCache>),
            )
            .await;
        }

        #[tokio::test]
        async fn mfa_token_cache_operations() {
            assert_get_store_remove_for(TokenType::MfaToken).await;
        }

        #[tokio::test]
        async fn id_token_cache_operations() {
            assert_get_store_remove_for(TokenType::IdToken).await;
        }
    }

    mod into_query_result_tests {
        use super::*;
        use serde_json::json;

        fn response_from_json(value: serde_json::Value) -> query_response::Response {
            serde_json::from_value(value).expect("valid response JSON")
        }

        #[test]
        fn success_returns_response_unchanged() {
            let resp = response_from_json(json!({
                "success": true,
                "data": {
                    "rowset": null,
                    "rowsetBase64": null
                }
            }));

            match into_query_result(resp, &QueryIds::default()) {
                Ok(r) => assert!(r.success),
                Err(e) => panic!("expected Ok, got {:?}", e),
            }
        }

        #[test]
        fn failure_returns_query_failed_with_all_fields() {
            let resp = response_from_json(json!({
                "success": false,
                "message": "SQL compilation error",
                "code": "1003",
                "data": {
                    "rowset": null,
                    "rowsetBase64": null,
                    "sqlState": "42000"
                }
            }));

            let request_id = Uuid::new_v4();
            match into_query_result(
                resp,
                &QueryIds {
                    request_id: Some(request_id),
                    query_id: Some("01abc-def-12345".to_owned()),
                },
            ) {
                Err(RestError::QueryFailed {
                    message,
                    code,
                    sql_state,
                    ids,
                    ..
                }) => {
                    assert_eq!(message, "SQL compilation error");
                    assert_eq!(code, Some(1003));
                    assert_eq!(sql_state, Some("42000".to_owned()));
                    assert_eq!(ids.query_id, Some("01abc-def-12345".to_owned()));
                    assert_eq!(ids.request_id, Some(request_id));
                }
                Err(other) => panic!("expected QueryFailed, got {:?}", other),
                Ok(_) => panic!("expected Err, got Ok"),
            }
        }

        #[test]
        fn failure_with_missing_optional_fields() {
            let resp = response_from_json(json!({
                "success": false,
                "data": {
                    "rowset": null,
                    "rowsetBase64": null
                }
            }));

            match into_query_result(resp, &QueryIds::default()) {
                Err(RestError::QueryFailed {
                    message,
                    code,
                    sql_state,
                    ids,
                    ..
                }) => {
                    assert_eq!(message, "Unknown error");
                    assert_eq!(code, None);
                    assert_eq!(sql_state, None);
                    assert_eq!(ids.query_id, None);
                    assert_eq!(ids.request_id, None);
                }
                Err(other) => panic!("expected QueryFailed, got {:?}", other),
                Ok(_) => panic!("expected Err, got Ok"),
            }
        }
    }

    #[test]
    fn deserialize_query_status_success_response() {
        let json = r#"{
            "success": true,
            "data": {
                "queries": [{
                    "status": "SUCCESS",
                    "errorCode": 0,
                    "errorMessage": "No error reported"
                }]
            }
        }"#;
        let response: QueryStatusResponse = serde_json::from_str(json).unwrap();
        assert!(response.success);
        let data = response.data.unwrap();
        assert_eq!(data.queries.len(), 1);
        assert_eq!(data.queries[0].status, "SUCCESS");
        assert_eq!(data.queries[0].error_code.as_deref(), Some("0"));
        assert_eq!(
            data.queries[0].error_message.as_deref(),
            Some("No error reported")
        );
    }

    #[test]
    fn deserialize_query_status_running_response() {
        let json = r#"{
            "success": true,
            "data": {
                "queries": [{
                    "status": "RUNNING",
                    "errorCode": 0,
                    "errorMessage": ""
                }]
            }
        }"#;
        let response: QueryStatusResponse = serde_json::from_str(json).unwrap();
        assert!(response.success);
        assert_eq!(response.data.unwrap().queries[0].status, "RUNNING");
    }

    #[test]
    fn deserialize_query_status_error_response_with_int_code() {
        let json = r#"{
            "success": true,
            "data": {
                "queries": [{
                    "status": "FAILED_WITH_ERROR",
                    "errorCode": 2003,
                    "errorMessage": "SQL compilation error:\nObject 'NONEXISTENTTABLE' does not exist or not authorized."
                }]
            }
        }"#;
        let response: QueryStatusResponse = serde_json::from_str(json).unwrap();
        assert!(response.success);
        let data = response.data.unwrap();
        assert_eq!(data.queries[0].status, "FAILED_WITH_ERROR");
        assert_eq!(data.queries[0].error_code.as_deref(), Some("2003"));
        assert!(
            data.queries[0]
                .error_message
                .as_ref()
                .unwrap()
                .contains("NONEXISTENTTABLE")
        );
    }

    #[test]
    fn deserialize_query_status_error_response_with_string_code() {
        let json = r#"{
            "success": true,
            "data": {
                "queries": [{
                    "status": "FAILED_WITH_ERROR",
                    "errorCode": "002003",
                    "errorMessage": "SQL compilation error"
                }]
            }
        }"#;
        let response: QueryStatusResponse = serde_json::from_str(json).unwrap();
        assert!(response.success);
        let data = response.data.unwrap();
        assert_eq!(data.queries[0].status, "FAILED_WITH_ERROR");
        assert_eq!(data.queries[0].error_code.as_deref(), Some("002003"));
    }

    #[test]
    fn deserialize_query_status_missing_optional_fields() {
        let json = r#"{
            "success": true,
            "data": {
                "queries": [{
                    "status": "QUEUED"
                }]
            }
        }"#;
        let response: QueryStatusResponse = serde_json::from_str(json).unwrap();
        assert!(response.success);
        let data = response.data.unwrap();
        assert_eq!(data.queries[0].status, "QUEUED");
        assert_eq!(data.queries[0].error_code, None);
        assert_eq!(data.queries[0].error_message, None);
    }

    #[test]
    fn deserialize_query_status_server_error_response() {
        let json = r#"{
            "success": false,
            "message": "Query not found",
            "code": "000707",
            "data": {
                "queries": []
            }
        }"#;
        let response: QueryStatusResponse = serde_json::from_str(json).unwrap();
        assert!(!response.success);
        assert_eq!(response.message.as_deref(), Some("Query not found"));
        assert_eq!(response.code.as_deref(), Some("000707"));
    }

    #[test]
    fn deserialize_query_status_error_without_data() {
        let json = r#"{
            "success": false,
            "message": "Unauthorized",
            "code": "000401"
        }"#;
        let response: QueryStatusResponse = serde_json::from_str(json).unwrap();
        assert!(!response.success);
        assert!(response.data.is_none());
        assert_eq!(response.message.as_deref(), Some("Unauthorized"));
    }

    #[test]
    fn password_auth_payload_does_not_include_authenticator() {
        let login_params = test_login_params();
        let rt = tokio::runtime::Runtime::new().unwrap();
        let client = reqwest::Client::new();
        let data = rt
            .block_on(auth_request_data(
                &client,
                &login_params,
                None,
                None,
                None,
                &RetryPolicy::default(),
            ))
            .unwrap();

        assert_eq!(data.login_name.as_deref(), Some("testuser"));
        assert_eq!(data.password.as_ref().unwrap().reveal(), "testpass");
        assert!(
            data.authenticator.is_none(),
            "Password auth should NOT include AUTHENTICATOR field (matching old driver behavior)"
        );
    }

    #[test]
    fn auth_request_uses_application_for_client_environment_application() {
        // CLIENT_APP_ID → driver identity (``client_app_id``).
        // CLIENT_ENVIRONMENT.APPLICATION → user-facing app name
        // (``application``). They must remain independent.
        let login_params = LoginParameters {
            client_info: ClientInfo {
                client_app_id: "PythonConnector".to_string(),
                application: "SNOWCLI.STAGE.COPY".to_string(),
                ..test_client_info()
            },
            ..test_login_params()
        };
        let rt = tokio::runtime::Runtime::new().unwrap();
        let client = reqwest::Client::new();
        let data = rt
            .block_on(auth_request_data(
                &client,
                &login_params,
                None,
                None,
                None,
                &RetryPolicy::default(),
            ))
            .unwrap();

        assert_eq!(data.client_app_id, "PythonConnector");
        assert_eq!(data.client_environment.application, "SNOWCLI.STAGE.COPY");
    }

    #[test]
    fn pat_auth_payload_includes_authenticator() {
        let login_params = LoginParameters {
            login_method: LoginMethod::Pat {
                username: "testuser".to_string(),
                token: "pat_secret".into(),
            },
            ..test_login_params()
        };
        let rt = tokio::runtime::Runtime::new().unwrap();
        let client = reqwest::Client::new();
        let data = rt
            .block_on(auth_request_data(
                &client,
                &login_params,
                None,
                None,
                None,
                &RetryPolicy::default(),
            ))
            .unwrap();

        assert_eq!(data.login_name.as_deref(), Some("testuser"));
        assert_eq!(data.token.as_ref().unwrap().reveal(), "pat_secret");
        assert_eq!(
            data.authenticator.as_deref(),
            Some("PROGRAMMATIC_ACCESS_TOKEN")
        );
    }

    #[test]
    fn pat_auth_without_user_omits_login_name() {
        let login_params = LoginParameters {
            login_method: LoginMethod::Pat {
                username: "".to_string(),
                token: "pat_secret".into(),
            },
            ..test_login_params()
        };
        let rt = tokio::runtime::Runtime::new().unwrap();
        let client = reqwest::Client::new();
        let data = rt
            .block_on(auth_request_data(
                &client,
                &login_params,
                None,
                None,
                None,
                &RetryPolicy::default(),
            ))
            .unwrap();

        assert_eq!(
            data.login_name, None,
            "LOGIN_NAME must be absent when user is empty"
        );
        assert_eq!(data.token.as_ref().unwrap().reveal(), "pat_secret");
        assert_eq!(
            data.authenticator.as_deref(),
            Some("PROGRAMMATIC_ACCESS_TOKEN")
        );
    }

    #[test]
    fn secondary_roles_is_uppercased_in_auth_body() {
        let rt = tokio::runtime::Runtime::new().unwrap();
        let client = reqwest::Client::new();
        for (input, expected) in [
            ("ALL", "ALL"),
            ("all", "ALL"),
            ("All", "ALL"),
            ("NONE", "NONE"),
            ("none", "NONE"),
            ("None", "NONE"),
            ("DEFAULT", "DEFAULT"),
            ("default", "DEFAULT"),
        ] {
            let login_params = LoginParameters {
                secondary_roles: Some(input.to_string()),
                ..test_login_params()
            };
            let data = rt
                .block_on(auth_request_data(
                    &client,
                    &login_params,
                    None,
                    None,
                    None,
                    &RetryPolicy::default(),
                ))
                .unwrap();
            assert_eq!(
                data.secondary_roles.as_deref(),
                Some(expected),
                "input {input:?} should uppercase to {expected:?}"
            );
        }
    }

    #[test]
    fn secondary_roles_omitted_when_not_specified() {
        let login_params = test_login_params();
        let rt = tokio::runtime::Runtime::new().unwrap();
        let client = reqwest::Client::new();
        let data = rt
            .block_on(auth_request_data(
                &client,
                &login_params,
                None,
                None,
                None,
                &RetryPolicy::default(),
            ))
            .unwrap();

        assert_eq!(data.secondary_roles, None);
    }

    #[test]
    fn secondary_roles_omitted_when_empty_string() {
        let login_params = LoginParameters {
            secondary_roles: Some(String::new()),
            ..test_login_params()
        };
        let rt = tokio::runtime::Runtime::new().unwrap();
        let client = reqwest::Client::new();
        let data = rt
            .block_on(auth_request_data(
                &client,
                &login_params,
                None,
                None,
                None,
                &RetryPolicy::default(),
            ))
            .unwrap();

        assert_eq!(data.secondary_roles, None);
    }

    mod send_login_request_retry_tests {
        use super::*;
        use std::sync::Arc;
        use std::sync::atomic::{AtomicU32, Ordering};
        use wiremock::matchers::{method, path_regex};
        use wiremock::{Mock, MockServer, Request, ResponseTemplate};

        #[tokio::test]
        async fn retries_on_503_then_succeeds() {
            let server = MockServer::start().await;
            let attempt = Arc::new(AtomicU32::new(0));

            let attempt_clone = attempt.clone();
            Mock::given(method("POST"))
                .and(path_regex(r"/session/v1/login-request"))
                .respond_with(move |_: &Request| {
                    let n = attempt_clone.fetch_add(1, Ordering::SeqCst);
                    if n < 2 {
                        ResponseTemplate::new(503).set_body_string("Service Unavailable")
                    } else {
                        ResponseTemplate::new(200).set_body_json(serde_json::json!({
                            "success": true,
                            "data": {
                                "token": "mock_token",
                                "masterToken": "mock_master_token",
                                "sessionId": 12345
                            }
                        }))
                    }
                })
                .expect(3)
                .mount(&server)
                .await;

            let client = reqwest::Client::new();
            let params = LoginParameters {
                server_url: server.uri(),
                ..test_login_params()
            };
            let auth_req = AuthRequest {
                data: AuthRequestData {
                    account_name: "testaccount".to_string(),
                    login_name: Some("testuser".to_string()),
                    password: Some("testpass".into()),
                    ..Default::default()
                },
            };

            let result =
                send_login_request(&client, &params, &auth_req, &RetryPolicy::default()).await;

            assert!(result.is_ok(), "Expected retry to succeed, got: {result:?}");
            assert_eq!(
                attempt.load(Ordering::SeqCst),
                3,
                "Expected exactly 3 attempts (2 failures + 1 success), got {}",
                attempt.load(Ordering::SeqCst)
            );
        }
    }

    mod snowflake_abort_query_tests {
        use super::*;
        use wiremock::matchers::{method, path_regex};
        use wiremock::{Mock, MockServer, ResponseTemplate};

        fn query_parameters(server_url: String) -> QueryParameters {
            QueryParameters {
                server_url,
                client_info: test_client_info(),
                log_max_query_length: 1024,
                log_query_text: false,
                log_query_parameters: false,
            }
        }

        #[tokio::test]
        async fn success_true_returns_ok_aborted() {
            let server = MockServer::start().await;
            Mock::given(method("POST"))
                .and(path_regex(r"/queries/.*/abort-request"))
                .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                    "success": true,
                })))
                .expect(1)
                .mount(&server)
                .await;

            let result = snowflake_abort_query(
                &reqwest::Client::new(),
                &query_parameters(server.uri()),
                "mock_session_token",
                "01abcdef-0000-0000-0000-000000000000",
            )
            .await;

            assert!(
                matches!(result, Ok(AbortOutcome::Aborted)),
                "expected Ok(Aborted), got {result:?}"
            );
        }

        /// Server declining the abort (query not running — e.g. already
        /// completed, code `000605`) is a normal outcome — `Ok(NotRunning)`,
        /// not an error, and no retry.
        #[tokio::test]
        async fn success_false_returns_ok_not_running_without_retry() {
            let server = MockServer::start().await;
            Mock::given(method("POST"))
                .and(path_regex(r"/queries/.*/abort-request"))
                .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                    "success": false,
                    "code": "000605",
                    "message": "Query is not currently executing",
                })))
                .expect(1)
                .mount(&server)
                .await;

            let result = snowflake_abort_query(
                &reqwest::Client::new(),
                &query_parameters(server.uri()),
                "mock_session_token",
                "01abcdef-0000-0000-0000-000000000000",
            )
            .await;

            assert!(
                matches!(result, Ok(AbortOutcome::NotRunning)),
                "expected Ok(NotRunning), got {result:?}"
            );
        }

        #[tokio::test]
        async fn non_2xx_response_returns_err() {
            let server = MockServer::start().await;
            Mock::given(method("POST"))
                .and(path_regex(r"/queries/.*/abort-request"))
                .respond_with(ResponseTemplate::new(500).set_body_string("Internal Server Error"))
                .expect(1)
                .mount(&server)
                .await;

            let result = snowflake_abort_query(
                &reqwest::Client::new(),
                &query_parameters(server.uri()),
                "mock_session_token",
                "01abcdef-0000-0000-0000-000000000000",
            )
            .await;

            assert!(
                matches!(result, Err(RestError::InvalidSnowflakeResponse { .. })),
                "expected InvalidSnowflakeResponse error, got {result:?}"
            );
        }

        /// `success:false` with body code `390112` (session token expired) still
        /// routes through the existing `SessionExpired` mapping in
        /// `read_response_json`, surfaced here as `RestError::SessionExpired`.
        #[tokio::test]
        async fn session_expired_code_returns_err() {
            let server = MockServer::start().await;
            Mock::given(method("POST"))
                .and(path_regex(r"/queries/.*/abort-request"))
                .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                    "success": false,
                    "code": "390112",
                    "message": "Session token expired",
                })))
                .expect(1)
                .mount(&server)
                .await;

            let result = snowflake_abort_query(
                &reqwest::Client::new(),
                &query_parameters(server.uri()),
                "mock_session_token",
                "01abcdef-0000-0000-0000-000000000000",
            )
            .await;

            assert!(
                matches!(result, Err(RestError::SessionExpired { .. })),
                "expected SessionExpired, got {result:?}"
            );
        }
    }

    /// Mirrors [`snowflake_abort_query_tests`]: the `success`-envelope →
    /// [`AbortOutcome`] mapping for the requestId-based cancel endpoint. The
    /// outbound body shape (`{sqlText, requestId}`) and the cross-thread
    /// orchestration are covered by
    /// `tests/integration/query/operation_cancellation.rs`.
    mod snowflake_cancel_query_tests {
        use super::*;
        use wiremock::matchers::{method, path};
        use wiremock::{Mock, MockServer, ResponseTemplate};

        fn query_parameters(server_url: String) -> QueryParameters {
            QueryParameters {
                server_url,
                client_info: test_client_info(),
                log_max_query_length: 1024,
                log_query_text: false,
                log_query_parameters: false,
            }
        }

        #[tokio::test]
        async fn success_true_returns_ok_aborted() {
            let server = MockServer::start().await;
            Mock::given(method("POST"))
                .and(path("/queries/v1/abort-request"))
                .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                    "success": true,
                })))
                .expect(1)
                .mount(&server)
                .await;

            let result = snowflake_cancel_query(
                &reqwest::Client::new(),
                &query_parameters(server.uri()),
                "mock_session_token",
                "running-request-id",
                "SELECT 1",
            )
            .await;

            assert!(
                matches!(result, Ok(AbortOutcome::Aborted)),
                "expected Ok(Aborted), got {result:?}"
            );
        }

        /// Server declining the cancel (query not running — e.g. already
        /// completed, code `000605`) is a normal outcome — `Ok(NotRunning)`,
        /// not an error.
        #[tokio::test]
        async fn success_false_returns_ok_not_running() {
            let server = MockServer::start().await;
            Mock::given(method("POST"))
                .and(path("/queries/v1/abort-request"))
                .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                    "success": false,
                    "code": "000605",
                    "message": "Query is not currently executing",
                })))
                .expect(1)
                .mount(&server)
                .await;

            let result = snowflake_cancel_query(
                &reqwest::Client::new(),
                &query_parameters(server.uri()),
                "mock_session_token",
                "running-request-id",
                "SELECT 1",
            )
            .await;

            assert!(
                matches!(result, Ok(AbortOutcome::NotRunning)),
                "expected Ok(NotRunning), got {result:?}"
            );
        }

        #[tokio::test]
        async fn non_2xx_response_returns_err() {
            let server = MockServer::start().await;
            Mock::given(method("POST"))
                .and(path("/queries/v1/abort-request"))
                .respond_with(ResponseTemplate::new(500).set_body_string("Internal Server Error"))
                .expect(1)
                .mount(&server)
                .await;

            let result = snowflake_cancel_query(
                &reqwest::Client::new(),
                &query_parameters(server.uri()),
                "mock_session_token",
                "running-request-id",
                "SELECT 1",
            )
            .await;

            assert!(
                matches!(result, Err(RestError::InvalidSnowflakeResponse { .. })),
                "expected InvalidSnowflakeResponse error, got {result:?}"
            );
        }

        /// `success:false` with body code `390112` (session token expired) still
        /// routes through the existing `SessionExpired` mapping in
        /// `read_response_json`, so `with_valid_session` can renew-and-retry.
        #[tokio::test]
        async fn session_expired_code_returns_err() {
            let server = MockServer::start().await;
            Mock::given(method("POST"))
                .and(path("/queries/v1/abort-request"))
                .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                    "success": false,
                    "code": "390112",
                    "message": "Session token expired",
                })))
                .expect(1)
                .mount(&server)
                .await;

            let result = snowflake_cancel_query(
                &reqwest::Client::new(),
                &query_parameters(server.uri()),
                "mock_session_token",
                "running-request-id",
                "SELECT 1",
            )
            .await;

            assert!(
                matches!(result, Err(RestError::SessionExpired { .. })),
                "expected SessionExpired, got {result:?}"
            );
        }
    }

    mod user_agent_tests {
        use super::*;

        const ARCH: &str = std::env::consts::ARCH;

        #[test]
        fn user_agent_without_runtime_info() {
            let info = ClientInfo {
                client_app_id: "MyApp".to_string(),
                version: "1.0.0".to_string(),
                os: "Linux".to_string(),
                ..test_client_info()
            };
            assert_eq!(user_agent(&info), format!("MyApp/1.0.0 (Linux-{ARCH})"));
        }

        #[test]
        fn user_agent_with_runtime_info() {
            let info = ClientInfo {
                client_app_id: "PythonConnector".to_string(),
                version: "3.15.0".to_string(),
                os: "Darwin".to_string(),
                runtime_name: Some("CPython".to_string()),
                runtime_version: Some("3.11.6".to_string()),
                ..test_client_info()
            };
            assert_eq!(
                user_agent(&info),
                format!("PythonConnector/3.15.0 (Darwin-{ARCH}) CPython/3.11.6")
            );
        }

        #[test]
        fn user_agent_with_only_runtime_name_no_version() {
            let info = ClientInfo {
                runtime_name: Some("CPython".to_string()),
                runtime_version: None,
                ..test_client_info()
            };
            // Only appended when both name and version are present
            assert!(!user_agent(&info).contains("CPython"));
        }

        #[test]
        fn user_agent_sanitizes_spaces_in_runtime_name() {
            let info = ClientInfo {
                client_app_id: "JDBC".to_string(),
                version: "4.0.2".to_string(),
                os: "Linux".to_string(),
                runtime_name: Some("OpenJDK 64-Bit Server VM".to_string()),
                runtime_version: Some("17.0.6".to_string()),
                ..test_client_info()
            };
            assert_eq!(
                user_agent(&info),
                format!("JDBC/4.0.2 (Linux-{ARCH}) OpenJDK_64-Bit_Server_VM/17.0.6")
            );
        }
    }

    mod strip_version_suffix_tests {
        use super::*;

        #[test]
        fn clean_version_unchanged() {
            assert_eq!(strip_version_suffix("5.0.0"), "5.0.0");
        }

        #[test]
        fn dev_suffix_stripped() {
            assert_eq!(strip_version_suffix("5.0.0dev"), "5.0.0");
        }

        #[test]
        fn rc_suffix_stripped() {
            assert_eq!(strip_version_suffix("3.12.1rc2"), "3.12.1");
        }

        #[test]
        fn four_segment_preserved() {
            assert_eq!(strip_version_suffix("2.21.8.1"), "2.21.8.1");
        }

        #[test]
        fn dotted_dev_segment_dropped_not_zeroed() {
            assert_eq!(strip_version_suffix("5.0.0.dev0"), "5.0.0");
        }

        #[test]
        fn dotted_post_segment_dropped_not_zeroed() {
            assert_eq!(strip_version_suffix("5.0.0.post1"), "5.0.0");
        }

        #[test]
        fn hyphenated_prerelease_stripped() {
            assert_eq!(strip_version_suffix("4.0.0-rc1"), "4.0.0");
        }

        #[test]
        fn fully_non_numeric_falls_back_to_zero() {
            assert_eq!(strip_version_suffix("dev"), "0");
        }
    }

    mod query_log_fields_tests {
        use super::*;
        use serde_json::value::RawValue;

        fn make_params(log_max_query_length: usize, text: bool, params: bool) -> QueryParameters {
            QueryParameters {
                server_url: "https://example.test".into(),
                client_info: test_client_info(),
                log_max_query_length,
                log_query_text: text,
                log_query_parameters: params,
            }
        }

        #[test]
        fn flags_off_returns_none_none() {
            let params = make_params(80, false, false);
            let input = QueryInput::new("SELECT 1");
            assert_eq!(query_log_fields(&params, &input), (None, None));
        }

        #[test]
        fn bindings_flag_without_text_flag_is_noop() {
            let params = make_params(80, false, true);
            let input = QueryInput::new("SELECT 1");
            assert_eq!(query_log_fields(&params, &input), (None, None));
        }

        #[test]
        fn text_only_returns_full_sql_when_within_limit() {
            let params = make_params(80, true, false);
            let input = QueryInput::new("SELECT 1");
            let (sql, bindings) = query_log_fields(&params, &input);
            assert_eq!(sql.as_deref(), Some("SELECT 1"));
            assert!(bindings.is_none());
        }

        #[test]
        fn text_only_truncates_to_log_max_query_length() {
            let params = make_params(6, true, false);
            let input = QueryInput::new("SELECT * FROM t WHERE x = 1");
            let (sql, bindings) = query_log_fields(&params, &input);
            assert_eq!(sql.as_deref(), Some("SELECT"));
            assert!(bindings.is_none());
        }

        #[test]
        fn text_only_truncates_at_char_boundary_for_multibyte() {
            // "héllo" — 'é' is 2 bytes in UTF-8 but a single `char`. With limit
            // 3 we expect "hél" (3 chars), not bytes.
            let params = make_params(3, true, false);
            let input = QueryInput::new("héllo world");
            let (sql, _) = query_log_fields(&params, &input);
            assert_eq!(sql.as_deref(), Some("hél"));
        }

        #[test]
        fn text_and_params_includes_bindings_json() {
            let params = make_params(80, true, true);
            let raw: Box<RawValue> = serde_json::value::to_raw_value(&serde_json::json!({
                "1": {"type": "TEXT", "value": "hello"}
            }))
            .unwrap();
            let mut input = QueryInput::new("SELECT ?");
            input.bindings = Some(&raw);
            let (sql, bindings) = query_log_fields(&params, &input);
            assert_eq!(sql.as_deref(), Some("SELECT ?"));
            assert!(bindings.is_some());
            let bindings = bindings.unwrap();
            assert!(
                bindings.contains("hello"),
                "expected bindings JSON to contain the value, got {bindings}"
            );
        }

        #[test]
        fn text_and_params_truncates_bindings_to_log_max_query_length() {
            let params = make_params(8, true, true);
            let raw: Box<RawValue> = serde_json::value::to_raw_value(&serde_json::json!({
                "1": {"type": "TEXT", "value": "abcdefghijklmnop"}
            }))
            .unwrap();
            let mut input = QueryInput::new("SELECT ?");
            input.bindings = Some(&raw);
            let (sql, bindings) = query_log_fields(&params, &input);
            assert_eq!(sql.as_deref().map(str::len), Some(8));
            let bindings = bindings.expect("bindings field should be present");
            assert_eq!(bindings.chars().count(), 8);
            assert!(
                raw.get().starts_with(&bindings),
                "truncated bindings should be the prefix of the raw JSON: {bindings}"
            );
        }

        #[test]
        fn text_and_params_returns_empty_string_when_no_bindings() {
            let params = make_params(80, true, true);
            let input = QueryInput::new("SELECT 1");
            let (sql, bindings) = query_log_fields(&params, &input);
            assert_eq!(sql.as_deref(), Some("SELECT 1"));
            assert_eq!(bindings.as_deref(), Some(""));
        }
    }

    mod execute_sync_query_retry_tests {
        use super::*;
        use std::sync::Arc;
        use std::sync::atomic::{AtomicU32, Ordering};
        use wiremock::matchers::{method, path_regex};
        use wiremock::{Mock, MockServer, Request, ResponseTemplate};

        #[tokio::test]
        async fn retries_on_503_then_succeeds_and_sets_retry_flag_on_replays() {
            let server = MockServer::start().await;
            let attempt = Arc::new(AtomicU32::new(0));
            let captured_urls: Arc<std::sync::Mutex<Vec<String>>> =
                Arc::new(std::sync::Mutex::new(Vec::new()));

            let attempt_clone = attempt.clone();
            let captured_clone = captured_urls.clone();
            Mock::given(method("POST"))
                .and(path_regex(r"/queries/v1/query-request"))
                .respond_with(move |req: &Request| {
                    captured_clone.lock().unwrap().push(req.url.to_string());
                    let n = attempt_clone.fetch_add(1, Ordering::SeqCst);
                    if n < 2 {
                        ResponseTemplate::new(503).set_body_string("Service Unavailable")
                    } else {
                        ResponseTemplate::new(200).set_body_json(serde_json::json!({
                            "success": true,
                            "data": {
                                "queryId": "01abcdef-0000-0000-0000-000000000000",
                            }
                        }))
                    }
                })
                .expect(3)
                .mount(&server)
                .await;

            let client = reqwest::Client::new();
            let query_parameters = QueryParameters {
                server_url: server.uri(),
                client_info: test_client_info(),
                log_max_query_length: 1024,
                log_query_text: false,
                log_query_parameters: false,
            };
            let query_input = QueryInput::new("SELECT 1");

            let retry_policy = RetryPolicy::default();
            let result = execute_sync_query(
                &client,
                &query_parameters,
                "mock_session_token",
                &query_input,
                uuid::Uuid::new_v4(),
                &retry_policy,
            )
            .await;

            if let Err(e) = &result {
                panic!("Expected retry to succeed, got error: {e:?}");
            }
            assert_eq!(
                attempt.load(Ordering::SeqCst),
                3,
                "Expected exactly 3 attempts (2 failures + 1 success)",
            );

            let urls = captured_urls.lock().unwrap();
            assert_eq!(urls.len(), 3, "Should have captured 3 request URLs");
            assert!(
                !urls[0].contains("retry=true"),
                "First attempt must not include retry=true (fresh request): {}",
                urls[0]
            );
            assert!(
                urls[1].contains("retry=true"),
                "Second attempt must include retry=true so the server dedupes: {}",
                urls[1]
            );
            assert!(
                urls[2].contains("retry=true"),
                "Third attempt must include retry=true so the server dedupes: {}",
                urls[2]
            );

            let request_ids: Vec<&str> = urls
                .iter()
                .filter_map(|u| {
                    u.split_once("requestId=")
                        .map(|(_, rest)| rest.split('&').next().unwrap_or(rest))
                })
                .collect();
            assert_eq!(request_ids.len(), 3);
            assert!(
                request_ids[0] == request_ids[1] && request_ids[1] == request_ids[2],
                "requestId must be stable across HTTP-level retries: {:?}",
                request_ids
            );
        }
    }

    /// 2xx response carrying `success:false, code:"390112"` must be surfaced as
    /// `SessionExpired` so the RefreshContext can refresh and retry — the only
    /// behavior this envelope refactor introduces beyond the existing HTTP 401 path.
    #[tokio::test]
    async fn read_response_json_maps_body_390112_to_session_expired() {
        use wiremock::matchers::{method, path};
        use wiremock::{Mock, MockServer, ResponseTemplate};

        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "success": false,
                "code": "390112",
                "message": "Session token expired",
            })))
            .mount(&server)
            .await;

        let response = reqwest::Client::new()
            .post(server.uri())
            .send()
            .await
            .expect("mock request sends");

        let result = read_response_json::<serde_json::Value>(response).await;
        assert!(
            matches!(result, Err(RestError::SessionExpired { .. })),
            "expected SessionExpired, got {result:?}"
        );
    }

    /// GS 390113/390114/390115 on a 2xx query response all mean the master
    /// token can never be renewed. Each must map to `MasterTokenTerminal`,
    /// carrying the real code — not a fabricated one.
    #[tokio::test]
    async fn read_response_json_maps_master_token_terminal_codes() {
        use wiremock::matchers::{method, path};
        use wiremock::{Mock, MockServer, ResponseTemplate};

        for code in [
            MASTER_TOKEN_NOT_FOUND,
            MASTER_TOKEN_EXPIRED,
            MASTER_TOKEN_INVALID,
        ] {
            let server = MockServer::start().await;
            Mock::given(method("POST"))
                .and(path("/"))
                .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                    "success": false,
                    "code": code.to_string(),
                    "message": "Master token is no longer valid",
                })))
                .mount(&server)
                .await;

            let response = reqwest::Client::new()
                .post(server.uri())
                .send()
                .await
                .expect("mock request sends");

            let result = read_response_json::<serde_json::Value>(response).await;
            match result {
                Err(RestError::MasterTokenTerminal { code: got, .. }) => {
                    assert_eq!(
                        got, code,
                        "must preserve the real GS code, not fabricate one"
                    );
                }
                other => panic!("expected MasterTokenTerminal for code {code}, got {other:?}"),
            }
        }
    }

    /// Non-2xx bodies can carry tokens in JSON `data`; the error Display must
    /// not embed raw body content — only status and a generic body-length hint.
    #[tokio::test]
    async fn read_response_json_error_omits_raw_body_from_display() {
        use wiremock::matchers::{method, path};
        use wiremock::{Mock, MockServer, ResponseTemplate};

        let secret_token = "super-secret-token-12345";
        let body = format!(
            r#"{{"success":false,"code":"390100","message":"Auth failed","data":{{"token":"{secret_token}"}}}}"#
        );
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/login"))
            .respond_with(ResponseTemplate::new(403).set_body_string(body.clone()))
            .mount(&server)
            .await;

        let response = reqwest::Client::new()
            .post(format!("{}/login", server.uri()))
            .send()
            .await
            .expect("mock request sends");

        let err = read_response_json::<serde_json::Value>(response)
            .await
            .expect_err("non-2xx should fail");
        let inner = std::error::Error::source(&err)
            .map(|s| s.to_string())
            .unwrap_or_default();
        assert!(
            inner.contains("403") && inner.contains(&format!("body length {}.", body.len())),
            "expected status and body length in inner display, outer={err}, inner={inner}"
        );
        let display = format!("{err}; {inner}");
        assert!(
            !display.contains(secret_token),
            "raw body must not appear in error display, got: {display}"
        );
        assert!(
            !display.contains("390100") && !display.contains("Auth failed"),
            "server error payload must not appear in display, got: {display}"
        );
    }

    /// Proves the WORKLOAD_IDENTITY host guard actually gates dispatch to the
    /// provider, rather than merely existing alongside it. Uses the OIDC
    /// provider (no network dependency) with a config that would fail with a
    /// *different*, provider-specific error (`MissingToken`) if it were ever
    /// reached. A disallowed `server_url` must short-circuit with the guard's
    /// `DisallowedHost` error and must never reach — let alone fail inside —
    /// `workload_identity::create_attestation`.
    mod workload_identity_host_guard_tests {
        use super::*;
        use crate::config::rest_parameters::{WifProvider, WorkloadIdentityConfig};

        /// Concatenates the `Display` message of `err` with every message in
        /// its `source()` chain. `AuthError::to_string()` only renders the
        /// outermost variant (e.g. "Workload Identity Federation attestation
        /// failed"), so assertions that need to see *which* inner error was
        /// produced must walk the chain, matching the pattern already used
        /// for root-cause extraction elsewhere in this crate (see
        /// `protobuf::apis::database_driver_v1::converter::extract_root_cause`).
        fn full_chain_message(err: &(dyn std::error::Error)) -> String {
            let mut messages = vec![err.to_string()];
            let mut current = err.source();
            while let Some(cause) = current {
                messages.push(cause.to_string());
                current = cause.source();
            }
            messages.join(" -> ")
        }

        fn wif_login_params(server_url: &str) -> LoginParameters {
            LoginParameters {
                login_method: LoginMethod::WorkloadIdentity(WorkloadIdentityConfig {
                    provider: WifProvider::Oidc,
                    entra_resource: None,
                    impersonation_path: Vec::new(),
                    // Deliberately absent: if dispatch ever reached the
                    // provider without the host guard short-circuiting first,
                    // `oidc::get_token` would fail with `MissingToken`
                    // instead — a distinct, observable error — proving the
                    // ambient-credential path was actually reached.
                    oidc_token: None,
                }),
                server_url: server_url.to_string(),
                ..test_login_params()
            }
        }

        #[tokio::test]
        async fn rejected_host_never_reaches_attestation_provider() {
            let params = wif_login_params("https://not-snowflake.example");

            let err = auth_request_data(
                &reqwest::Client::new(),
                &params,
                None,
                None,
                None,
                &RetryPolicy::default(),
            )
            .await
            .expect_err("disallowed WIF host must fail closed");

            let display = full_chain_message(&err);
            assert!(
                display.contains("Refusing to send a Workload Identity attestation")
                    && display.contains("not-snowflake.example"),
                "expected the host-guard's DisallowedHost error, got: {display}"
            );
            assert!(
                !display.contains("pre-acquired token"),
                "OIDC provider's MissingToken error leaked through — the guard did not \
                 short-circuit before create_attestation, got: {display}"
            );
        }

        /// Control: the same config against an *allowed* host does reach the
        /// provider (and fails there, with its own `MissingToken` error,
        /// since no OIDC token was supplied). This confirms the previous
        /// test's rejection is actually caused by the host guard and not by
        /// some unrelated failure that would occur regardless of host.
        #[tokio::test]
        async fn allowed_host_does_reach_attestation_provider() {
            let params = wif_login_params("https://acct.snowflakecomputing.com");

            let err = auth_request_data(
                &reqwest::Client::new(),
                &params,
                None,
                None,
                None,
                &RetryPolicy::default(),
            )
            .await
            .expect_err("missing OIDC token must fail");

            let display = full_chain_message(&err);
            assert!(
                display.contains("pre-acquired token"),
                "expected the OIDC provider's MissingToken error, got: {display}"
            );
            assert!(
                !display.contains("Refusing to send a Workload Identity attestation"),
                "allowed host must not trip the host guard, got: {display}"
            );
        }
    }
}
