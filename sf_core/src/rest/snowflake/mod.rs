#![allow(clippy::result_large_err)]
pub mod async_exec;
mod auth;
mod browser;
pub mod error;
mod external_browser;
pub mod heartbeat;
pub mod logout;
mod native_okta;
mod oauth;
pub mod prompt_lock;
mod workload_identity;
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
use crate::rest::snowflake::error::SfError;
use crate::rest::snowflake::external_browser::{
    DefaultBrowserOpener, external_browser_authenticate,
};
use crate::rest::snowflake::native_okta::fetch_native_okta_saml;
use crate::sensitive::SensitiveString;
use crate::tls::client::create_tls_client_with_proxy;
use crate::tls::error::TlsError;
use crate::token_cache::{CacheKey, TokenCache, TokenType, normalize_identifier, normalize_url};
use reqwest::{self, Method, StatusCode, header};
use serde_json;
use serde_json::value::RawValue;
use snafu::{Location, OptionExt, ResultExt, Snafu};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use tracing;
use url::Url;
use uuid::Uuid;

pub const STATEMENT_ASYNC_EXECUTION_OPTION: &str = "async_execution";
pub(crate) const QUERY_REQUEST_PATH: &str = "/queries/v1/query-request";
const TOKEN_REQUEST_PATH: &str = "/session/token-request";

/// Send an HTTP request with retry and return `(StatusCode, body_text)`.
///
/// Shared by `native_okta` and `external_browser` authentication flows.
async fn request_text_with_retry(
    build: impl Fn() -> reqwest::RequestBuilder,
    ctx: &HttpContext,
    policy: &RetryPolicy,
    cancel: tokio_util::sync::CancellationToken,
) -> Result<(StatusCode, String), HttpError> {
    execute_with_retry(
        build,
        ctx,
        policy,
        |resp| async move {
            let status = resp.status();
            let text = resp.text().await.context(TransportSnafu)?;
            Ok((status, text))
        },
        cancel,
    )
    .await
}

// ─── Snowflake GS protocol error codes ───────────────────────────────────────
/// GS error code returned when a session no longer exists on the server.
/// Logout callers treat this as success — the goal (an invalidated session) is achieved.
pub const SESSION_GONE: i32 = 390111;
/// GS error code returned when the session token has expired.
/// The caller must use the master token to obtain a fresh session token and retry.
pub const SESSION_TOKEN_EXPIRED: i32 = 390112;
/// GS error code returned when the master token has expired.
/// Full re-authentication is required; the session can never be renewed.
pub const MASTER_TOKEN_EXPIRED: i32 = 390114;
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

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum QueryExecutionMode {
    Blocking,
    Async,
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
/// remain enabled, so this helper removes alphabetic suffixes like `"dev"` or
/// `"rc1"` from each dot-separated segment while preserving existing numeric
/// segments. Examples: `"5.0.0dev"` → `"5.0.0"`, `"4.0.0"` → `"4.0.0"`,
/// `"2.21.8.1"` → `"2.21.8.1"`.
fn strip_version_suffix(version: &str) -> String {
    version
        .split('.')
        .map(|seg| {
            let numeric: String = seg.chars().take_while(|c| c.is_ascii_digit()).collect();
            if numeric.is_empty() {
                "0".to_owned()
            } else {
                numeric
            }
        })
        .collect::<Vec<_>>()
        .join(".")
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
        },
        ..Default::default()
    }
}

const EXT_AUTHN_ERROR_CODES: [i32; 8] = [
    390120, // EXT_AUTHN_DENIED
    390122, // EXT_AUTHN_NOT_ENROLLED
    390123, // EXT_AUTHN_LOCKED
    390126, // EXT_AUTHN_TIMEOUT
    390127, // EXT_AUTHN_INVALID
    390129, // EXT_AUTHN_EXCEPTION
    390132, // EXT_AUTHN_DUO_PUSH_DISABLED
    390195, // ID_TOKEN_INVALID
];

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

fn try_get_cached_token(
    server_url: &str,
    username: &str,
    role: &str,
    token_type: TokenType,
    token_cache: Option<&dyn TokenCache>,
) -> Option<SensitiveString> {
    let cache = token_cache?;
    let key = CacheKey {
        token_type,
        idp: normalize_url(server_url),
        snowflake: normalize_url(server_url),
        username: normalize_identifier(username),
        role: normalize_identifier(role),
    };
    match cache.get_token(&key) {
        Ok(Some(token)) if !token.is_empty() => {
            tracing::info!(%token_type, "Found cached token");
            Some(token.into())
        }
        Ok(_) => None,
        Err(e) => {
            tracing::warn!(%token_type, error = %e, "Failed to retrieve cached token");
            None
        }
    }
}

fn store_token_in_cache(
    server_url: &str,
    username: &str,
    role: &str,
    token_type: TokenType,
    token_value: &str,
    token_cache: Option<&dyn TokenCache>,
) {
    let Some(cache) = token_cache else {
        tracing::debug!(%token_type, "No token cache available");
        return;
    };
    let key = CacheKey {
        token_type,
        idp: normalize_url(server_url),
        snowflake: normalize_url(server_url),
        username: normalize_identifier(username),
        role: normalize_identifier(role),
    };
    if let Err(e) = cache.add_token(&key, token_value) {
        tracing::warn!(%token_type, error = %e, "Failed to cache token");
    } else {
        tracing::info!(%token_type, "Cached token for future use");
    }
}

fn remove_token_from_cache(
    server_url: &str,
    username: &str,
    role: &str,
    token_type: TokenType,
    token_cache: Option<&dyn TokenCache>,
) {
    let Some(cache) = token_cache else {
        return;
    };
    let key = CacheKey {
        token_type,
        idp: normalize_url(server_url),
        snowflake: normalize_url(server_url),
        username: normalize_identifier(username),
        role: normalize_identifier(role),
    };
    if let Err(e) = cache.remove_token(&key) {
        tracing::warn!(%token_type, error = %e, "Failed to remove cached token");
    } else {
        tracing::info!(%token_type, "Removed cached token");
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
fn evict_oauth_access_token_for_authorization_code(
    cfg: &crate::config::rest_parameters::OAuthAuthorizationCodeConfig,
    server_url: &str,
    role: &str,
    token_cache: Option<&dyn TokenCache>,
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
    oauth::remove_oauth_access_token(&idp_url, server_url, &cfg.username, role, token_cache);
    oauth::remove_oauth_dpop_bundled(&idp_url, server_url, &cfg.username, role, token_cache);
}

pub async fn auth_request_data(
    client: &reqwest::Client,
    login_parameters: &LoginParameters,
    session_parameters: Option<&HashMap<String, String>>,
    token_cache: Option<&dyn TokenCache>,
    prompt_locks: Option<&std::sync::Arc<prompt_lock::PromptLockMap>>,
    retry_policy: &RetryPolicy,
    cancel: tokio_util::sync::CancellationToken,
) -> Result<AuthRequestData, RestError> {
    let mut data = base_auth_request_data(login_parameters);
    data.spcs_token = login_parameters.spcs_token.clone();

    if let Some(params) = session_parameters {
        let json_params = params
            .iter()
            .map(|(k, v)| (k.clone(), serde_json::Value::String(v.clone())))
            .collect();
        data.session_parameters = Some(json_params);
    }

    match &login_parameters.login_method {
        LoginMethod::NativeOkta(okta_config) => {
            let saml_html = fetch_native_okta_saml(
                client,
                login_parameters,
                retry_policy,
                okta_config,
                cancel.clone(),
            )
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
                try_get_cached_token(
                    &login_parameters.server_url,
                    username,
                    login_parameters.role.as_deref().unwrap_or(""),
                    TokenType::IdToken,
                    token_cache,
                )
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
                    cancel.clone(),
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
                token_cache,
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
                        token_cache,
                    )
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
    cancel: tokio_util::sync::CancellationToken,
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

    let response = execute_with_retry(
        build_request,
        &ctx,
        policy,
        |r| async move { Ok(r) },
        cancel.clone(),
    )
    .await
    .context(HttpRetrySnafu {
        context: "login request",
    })?;

    tokio::select! {
        biased;
        _ = cancel.cancelled() => Err(crate::http::retry::CancelledSnafu.build()).context(HttpRetrySnafu {
            context: "login request",
        }),
        parsed = read_response_json::<auth::AuthResponseMain>(response) => {
            parsed.context(InvalidSnowflakeResponseSnafu)
        }
    }
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
    cancel: tokio_util::sync::CancellationToken,
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
        cancel,
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
    token_cache: Option<&dyn TokenCache>,
    prompt_locks: Option<&std::sync::Arc<prompt_lock::PromptLockMap>>,
    retry_policy: &RetryPolicy,
    cancel: tokio_util::sync::CancellationToken,
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
                let lock_key = CacheKey {
                    token_type: TokenType::IdToken,
                    idp: normalize_url(&login_parameters.server_url),
                    snowflake: normalize_url(&login_parameters.server_url),
                    username: normalize_identifier(username),
                    role: normalize_identifier(login_parameters.role.as_deref().unwrap_or("")),
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
                    idp: normalize_url(&login_parameters.server_url),
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
        token_cache,
        prompt_locks,
        retry_policy,
        cancel.clone(),
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
    let mut auth_response = send_login_request(
        client,
        login_parameters,
        &login_request,
        retry_policy,
        cancel.clone(),
    )
    .await?;

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
                    token_cache,
                );
                let retry_data = auth_request_data(
                    client,
                    login_parameters,
                    session_parameters,
                    token_cache,
                    prompt_locks,
                    retry_policy,
                    cancel.clone(),
                )
                .await?;
                let retry_request = AuthRequest { data: retry_data };
                auth_response = send_login_request(
                    client,
                    login_parameters,
                    &retry_request,
                    retry_policy,
                    cancel.clone(),
                )
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
                        token_cache,
                    );
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
                    token_cache,
                    prompt_locks,
                    retry_policy,
                    cancel.clone(),
                )
                .await?;
                let retry_request = AuthRequest { data: retry_data };
                auth_response = send_login_request(
                    client,
                    login_parameters,
                    &retry_request,
                    retry_policy,
                    cancel.clone(),
                )
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
            .unwrap_or(-1);
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
                    token_cache,
                );
            }
        }
        LoginSnafu { message, code }.fail()?;
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
                auth_response.data.id_token.as_ref().map(|t| {
                    (
                        username.as_str(),
                        login_parameters.role.as_deref().unwrap_or(""),
                        TokenType::IdToken,
                        t,
                    )
                })
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
        );
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
        // GS 390114 on the refresh endpoint means the master token itself has
        // expired: the session can never be renewed. Surface the discriminable
        // MasterTokenExpired variant so callers can mark the connection expired,
        // mirroring the query-response path in read_response_json.
        if code == MASTER_TOKEN_EXPIRED {
            return MasterTokenExpiredSnafu
                .fail()
                .context(InvalidSnowflakeResponseSnafu);
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
    execution_mode: QueryExecutionMode,
    crl_worker: SharedCrlWorker,
    cancel: tokio_util::sync::CancellationToken,
) -> Result<query_response::Response, RestError> {
    let client = build_tls_http_client(&query_parameters.client_info, crl_worker)?;
    let policy = RetryPolicy::default();
    snowflake_query_with_client(
        &client,
        query_parameters,
        session_token,
        query_input,
        &policy,
        execution_mode,
        cancel,
    )
    .await
}

#[tracing::instrument(
    skip(client, query_parameters, session_token, query_input),
    fields(sql)
)]
pub async fn snowflake_query_with_client<'a>(
    client: &reqwest::Client,
    query_parameters: QueryParameters,
    session_token: impl AsRef<str>,
    query_input: QueryInput<'a>,
    retry_policy: &RetryPolicy,
    execution_mode: QueryExecutionMode,
    cancel: tokio_util::sync::CancellationToken,
) -> Result<query_response::Response, RestError> {
    let session_token = session_token.as_ref();

    // Async mode path (legacy, opt-in)
    if matches!(execution_mode, QueryExecutionMode::Async) {
        return execute_async_with_fallback(
            client,
            &query_parameters,
            session_token,
            query_input,
            retry_policy,
            cancel,
        )
        .await;
    }

    // Sync mode (default): use requestId-based retry for connection failures
    execute_sync_with_retry(
        client,
        &query_parameters,
        session_token,
        &query_input,
        retry_policy,
        cancel,
    )
    .await
}

/// Execute query in async mode with fallback to sync for error 612.
async fn execute_async_with_fallback<'a>(
    client: &reqwest::Client,
    query_parameters: &QueryParameters,
    session_token: &str,
    query_input: QueryInput<'a>,
    retry_policy: &RetryPolicy,
    cancel: tokio_util::sync::CancellationToken,
) -> Result<query_response::Response, RestError> {
    match snowflake_query_async_style(
        client,
        query_parameters,
        session_token,
        &query_input,
        retry_policy,
        cancel.clone(),
    )
    .await
    {
        Ok(response) => return Ok(response),
        Err(RestError::AsyncQuery {
            source:
                SfError::AsyncPollResultNotFound {
                    is_first_poll: true,
                    ..
                },
            ..
        }) => {
            // Error 612 "Result not found" on first poll - fall through to sync retry.
        }
        Err(
            e @ RestError::AsyncQuery {
                source:
                    SfError::AsyncPollResultNotFound {
                        is_first_poll: false,
                        ..
                    },
                ..
            },
        ) => {
            let RestError::AsyncQuery { request_id, .. } = &e else {
                unreachable!()
            };
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
    let response = execute_sync_with_retry(
        client,
        query_parameters,
        session_token,
        &query_input,
        retry_policy,
        cancel,
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

/// Execute a sync query with HTTP-level retries for transient transport / 5xx
/// failures.
///
/// Retry handling lives in [`execute_sync_query`], which wraps the actual
/// `POST /queries/v1/query-request` call with [`execute_with_retry`]. The
/// `requestId` is generated here once and threaded through so that every
/// HTTP-level replay reuses the same id; the second and subsequent attempts
/// also carry `retry=true`, giving the server the hint it needs to dedupe
/// against an already-running/completed query.
async fn execute_sync_with_retry<'a>(
    client: &reqwest::Client,
    query_parameters: &QueryParameters,
    session_token: &str,
    query_input: &QueryInput<'a>,
    retry_policy: &RetryPolicy,
    cancel: tokio_util::sync::CancellationToken,
) -> Result<query_response::Response, RestError> {
    let request_id = uuid::Uuid::new_v4();

    execute_sync_query(
        client,
        query_parameters,
        session_token,
        query_input,
        request_id,
        retry_policy,
        cancel,
    )
    .await
}

/// Map a Snowflake query response into a `Result`, converting
/// `response.success == false` into `RestError::QueryFailed` with
/// the server's message, error code, SQL state, and query ID.
fn into_query_result(
    response: query_response::Response,
) -> Result<query_response::Response, RestError> {
    if !response.success {
        let message = response
            .message
            .unwrap_or_else(|| "Unknown error".to_owned());
        let code = response.code.as_deref().and_then(|c| c.parse::<i32>().ok());
        let sql_state = response.data.sql_state.clone();
        let query_id = response.data.query_id.clone();

        return QueryFailedSnafu {
            message,
            code,
            sql_state,
            query_id,
        }
        .fail();
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
    cancel: tokio_util::sync::CancellationToken,
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

    let response = execute_with_retry(
        build_request,
        &ctx,
        retry_policy,
        |r| async move { Ok(r) },
        cancel.clone(),
    )
    .await
    .context(HttpRetrySnafu {
        context: "query request",
    })?;

    let query_response = tokio::select! {
        biased;
        _ = cancel.cancelled() => {
            return Err(crate::http::retry::CancelledSnafu.build()).context(HttpRetrySnafu {
                context: "query request",
            });
        }
        parsed = read_response_json::<query_response::Data>(response) => {
            parsed.context(InvalidSnowflakeResponseSnafu)?
        }
    };

    let elapsed_ms = send_start.elapsed().as_secs_f64() * 1000.0;
    tracing::debug!(
        elapsed_ms,
        request_id = %request_id,
        query_id = query_response.data.query_id.as_deref().unwrap_or_default(),
        "Sync query response received"
    );

    let query_response = if async_exec::should_poll_for_completion(&query_response) {
        tracing::debug!(request_id = %request_id, "detached query - polling for completion");
        async_exec::poll_detached_query(
            client,
            query_parameters,
            session_token,
            &query_response,
            retry_policy,
            cancel,
        )
        .await
        .context(AsyncQuerySnafu {
            request_id: Some(request_id),
            query_id: query_response
                .data
                .query_id
                .as_deref()
                .and_then(|s| Uuid::parse_str(s).ok()),
        })?
    } else {
        query_response
    };

    into_query_result(query_response)
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
    cancel: tokio_util::sync::CancellationToken,
) -> Result<query_response::Response, RestError> {
    let request_id = uuid::Uuid::new_v4();
    crate::rest::snowflake::async_exec::execute_blocking_with_async(
        client,
        query_parameters,
        session_token.as_ref(),
        query_input,
        request_id,
        retry_policy,
        cancel,
    )
    .await
    .context(AsyncQuerySnafu {
        request_id: Some(request_id),
        query_id: None,
    })
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
    cancel: tokio_util::sync::CancellationToken,
) -> Result<query_response::Response, RestError> {
    tracing::info!(query_id = query_id, "Fetching query result");

    let result_url = format!(
        "{}/queries/{}/result",
        query_parameters.server_url, query_id
    );
    let uuid = Uuid::parse_str(query_id).expect("Failed to parse query_id");
    let query_response = async_exec::poll_query_status(
        client,
        &query_parameters.client_info,
        session_token,
        &result_url,
        retry_policy,
        cancel,
    )
    .await
    .context(AsyncQuerySnafu {
        request_id: None,
        query_id: Some(uuid),
    })?;

    into_query_result(query_response)
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
    cancel: tokio_util::sync::CancellationToken,
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
    let response = execute_with_retry(
        build_request,
        &ctx,
        retry_policy,
        |r| async move { Ok(r) },
        cancel.clone(),
    )
    .await
    .context(HttpRetrySnafu {
        context: "query status",
    })?;

    let body: QueryStatusResponse = tokio::select! {
        biased;
        _ = cancel.cancelled() => {
            return Err(crate::http::retry::CancelledSnafu.build()).context(HttpRetrySnafu {
                context: "query status",
            });
        }
        parsed = read_response_json::<Option<QueryStatusResponseData>>(response) => {
            parsed.context(InvalidSnowflakeResponseSnafu)?
        }
    };

    if !body.success {
        let message = body.message.unwrap_or_else(|| "Unknown error".to_owned());
        let code = body.code.as_deref().and_then(|c| c.parse::<i32>().ok());
        return QueryFailedSnafu {
            message,
            code,
            sql_state: None::<String>,
            query_id: Some(query_id.to_owned()),
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

/// Abort a running query by its Snowflake Query ID.
///
/// Issues `POST /queries/{query_id}/abort-request` with an empty JSON body.
/// Returns `Ok(())` when the server acknowledges the abort (`success: true`),
/// or `RestError::QueryFailed` when `success: false`.
#[tracing::instrument(skip(client, query_parameters, session_token))]
pub async fn snowflake_abort_query(
    client: &reqwest::Client,
    query_parameters: &QueryParameters,
    session_token: &str,
    query_id: &str,
) -> Result<(), RestError> {
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

    let abort_response = read_response_json::<serde_json::Value>(response)
        .await
        .context(InvalidSnowflakeResponseSnafu)?;

    if !abort_response.success {
        return QueryFailedSnafu {
            message: abort_response
                .message
                .unwrap_or_else(|| "Abort query failed".to_owned()),
            query_id: query_id.to_owned(),
            code: Option::<i32>::None,
            sql_state: Option::<String>::None,
        }
        .fail();
    }

    Ok(())
}

/// Standard Snowflake JSON response envelope: `{success, code, message, data: T}`.
///
/// Every REST endpoint parsed by [`read_response_json`] returns this shape; the
/// generic `T` is the endpoint-specific payload. Keeping the envelope uniform
/// lets `read_response_json` inspect `success` + `code` centrally and map
/// body-level `390112` (session-token expired) to `SessionExpired` for the
/// single-flight `RefreshContext` refresh path — without each caller having
/// to re-implement that check.
#[derive(Debug, serde::Deserialize)]
pub struct SnowflakeResponse<T> {
    pub success: bool,
    #[serde(default)]
    pub code: Option<String>,
    #[serde(default)]
    pub message: Option<String>,
    #[serde(default)]
    pub data: T,
}

pub(crate) async fn read_response_json<T>(
    response: reqwest::Response,
) -> Result<SnowflakeResponse<T>, SnowflakeResponseError>
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
        let body = response_text.unwrap_or("Unknown error".to_string());
        let truncated = if body.len() > 1024 {
            let mut end = 1024;
            while !body.is_char_boundary(end) {
                end -= 1;
            }
            format!("{}… ({} bytes total)", &body[..end], body.len())
        } else {
            body
        };
        return ResponseStatusSnafu {
            status: response_status,
            message: truncated,
        }
        .fail();
    }

    let response_text = response_text.context(ResponseTextSnafu)?;

    tracing::debug!(response_len = response_text.len(), "Received HTTP response");
    let parsed: SnowflakeResponse<T> =
        serde_json::from_str(&response_text).context(ResponseFormatSnafu)?;

    // 2xx with `success:false, code:"390112"` means the session token expired.
    // Surface it as SessionExpired so the RefreshContext can refresh and retry,
    // matching the HTTP 401 branch above.
    if !parsed.success
        && parsed.code.as_deref().and_then(|c| c.parse::<i32>().ok()) == Some(SESSION_TOKEN_EXPIRED)
    {
        return SessionExpiredSnafu.fail();
    }

    // 2xx with `success:false, code:"390114"` means the master token has expired.
    // The session can never be renewed; surface it so RefreshContext can set
    // `is_master_token_expired = true` and propagate `MasterTokenExpired` to the caller.
    if !parsed.success
        && parsed.code.as_deref().and_then(|c| c.parse::<i32>().ok()) == Some(MASTER_TOKEN_EXPIRED)
    {
        return MasterTokenExpiredSnafu.fail();
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

#[derive(Debug, Snafu, error_trace::ErrorTrace)]
pub enum RestError {
    #[snafu(display("{operation} timed out after {budget:?}"))]
    #[snafu(visibility(pub(crate)))]
    OperationTimeout {
        operation: String,
        budget: std::time::Duration,
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
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("Async Snowflake query failed"))]
    AsyncQuery {
        source: SfError,
        request_id: Option<Uuid>,
        query_id: Option<Uuid>,
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
        /// Snowflake Query ID associated with the failed query.
        query_id: Option<String>,
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("HTTP request failed after retries: {context}"))]
    HttpRetry {
        context: &'static str,
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

impl RestError {
    pub(crate) fn is_cancelled(&self) -> bool {
        match self {
            RestError::AsyncQuery { source, .. } => source.is_cancelled(),
            RestError::HttpRetry { source, .. } => source.is_cancelled(),
            RestError::NativeOkta { source, .. } => source.is_cancelled(),
            RestError::ExternalBrowser { source, .. } => source.is_cancelled(),
            _ => false,
        }
    }
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
    #[snafu(display("Session expired - reauthentication required"))]
    SessionExpired {
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("Master token expired - full re-authentication required (GS code 390114)"))]
    MasterTokenExpired {
        #[snafu(implicit)]
        location: Location,
    },
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
                idp: normalize_url(server_url),
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
            idp: normalize_url(server_url),
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
            client_info: test_client_info(),
            session_parameters: None,
            spcs_token: None,
            disable_parallel_user_prompt: false,
        }
    }

    mod token_cache_helpers_tests {
        use super::*;

        fn assert_get_store_remove_for(token_type: TokenType) {
            const SERVER: &str = "https://host.example.com";

            // try_get: returns cached token on hit
            let cache = StubTokenCache::with_token(SERVER, "alice", "", token_type, "tok_val");
            let result = try_get_cached_token(SERVER, "alice", "", token_type, Some(&cache));
            assert_eq!(result.unwrap().reveal(), "tok_val");

            // try_get: returns None on cache miss
            let empty = StubTokenCache::new();
            assert!(try_get_cached_token(SERVER, "alice", "", token_type, Some(&empty)).is_none());

            // try_get: returns None when no cache provided
            assert!(try_get_cached_token(SERVER, "alice", "", token_type, None).is_none());

            // try_get: returns None for invalid URL
            assert!(
                try_get_cached_token("not-a-url", "alice", "", token_type, Some(&empty)).is_none()
            );

            // try_get: returns None for empty cached value
            let empty_val = StubTokenCache::with_token(SERVER, "alice", "", token_type, "");
            assert!(
                try_get_cached_token(SERVER, "alice", "", token_type, Some(&empty_val)).is_none()
            );

            // store + get round-trip
            let cache = StubTokenCache::new();
            store_token_in_cache(SERVER, "alice", "", token_type, "new_tok", Some(&cache));
            let stored = cache
                .get_token(&key_for(SERVER, "alice", "", token_type))
                .unwrap();
            assert_eq!(stored.as_deref(), Some("new_tok"));

            // store: no panic when no cache
            store_token_in_cache(SERVER, "alice", "", token_type, "tok", None);

            // store: no panic for invalid URL
            store_token_in_cache(
                "not-a-url",
                "alice",
                "",
                token_type,
                "tok",
                Some(&StubTokenCache::new()),
            );

            // remove evicts token
            let cache = StubTokenCache::with_token(SERVER, "alice", "", token_type, "to_remove");
            remove_token_from_cache(SERVER, "alice", "", token_type, Some(&cache));
            assert!(
                cache
                    .get_token(&key_for(SERVER, "alice", "", token_type))
                    .unwrap()
                    .is_none()
            );

            // remove: no panic when no cache
            remove_token_from_cache(SERVER, "alice", "", token_type, None);

            // remove: no panic for invalid URL
            remove_token_from_cache(
                "not-a-url",
                "alice",
                "",
                token_type,
                Some(&StubTokenCache::new()),
            );
        }

        #[test]
        fn mfa_token_cache_operations() {
            assert_get_store_remove_for(TokenType::MfaToken);
        }

        #[test]
        fn id_token_cache_operations() {
            assert_get_store_remove_for(TokenType::IdToken);
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

            match into_query_result(resp) {
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
                    "sqlState": "42000",
                    "queryId": "01abc-def-12345"
                }
            }));

            match into_query_result(resp) {
                Err(RestError::QueryFailed {
                    message,
                    code,
                    sql_state,
                    query_id,
                    ..
                }) => {
                    assert_eq!(message, "SQL compilation error");
                    assert_eq!(code, Some(1003));
                    assert_eq!(sql_state, Some("42000".to_owned()));
                    assert_eq!(query_id, Some("01abc-def-12345".to_owned()));
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

            match into_query_result(resp) {
                Err(RestError::QueryFailed {
                    message,
                    code,
                    sql_state,
                    query_id,
                    ..
                }) => {
                    assert_eq!(message, "Unknown error");
                    assert_eq!(code, None);
                    assert_eq!(sql_state, None);
                    assert_eq!(query_id, None);
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
                tokio_util::sync::CancellationToken::new(),
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
                tokio_util::sync::CancellationToken::new(),
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
                tokio_util::sync::CancellationToken::new(),
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
                tokio_util::sync::CancellationToken::new(),
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

            let result = send_login_request(
                &client,
                &params,
                &auth_req,
                &RetryPolicy::default(),
                tokio_util::sync::CancellationToken::new(),
            )
            .await;

            assert!(result.is_ok(), "Expected retry to succeed, got: {result:?}");
            assert_eq!(
                attempt.load(Ordering::SeqCst),
                3,
                "Expected exactly 3 attempts (2 failures + 1 success), got {}",
                attempt.load(Ordering::SeqCst)
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
                tokio_util::sync::CancellationToken::new(),
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
            matches!(result, Err(SnowflakeResponseError::SessionExpired { .. })),
            "expected SessionExpired, got {result:?}"
        );
    }
}
