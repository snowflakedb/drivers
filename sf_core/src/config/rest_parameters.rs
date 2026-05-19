use std::collections::HashMap;
use std::fmt;
use std::fs;
use std::sync::Arc;

use url::Url;

use crate::config::InvalidParameterValueSnafu;
use crate::config::param_registry::param_names;
use crate::config::settings::Setting;
use crate::config::settings::Settings;
use crate::config::{ConfigError, ConflictingParametersSnafu, MissingParameterSnafu};
use crate::crl::config::CrlConfig;
use crate::rest::snowflake::BrowserLaunchFn;
use crate::sensitive::SensitiveString;
use crate::tls::config::TlsConfig;
use openssl::pkey::PKey;
use snafu::OptionExt;

fn get_server_url(settings: &dyn Settings) -> Result<String, ConfigError> {
    if let Some(Setting::String(value)) = settings.get("server_url") {
        return Ok(value.clone());
    }

    let protocol = settings
        .get_string("protocol")
        .unwrap_or("https".to_string());
    let host = settings
        .get_string("host")
        .context(MissingParameterSnafu { parameter: "host" })?;
    if protocol != "https" && protocol != "http" {
        tracing::warn!("Unexpected protocol specified during server url construction: {protocol}");
    }

    // Check if a custom port is specified
    let base_url = format!("{protocol}://{host}");
    if let Some(port) = settings.get_int("port") {
        return Ok(format!("{base_url}:{port}"));
    }

    Ok(base_url)
}

pub const DEFAULT_LOG_MAX_QUERY_LENGTH: usize = 80;

/// Read `log_max_query_length` from a settings bag, clamp to non-negative,
/// and fall back to [`DEFAULT_LOG_MAX_QUERY_LENGTH`] when absent.
pub fn resolve_log_max_query_length(settings: &dyn Settings) -> usize {
    settings
        .get_int(param_names::LOG_MAX_QUERY_LENGTH.as_str())
        .map(|v| v.max(0) as usize)
        .unwrap_or(DEFAULT_LOG_MAX_QUERY_LENGTH)
}

/// Read a boolean-typed parameter that may have been provided as a bool, an int,
/// or a string ("true"/"false"/"1"/"0"). Falls back to `default` when absent or
/// when present but unparseable.
fn resolve_bool_param(settings: &dyn Settings, key: &str, default: bool) -> bool {
    settings
        .get_bool(key)
        .or_else(|| {
            settings
                .get_string(key)
                .map(|v| v.eq_ignore_ascii_case("true") || v == "1")
        })
        .or_else(|| settings.get_int(key).map(|v| v != 0))
        .unwrap_or(default)
}

/// Read `log_query_text` from a settings bag, accepting bool/int/string values.
pub fn resolve_log_query_text(settings: &dyn Settings) -> bool {
    resolve_bool_param(settings, param_names::LOG_QUERY_TEXT.as_str(), false)
}

/// Read `log_query_parameters` from a settings bag, accepting bool/int/string values.
pub fn resolve_log_query_parameters(settings: &dyn Settings) -> bool {
    resolve_bool_param(settings, param_names::LOG_QUERY_PARAMETERS.as_str(), false)
}

#[derive(Clone)]
pub struct QueryParameters {
    pub server_url: String,
    pub client_info: ClientInfo,
    pub log_max_query_length: usize,
    /// Include the (truncated) SQL text in INFO query logs.
    pub log_query_text: bool,
    /// Include the (truncated) JSON bindings in INFO query logs (only honored
    /// when [`Self::log_query_text`] is also true).
    pub log_query_parameters: bool,
}

impl QueryParameters {
    /// Build transport parameters from an arbitrary settings bag (e.g. tests, pre-connect paths).
    ///
    /// After login, prefer `Connection::query_transport_parameters` (transport snapshot)
    /// instead of re-reading merged settings.
    pub fn from_settings(settings: &dyn Settings) -> Result<Self, ConfigError> {
        Ok(Self {
            server_url: get_server_url(settings)?,
            client_info: ClientInfo::from_settings(settings)?,
            log_max_query_length: resolve_log_max_query_length(settings),
            log_query_text: resolve_log_query_text(settings),
            log_query_parameters: resolve_log_query_parameters(settings),
        })
    }
}
#[derive(Clone, Debug)]
pub struct ClientInfo {
    /// Driver identity sent as CLIENT_APP_ID and used in the User-Agent header.
    pub client_app_id: String,
    /// User-facing application name sent as CLIENT_ENVIRONMENT.APPLICATION.
    /// Falls back to `client_app_id` when not explicitly provided.
    pub application: String,
    pub version: String,
    pub os: String,
    pub os_version: String,
    pub ocsp_mode: Option<String>,
    /// Wrapper runtime name (e.g. "CPython", "OpenJDK"). Only set by language wrappers.
    pub runtime_name: Option<String>,
    /// Wrapper runtime version (e.g. "3.11.6", "21.0.1"). Only set by language wrappers.
    pub runtime_version: Option<String>,
    /// Wrapper compiler info (e.g. "Clang 13.0.0 ..."). Only set by language wrappers.
    pub compiler: Option<String>,
    pub crl_config: CrlConfig,
    pub tls_config: TlsConfig,
    pub platforms: Vec<String>,
    pub os_details: Option<HashMap<String, String>>,
}

impl ClientInfo {
    pub fn from_settings(settings: &dyn Settings) -> Result<Self, ConfigError> {
        let crl_config = CrlConfig::from_settings(settings)?;
        let tls_config = TlsConfig::from_settings(settings)?;

        let client_app_id = settings
            .get_string("client_app_id")
            .filter(|s| !s.trim().is_empty())
            .unwrap_or_else(|| env!("CARGO_PKG_NAME").to_string());
        let application = settings
            .get_string("application")
            .filter(|s| !s.trim().is_empty())
            .unwrap_or_else(|| client_app_id.clone());
        let client_info = ClientInfo {
            client_app_id,
            application,
            version: settings
                .get_string("client_app_version")
                .filter(|s| !s.trim().is_empty())
                .unwrap_or_else(|| env!("CARGO_PKG_VERSION").to_string()),
            os: std::env::consts::OS.to_string(),
            os_version: crate::telemetry::environment::detect_os_version(),
            ocsp_mode: Some("FAIL_OPEN".to_string()),
            runtime_name: settings
                .get_string("client_runtime_name")
                .and_then(|s| if s.trim().is_empty() { None } else { Some(s) }),
            runtime_version: settings
                .get_string("client_runtime_version")
                .and_then(|s| if s.trim().is_empty() { None } else { Some(s) }),
            compiler: settings
                .get_string("client_compiler")
                .and_then(|s| if s.trim().is_empty() { None } else { Some(s) }),
            crl_config,
            tls_config,
            platforms: Vec::new(),
            os_details: None,
        };
        Ok(client_info)
    }
}

#[cfg(any(test, feature = "test-utils"))]
pub mod test_fixtures {
    use super::ClientInfo;
    use crate::crl::config::CrlConfig;
    use crate::tls::config::TlsConfig;

    /// Minimal [`ClientInfo`] for tests. Uses [`TlsConfig::insecure`] so it works
    /// with plain-HTTP mock servers. Override specific fields with struct-update
    /// syntax: `ClientInfo { application: "foo".into(), ..test_client_info() }`.
    pub fn test_client_info() -> ClientInfo {
        ClientInfo {
            client_app_id: "sf_core_test".to_string(),
            application: "sf_core_test".to_string(),
            version: "1.0.0".to_string(),
            os: std::env::consts::OS.to_string(),
            os_version: "1.0".to_string(),
            ocsp_mode: None,
            runtime_name: None,
            runtime_version: None,
            compiler: None,
            crl_config: CrlConfig::default(),
            tls_config: TlsConfig::insecure(),
            platforms: Vec::new(),
            os_details: None,
        }
    }
}

pub struct LoginParameters {
    pub account_name: String,
    pub login_method: LoginMethod,
    pub server_url: String,
    pub database: Option<String>,
    pub schema: Option<String>,
    pub warehouse: Option<String>,
    pub role: Option<String>,
    pub client_info: ClientInfo,
    pub session_parameters: Option<HashMap<String, String>>,
    pub spcs_token: Option<String>,
}

impl LoginParameters {
    /// Build login request fields from a resolved settings map (defaults + files + connection seed).
    ///
    /// Session defaults (`database`, `schema`, etc.) are included only when they are part of the
    /// resolved connect seed (`used_at_connect` session fields in the registry).
    pub fn from_settings(settings: &dyn Settings) -> Result<Self, ConfigError> {
        Ok(Self {
            account_name: {
                if let Some(value) = settings.get_string("account") {
                    value
                } else {
                    MissingParameterSnafu {
                        parameter: "account",
                    }
                    .fail()?
                }
            },
            login_method: LoginMethod::from_settings(settings)?,
            server_url: get_server_url(settings)?,
            database: settings.get_string("database"),
            schema: settings.get_string("schema"),
            warehouse: settings.get_string("warehouse"),
            role: settings.get_string("role"),
            client_info: ClientInfo::from_settings(settings)?,
            session_parameters: None,
            spcs_token: None,
        })
    }
}

pub const DEFAULT_AUTHENTICATION_TIMEOUT_SECS: u64 = 120;

#[derive(Debug)]
pub struct NativeOktaConfig {
    /// Snowflake user name (used in authenticator-request to Snowflake).
    pub username: String,
    /// Optional override for the Okta login name. When set, this is sent to
    /// Okta's `/api/v1/authn` instead of `username`. Matches JDBC's `oktausername`
    /// property — useful when the Okta email differs from the Snowflake user.
    pub okta_username: Option<String>,
    /// IdP password (native Okta SSO).
    pub password: SensitiveString,
    /// Okta authenticator URL endpoint (native Okta SSO).
    pub okta_url: Url,
    /// Disable SAML destination/postback validation (default false; discouraged).
    pub disable_saml_url_check: bool,
    /// End-to-end auth budget for the Okta flow, mapped onto retry max_elapsed.
    pub authentication_timeout_secs: u64,
}

/// OAuth 2.0 Authorization Code (with PKCE) flow configuration.
///
/// Mirrors the cross-driver configuration matrix
/// (JDBC/ODBC/Python/.NET/Go/Node). All optional URL fields fall back to
/// Snowflake-as-IdP defaults at flow time: `https://{host}/oauth/authorize`,
/// `https://{host}/oauth/token-request`, and an ephemeral
/// `http://127.0.0.1:<random>` loopback redirect URI.
pub struct OAuthAuthorizationCodeConfig {
    /// Snowflake user name. Sent unchanged in the login-request body
    /// (`LOGIN_NAME` is always set; .NET's `loginName=""` quirk is not replicated).
    pub username: String,
    /// IdP-issued client identifier. For Snowflake-as-IdP the wiring step
    /// will substitute `LOCAL_APPLICATION` when this is empty.
    pub client_id: String,
    /// IdP-issued client secret.
    pub client_secret: SensitiveString,
    /// Optional override for the IdP authorization endpoint.
    /// `None` ⇒ default `https://{host}/oauth/authorize`.
    pub authorization_url: Option<Url>,
    /// Optional override for the IdP token endpoint.
    /// `None` ⇒ default `https://{host}/oauth/token-request`. Also used to
    /// derive the OAuth cache-key host.
    pub token_url: Option<Url>,
    /// Optional override for the loopback redirect URI advertised to the IdP.
    /// `None` ⇒ ephemeral `http://127.0.0.1:<random>` (bind to `127.0.0.1`,
    /// never `0.0.0.0`).
    pub redirect_uri: Option<Url>,
    /// OAuth scope string (space-separated). `None` ⇒ derived from role
    /// (`session:role:<role>`).
    pub scope: Option<String>,
    /// Snowflake-as-IdP only: request single-use refresh-token rotation by
    /// adding `enable_single_use_refresh_tokens=true` to the token body.
    /// Defaults to `false`.
    pub enable_single_use_refresh_tokens: bool,
    /// Python-only escape hatch: disable PKCE S256. All other drivers
    /// always run PKCE; defaults to `false` here.
    pub disable_pkce: bool,
    /// Whether refresh tokens may be persisted to the OS-level token cache
    /// (controls `client_store_temporary_credential`).
    pub client_store_temporary_credential: bool,
    /// Driver-local flow behavior (DPoP, timeout). Not sent to Snowflake.
    pub flow_options: OAuthFlowOptions,
    /// Optional factory that mints the browser launcher used by the AC
    /// interactive leg. `None` ⇒ the flow falls back to the production
    /// "open the OS browser, paste-URL on failure" default. Tests inject
    /// a no-op (or deterministic loopback driver) by setting this field
    /// directly. Wrapped in an `Arc` so the AC retry-on-failure path can
    /// rebuild a fresh `FnOnce` for the retry leg without consuming the
    /// caller's factory.
    ///
    /// Defaulted to a no-op by [`Self::from_settings`] under
    /// `cfg(any(test, feature = "test-utils"))` so integration / e2e
    /// builds never pop a real browser window against wiremock IdPs;
    /// production builds carry `None` and use the system browser.
    pub(crate) browser_launcher: Option<Arc<dyn Fn() -> BrowserLaunchFn + Send + Sync>>,
}

// `Arc<dyn Fn() -> BrowserLaunchFn + Send + Sync>` is not `Debug`, so we
// can't derive `Debug` on the struct as a whole. The manual impl elides
// the launcher (it's opaque function data and carries nothing useful in
// log output) and prints every other field with its normal `Debug`.
impl fmt::Debug for OAuthAuthorizationCodeConfig {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("OAuthAuthorizationCodeConfig")
            .field("username", &self.username)
            .field("client_id", &self.client_id)
            .field("client_secret", &self.client_secret)
            .field("authorization_url", &self.authorization_url)
            .field("token_url", &self.token_url)
            .field("redirect_uri", &self.redirect_uri)
            .field("scope", &self.scope)
            .field(
                "enable_single_use_refresh_tokens",
                &self.enable_single_use_refresh_tokens,
            )
            .field("disable_pkce", &self.disable_pkce)
            .field(
                "client_store_temporary_credential",
                &self.client_store_temporary_credential,
            )
            .field("flow_options", &self.flow_options)
            .field(
                "browser_launcher",
                &self.browser_launcher.as_ref().map(|_| "<opaque>"),
            )
            .finish()
    }
}

/// Driver-local OAuth flow behavior knobs shared by both AC and CC.
///
/// These parameters control the driver's own OAuth flow machinery (DPoP
/// proof generation, timeout budget). They are **not** transmitted to
/// Snowflake in the login-request JSON — they are consumed entirely
/// within the OAuth flow engine in `sf_core::rest::snowflake::oauth`.
#[derive(Debug)]
pub struct OAuthFlowOptions {
    /// Enable RFC 9449 DPoP proof-of-possession on token + login requests.
    /// Currently only JDBC has DPoP parity; defaults to `false`.
    pub enable_dpop: bool,
    /// End-to-end auth budget for the OAuth flow.
    pub authentication_timeout_secs: u64,
}

/// OAuth 2.0 Client Credentials flow configuration (external IdP only —
/// Snowflake's GS does not issue tokens for
/// `grant_type=client_credentials`).
#[derive(Debug)]
pub struct OAuthClientCredentialsConfig {
    /// Snowflake user name (sent in the Snowflake login-request body).
    pub username: String,
    /// IdP-issued client identifier (required for CC).
    pub client_id: String,
    /// IdP-issued client secret (required for CC).
    pub client_secret: SensitiveString,
    /// IdP token endpoint. **Required** for CC: there is no Snowflake default
    /// because Snowflake-as-IdP does not support CC.
    pub token_url: Url,
    /// OAuth scope string (space-separated). `None` ⇒ derived from role.
    pub scope: Option<String>,
    /// Driver-local flow behavior (DPoP, timeout). Not sent to Snowflake.
    pub flow_options: OAuthFlowOptions,
}

#[derive(Debug)]
pub enum LoginMethod {
    Password {
        username: String,
        password: SensitiveString,
    },
    NativeOkta(NativeOktaConfig),
    PrivateKey {
        username: String,
        private_key: SensitiveString,
        passphrase: Option<SensitiveString>,
    },
    Pat {
        username: String,
        token: SensitiveString,
    },
    UserPasswordMfa {
        username: String,
        password: SensitiveString,
        passcode_in_password: bool,
        passcode: Option<SensitiveString>,
        client_store_temporary_credential: bool,
    },
    ExternalBrowser {
        username: String,
        authentication_timeout_secs: u64,
    },
    /// Pre-acquired OAuth access token (legacy `AUTHENTICATOR=OAUTH` with
    /// raw `token=`). The driver forwards the token to Snowflake unchanged.
    OAuthAccessToken {
        username: String,
        token: SensitiveString,
    },
    /// OAuth 2.0 Authorization Code with PKCE (S256). Multi-step flow
    /// orchestrated outside of `create_credentials`.
    OAuthAuthorizationCode(OAuthAuthorizationCodeConfig),
    /// OAuth 2.0 Client Credentials. External IdP only.
    OAuthClientCredentials(OAuthClientCredentialsConfig),
}

pub(crate) fn non_empty_string(settings: &dyn Settings, key: &str) -> Option<String> {
    settings.get_string(key).filter(|s| !s.is_empty())
}

/// Read a boolean parameter that wrappers may submit as a typed bool,
/// a string (`"true"`, `"1"`), or an int (`0`/`1`). Defaults to `false`
/// when absent or unparseable so OAuth knobs like `enable_dpop` behave
/// the same regardless of the wrapper's setting representation.
pub(crate) fn get_flexible_bool(settings: &dyn Settings, key: &str) -> bool {
    settings
        .get_bool(key)
        .or_else(|| {
            settings
                .get_string(key)
                .map(|v| v.eq_ignore_ascii_case("true") || v == "1")
        })
        .or_else(|| settings.get_int(key).map(|v| v != 0))
        .unwrap_or(false)
}

/// Parse an optional URL parameter, returning [`InvalidParameterValue`]
/// when the user supplied a value that cannot be parsed by the `url`
/// crate. Empty/missing values resolve to `None` so callers can fall
/// back to flow-time defaults (e.g. `https://{host}/oauth/authorize`).
pub(crate) fn parse_optional_url(
    settings: &dyn Settings,
    key: &'static str,
) -> Result<Option<Url>, ConfigError> {
    let Some(raw) = non_empty_string(settings, key) else {
        return Ok(None);
    };
    let url = Url::parse(&raw).map_err(|e| {
        InvalidParameterValueSnafu {
            parameter: key,
            value: raw,
            explanation: format!("Could not parse URL: {e}"),
        }
        .build()
    })?;
    Ok(Some(url))
}

/// Parse a required URL parameter (e.g. CC `oauth_token_request_url`).
pub(crate) fn parse_required_url(
    settings: &dyn Settings,
    key: &'static str,
) -> Result<Url, ConfigError> {
    let raw = non_empty_string(settings, key).context(MissingParameterSnafu { parameter: key })?;
    Url::parse(&raw).map_err(|e| {
        InvalidParameterValueSnafu {
            parameter: key,
            value: raw,
            explanation: format!("Could not parse URL: {e}"),
        }
        .build()
    })
}

impl OAuthAuthorizationCodeConfig {
    pub(crate) fn from_settings(settings: &dyn Settings) -> Result<Self, ConfigError> {
        let username = non_empty_string(settings, "user")
            .context(MissingParameterSnafu { parameter: "user" })?;
        // Snowflake-as-IdP substitutes `LOCAL_APPLICATION` for
        // missing client_id/client_secret at flow time
        // (analysis_feature_oauth.md §1, §9). Keep them empty here
        // and let the AC provider apply that default.
        let client_id = non_empty_string(settings, "oauth_client_id").unwrap_or_default();
        let client_secret = non_empty_string(settings, "oauth_client_secret").unwrap_or_default();
        let authorization_url = parse_optional_url(settings, "oauth_authorization_url")?;
        let token_url = parse_optional_url(settings, "oauth_token_request_url")?;
        let redirect_uri = parse_optional_url(settings, "oauth_redirect_uri")?;
        let scope = non_empty_string(settings, "oauth_scope");
        let enable_single_use_refresh_tokens =
            get_flexible_bool(settings, "oauth_enable_single_use_refresh_tokens");
        let disable_pkce = get_flexible_bool(settings, "oauth_disable_pkce");
        let enable_dpop = get_flexible_bool(settings, "oauth_enable_dpop");
        let client_store_temporary_credential =
            get_flexible_bool(settings, "client_store_temporary_credential");
        let authentication_timeout_secs = settings
            .get_u64("authentication_timeout")
            .unwrap_or(DEFAULT_AUTHENTICATION_TIMEOUT_SECS);

        // Compile-time conditional default for the browser launcher.
        // Production builds carry `None` and let the AC flow open the
        // real OS browser. Test / `test-utils` builds default to a
        // no-op factory so integration and e2e tests that drive the
        // AC flow against wiremock IdPs never pop a real browser
        // window. There is no runtime mutable state involved — the
        // value is fixed at compile time and rides with the config.
        #[cfg(not(any(test, feature = "test-utils")))]
        let browser_launcher: Option<Arc<dyn Fn() -> BrowserLaunchFn + Send + Sync>> = None;
        #[cfg(any(test, feature = "test-utils"))]
        let browser_launcher: Option<Arc<dyn Fn() -> BrowserLaunchFn + Send + Sync>> =
            Some(Arc::new(|| -> BrowserLaunchFn {
                Box::new(|_authorize_url, _redirect_uri| Box::pin(async {}))
            }));

        Ok(Self {
            username,
            client_id,
            client_secret: client_secret.into(),
            authorization_url,
            token_url,
            redirect_uri,
            scope,
            enable_single_use_refresh_tokens,
            disable_pkce,
            client_store_temporary_credential,
            flow_options: OAuthFlowOptions {
                enable_dpop,
                authentication_timeout_secs,
            },
            browser_launcher,
        })
    }
}

impl OAuthClientCredentialsConfig {
    pub(crate) fn from_settings(settings: &dyn Settings) -> Result<Self, ConfigError> {
        // CC is external-IdP only: Snowflake's GS does not issue
        // tokens for `grant_type=client_credentials` (analysis §4),
        // so client_id, client_secret and token_url are mandatory.
        let username = non_empty_string(settings, "user")
            .context(MissingParameterSnafu { parameter: "user" })?;
        let client_id =
            non_empty_string(settings, "oauth_client_id").context(MissingParameterSnafu {
                parameter: "oauth_client_id",
            })?;
        let client_secret =
            non_empty_string(settings, "oauth_client_secret").context(MissingParameterSnafu {
                parameter: "oauth_client_secret",
            })?;
        let token_url = parse_required_url(settings, "oauth_token_request_url")?;
        let scope = non_empty_string(settings, "oauth_scope");
        let enable_dpop = get_flexible_bool(settings, "oauth_enable_dpop");
        let authentication_timeout_secs = settings
            .get_u64("authentication_timeout")
            .unwrap_or(DEFAULT_AUTHENTICATION_TIMEOUT_SECS);

        Ok(Self {
            username,
            client_id,
            client_secret: client_secret.into(),
            token_url,
            scope,
            flow_options: OAuthFlowOptions {
                enable_dpop,
                authentication_timeout_secs,
            },
        })
    }
}

impl LoginMethod {
    /// Convert DER-encoded private key bytes to PEM format string
    fn der_to_pem(der_bytes: &[u8]) -> Result<String, ConfigError> {
        let pkey = PKey::private_key_from_der(der_bytes).map_err(|e| {
            InvalidParameterValueSnafu {
                parameter: "private_key",
                value: "(binary data)".to_string(),
                explanation: format!("Could not parse DER private key: {e}"),
            }
            .build()
        })?;

        let pem_bytes = pkey.private_key_to_pem_pkcs8().map_err(|e| {
            InvalidParameterValueSnafu {
                parameter: "private_key",
                value: "(binary data)".to_string(),
                explanation: format!("Could not convert private key to PEM: {e}"),
            }
            .build()
        })?;

        String::from_utf8(pem_bytes).map_err(|e| {
            InvalidParameterValueSnafu {
                parameter: "private_key",
                value: "(binary data)".to_string(),
                explanation: format!("PEM output is not valid UTF-8: {e}"),
            }
            .build()
        })
    }

    fn read_private_key(settings: &dyn Settings) -> Result<String, ConfigError> {
        let has_private_key = settings.get("private_key").is_some();
        let has_private_key_file = settings.get_string("private_key_file").is_some();

        // Validate that both are not set at the same time
        if has_private_key && has_private_key_file {
            return ConflictingParametersSnafu {
                explanation:
                    "Both 'private_key' and 'private_key_file' are set. Please provide only one."
                        .to_string(),
            }
            .fail();
        }

        // First, check if private_key is provided as bytes (DER format from Python)
        if let Some(Setting::Bytes(private_key_bytes)) = settings.get("private_key") {
            return Self::der_to_pem(&private_key_bytes);
        }

        // Check if private_key is provided as a string (base64-encoded)
        if let Some(private_key_base64) = settings.get_string("private_key") {
            use base64::{Engine as _, engine::general_purpose};
            let private_key_bytes = general_purpose::STANDARD
                .decode(&private_key_base64)
                .map_err(|e| {
                    InvalidParameterValueSnafu {
                        parameter: "private_key",
                        value: "(redacted)".to_string(),
                        explanation: format!("Could not decode base64 private key: {e}"),
                    }
                    .build()
                })?;

            // Check if it's PEM format (starts with "-----BEGIN")
            if private_key_bytes.starts_with(b"-----BEGIN") {
                let private_key = String::from_utf8(private_key_bytes).map_err(|e| {
                    InvalidParameterValueSnafu {
                        parameter: "private_key",
                        value: "(redacted)".to_string(),
                        explanation: format!("Private key is not valid UTF-8: {e}"),
                    }
                    .build()
                })?;
                return Ok(private_key);
            }

            // Otherwise, assume it's DER format and convert to PEM
            return Self::der_to_pem(&private_key_bytes);
        }
        if let Some(private_key_file) = settings.get_string("private_key_file") {
            let private_key = fs::read_to_string(private_key_file.clone()).map_err(|e| {
                InvalidParameterValueSnafu {
                    parameter: "private_key_file",
                    value: private_key_file,
                    explanation: format!("Could not read private key file: {e}"),
                }
                .build()
            })?;
            return Ok(private_key);
        }

        MissingParameterSnafu {
            parameter: "private_key or private_key_file",
        }
        .fail()?
    }

    /// Check if private key parameters are present in settings
    fn has_private_key_params(settings: &dyn Settings) -> bool {
        settings.get("private_key").is_some() || settings.get_string("private_key_file").is_some()
    }

    pub fn from_settings(settings: &dyn Settings) -> Result<Self, ConfigError> {
        let authenticator = settings.get_string("authenticator").unwrap_or_default();
        let auth_upper = authenticator.to_ascii_uppercase();

        // Auto-detect JWT authentication if private key params are present
        // and authenticator is not explicitly set to something else
        let use_jwt = auth_upper == "SNOWFLAKE_JWT"
            || (authenticator.is_empty() && Self::has_private_key_params(settings));

        if use_jwt {
            return Ok(Self::PrivateKey {
                username: non_empty_string(settings, "user")
                    .context(MissingParameterSnafu { parameter: "user" })?,
                private_key: Self::read_private_key(settings)?.into(),
                passphrase: settings
                    .get_string("private_key_password")
                    .map(SensitiveString::from),
            });
        }

        match auth_upper.as_str() {
            "SNOWFLAKE" | "SNOWFLAKE_PASSWORD" | "" => Ok(Self::Password {
                username: non_empty_string(settings, "user")
                    .context(MissingParameterSnafu { parameter: "user" })?,
                password: non_empty_string(settings, "password")
                    .context(MissingParameterSnafu {
                        parameter: "password",
                    })?
                    .into(),
            }),
            "PROGRAMMATIC_ACCESS_TOKEN" => Ok(Self::Pat {
                username: non_empty_string(settings, "user")
                    .context(MissingParameterSnafu { parameter: "user" })?,
                token: non_empty_string(settings, "token")
                    .context(MissingParameterSnafu { parameter: "token" })?
                    .into(),
            }),
            _ if auth_upper.starts_with("HTTPS://") => {
                // Native Okta SSO is configured by passing the Okta URL endpoint as `authenticator`.
                // This is intentionally broad (vanity domains may not contain "okta").
                // Validate the URL is well-formed early to provide a clear error message.
                let okta_url = Url::parse(&authenticator).map_err(|_| {
                    InvalidParameterValueSnafu {
                        parameter: "authenticator",
                        value: authenticator,
                        explanation: "The authenticator URL is not a valid URL",
                    }
                    .build()
                })?;

                let username = non_empty_string(settings, "user")
                    .context(MissingParameterSnafu { parameter: "user" })?;
                let okta_username = settings.get_string("okta_username");
                let password = settings
                    .get_string("password")
                    .context(MissingParameterSnafu {
                        parameter: "password",
                    })?;

                let disable_saml_url_check = settings
                    .get_string("disable_saml_url_check")
                    .map(|v| v.eq_ignore_ascii_case("true") || v == "1")
                    .or_else(|| settings.get_int("disable_saml_url_check").map(|v| v != 0))
                    .unwrap_or(false);

                let authentication_timeout_secs = settings
                    .get_u64("authentication_timeout")
                    .unwrap_or(DEFAULT_AUTHENTICATION_TIMEOUT_SECS);

                Ok(Self::NativeOkta(NativeOktaConfig {
                    username,
                    okta_username,
                    password: password.into(),
                    okta_url,
                    disable_saml_url_check,
                    authentication_timeout_secs,
                }))
            }
            "OAUTH" => Ok(Self::OAuthAccessToken {
                username: non_empty_string(settings, "user")
                    .context(MissingParameterSnafu { parameter: "user" })?,
                token: non_empty_string(settings, "token")
                    .context(MissingParameterSnafu { parameter: "token" })?
                    .into(),
            }),
            "OAUTH_AUTHORIZATION_CODE" => Ok(Self::OAuthAuthorizationCode(
                // Snowflake-as-IdP substitutes `LOCAL_APPLICATION` for
                // missing client_id/client_secret at flow time. Keep them
                // empty here and let the AC provider apply that default.
                OAuthAuthorizationCodeConfig::from_settings(settings)?,
            )),
            "OAUTH_CLIENT_CREDENTIALS" => Ok(Self::OAuthClientCredentials(
                // CC is external-IdP only: Snowflake does not issue
                // tokens for `grant_type=client_credentials`, so client_id,
                // client_secret and token_url are mandatory.
                OAuthClientCredentialsConfig::from_settings(settings)?,
            )),
            "USERNAME_PASSWORD_MFA" => Ok(Self::UserPasswordMfa {
                username: non_empty_string(settings, "user")
                    .context(MissingParameterSnafu { parameter: "user" })?,
                password: non_empty_string(settings, "password")
                    .context(MissingParameterSnafu {
                        parameter: "password",
                    })?
                    .into(),
                passcode_in_password: settings
                    .get_bool("passcodeInPassword")
                    .or_else(|| {
                        settings
                            .get_string("passcodeInPassword")
                            .map(|v| v.eq_ignore_ascii_case("true") || v == "1")
                    })
                    .or_else(|| settings.get_int("passcodeInPassword").map(|v| v != 0))
                    .unwrap_or(false),
                passcode: settings.get_string("passcode").map(SensitiveString::from),
                client_store_temporary_credential: settings
                    .get_bool("client_store_temporary_credential")
                    .or_else(|| {
                        settings
                            .get_string("client_store_temporary_credential")
                            .map(|v| v.eq_ignore_ascii_case("true") || v == "1")
                    })
                    .or_else(|| {
                        settings
                            .get_int("client_store_temporary_credential")
                            .map(|v| v != 0)
                    })
                    .unwrap_or(false),
            }),
            "EXTERNALBROWSER" => {
                let authentication_timeout_secs = settings
                    .get_u64("authentication_timeout")
                    .unwrap_or(DEFAULT_AUTHENTICATION_TIMEOUT_SECS);

                Ok(Self::ExternalBrowser {
                    username: non_empty_string(settings, "user")
                        .context(MissingParameterSnafu { parameter: "user" })?,
                    authentication_timeout_secs,
                })
            }
            _ => InvalidParameterValueSnafu {
                parameter: "authenticator",
                value: authenticator,
                explanation: crate::config::AUTHENTICATOR_ALLOWED_VALUES,
            }
            .fail()?,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::settings::Setting;
    use std::collections::HashMap;

    fn create_test_settings(options: Vec<(&str, Setting)>) -> HashMap<String, Setting> {
        options
            .into_iter()
            .map(|(k, v)| (k.to_string(), v))
            .collect()
    }

    #[test]
    fn test_conflicting_private_key_and_private_key_file_string() {
        // Both private_key (string) and private_key_file are set
        let settings = create_test_settings(vec![
            ("user", Setting::String("test_user".to_string())),
            (
                "authenticator",
                Setting::String("SNOWFLAKE_JWT".to_string()),
            ),
            (
                "private_key",
                Setting::String("some_base64_key".to_string()),
            ),
            (
                "private_key_file",
                Setting::String("/path/to/key.p8".to_string()),
            ),
        ]);

        let result = LoginMethod::from_settings(&settings);
        assert!(result.is_err());
        let err = result.unwrap_err();
        let err_msg = err.to_string();
        assert!(
            err_msg.contains("Conflicting parameters"),
            "Expected 'Conflicting parameters' error, got: {err_msg}"
        );
        assert!(
            err_msg.contains("private_key") && err_msg.contains("private_key_file"),
            "Error should mention both parameters: {err_msg}"
        );
    }

    #[test]
    fn test_conflicting_private_key_bytes_and_private_key_file() {
        // Both private_key (bytes) and private_key_file are set
        let settings = create_test_settings(vec![
            ("user", Setting::String("test_user".to_string())),
            (
                "authenticator",
                Setting::String("SNOWFLAKE_JWT".to_string()),
            ),
            ("private_key", Setting::Bytes(vec![0x30, 0x82])), // Some DER bytes
            (
                "private_key_file",
                Setting::String("/path/to/key.p8".to_string()),
            ),
        ]);

        let result = LoginMethod::from_settings(&settings);
        assert!(result.is_err());
        let err = result.unwrap_err();
        let err_msg = err.to_string();
        assert!(
            err_msg.contains("Conflicting parameters"),
            "Expected 'Conflicting parameters' error, got: {err_msg}"
        );
    }

    #[test]
    fn test_only_private_key_file_is_allowed() {
        // Only private_key_file is set (should not error on conflict check)
        // Note: This will fail because the file doesn't exist, but it should NOT
        // fail with "Conflicting parameters" error
        let settings = create_test_settings(vec![
            ("user", Setting::String("test_user".to_string())),
            (
                "authenticator",
                Setting::String("SNOWFLAKE_JWT".to_string()),
            ),
            (
                "private_key_file",
                Setting::String("/nonexistent/path/to/key.p8".to_string()),
            ),
        ]);

        let result = LoginMethod::from_settings(&settings);
        assert!(result.is_err());
        let err = result.unwrap_err();
        let err_msg = err.to_string();
        // Should fail because file doesn't exist, NOT because of conflicting params
        assert!(
            !err_msg.contains("Conflicting parameters"),
            "Should not be a conflicting parameters error: {err_msg}"
        );
        assert!(
            err_msg.contains("private_key_file") && err_msg.contains("Could not read"),
            "Should fail because file doesn't exist: {err_msg}"
        );
    }

    #[test]
    fn test_only_private_key_string_is_allowed() {
        // Only private_key (string) is set - should not fail with conflict error
        // Note: This will fail because of invalid base64/key format, but NOT conflict
        let settings = create_test_settings(vec![
            ("user", Setting::String("test_user".to_string())),
            (
                "authenticator",
                Setting::String("SNOWFLAKE_JWT".to_string()),
            ),
            (
                "private_key",
                Setting::String("!!!invalid_base64!!!".to_string()),
            ),
        ]);

        let result = LoginMethod::from_settings(&settings);
        assert!(result.is_err());
        let err = result.unwrap_err();
        let err_msg = err.to_string();
        // Should fail because of invalid base64, NOT because of conflicting params
        assert!(
            !err_msg.contains("Conflicting parameters"),
            "Should not be a conflicting parameters error: {err_msg}"
        );
    }

    #[test]
    fn test_auto_detect_jwt_does_not_conflict_check_when_no_private_key() {
        // No private key params - should fall back to password auth
        let settings = create_test_settings(vec![
            ("user", Setting::String("test_user".to_string())),
            ("password", Setting::String("test_password".to_string())),
        ]);

        let result = LoginMethod::from_settings(&settings);
        assert!(result.is_ok());
        match result.unwrap() {
            LoginMethod::Password { username, password } => {
                assert_eq!(username, "test_user");
                assert_eq!(password.reveal(), "test_password");
            }
            _ => panic!("Expected Password login method"),
        }
    }

    #[test]
    fn test_snowflake_lowercase_resolves_to_password() {
        let settings = create_test_settings(vec![
            ("user", Setting::String("test_user".to_string())),
            ("password", Setting::String("test_password".to_string())),
            ("authenticator", Setting::String("snowflake".to_string())),
        ]);

        let result = LoginMethod::from_settings(&settings);
        assert!(result.is_ok());
        match result.unwrap() {
            LoginMethod::Password { username, password } => {
                assert_eq!(username, "test_user");
                assert_eq!(password.reveal(), "test_password");
            }
            _ => panic!("Expected Password login method for 'snowflake'"),
        }
    }

    #[test]
    fn test_snowflake_mixed_case_resolves_to_password() {
        let settings = create_test_settings(vec![
            ("user", Setting::String("test_user".to_string())),
            ("password", Setting::String("test_password".to_string())),
            ("authenticator", Setting::String("Snowflake".to_string())),
        ]);

        let result = LoginMethod::from_settings(&settings);
        assert!(result.is_ok());
        match result.unwrap() {
            LoginMethod::Password { .. } => {}
            _ => panic!("Expected Password login method for 'Snowflake'"),
        }
    }

    #[test]
    fn test_pat_lowercase_resolves_to_pat() {
        let settings = create_test_settings(vec![
            ("user", Setting::String("test_user".to_string())),
            ("token", Setting::String("test_token".to_string())),
            (
                "authenticator",
                Setting::String("programmatic_access_token".to_string()),
            ),
        ]);

        let result = LoginMethod::from_settings(&settings);
        assert!(result.is_ok());
        match result.unwrap() {
            LoginMethod::Pat { username, token } => {
                assert_eq!(username, "test_user");
                assert_eq!(token.reveal(), "test_token");
            }
            _ => panic!("Expected Pat login method for lowercase 'programmatic_access_token'"),
        }
    }

    #[test]
    fn test_mfa_lowercase_resolves_to_mfa() {
        let settings = create_test_settings(vec![
            ("user", Setting::String("test_user".to_string())),
            ("password", Setting::String("test_password".to_string())),
            (
                "authenticator",
                Setting::String("username_password_mfa".to_string()),
            ),
        ]);

        let result = LoginMethod::from_settings(&settings);
        assert!(result.is_ok());
        match result.unwrap() {
            LoginMethod::UserPasswordMfa { .. } => {}
            _ => panic!("Expected UserPasswordMfa login method"),
        }
    }

    fn okta_config(extras: Vec<(&str, Setting)>) -> NativeOktaConfig {
        let mut base = vec![
            ("user", Setting::String("okta_user".to_string())),
            ("password", Setting::String("okta_pass".to_string())),
            (
                "host",
                Setting::String("account.snowflakecomputing.com".to_string()),
            ),
            ("account", Setting::String("account".to_string())),
            (
                "authenticator",
                Setting::String("https://myorg.okta.com".to_string()),
            ),
        ];
        base.extend(extras);
        let settings = create_test_settings(base);
        match LoginMethod::from_settings(&settings).unwrap() {
            LoginMethod::NativeOkta(cfg) => cfg,
            other => panic!("Expected NativeOkta, got {other:?}"),
        }
    }

    #[test]
    fn test_native_okta_uses_default_authentication_timeout() {
        let cfg = okta_config(vec![]);
        assert_eq!(
            cfg.authentication_timeout_secs,
            DEFAULT_AUTHENTICATION_TIMEOUT_SECS
        );
    }

    #[test]
    fn test_native_okta_custom_authentication_timeout() {
        let cfg = okta_config(vec![(
            "authentication_timeout",
            Setting::String("60".to_string()),
        )]);
        assert_eq!(cfg.authentication_timeout_secs, 60);
    }

    #[test]
    fn test_native_okta_invalid_authentication_timeout_uses_default() {
        let cfg = okta_config(vec![(
            "authentication_timeout",
            Setting::String("not_a_number".to_string()),
        )]);
        assert_eq!(
            cfg.authentication_timeout_secs, DEFAULT_AUTHENTICATION_TIMEOUT_SECS,
            "Invalid timeout should fall back to the default"
        );
    }

    #[test]
    fn test_native_okta_disable_saml_url_check_defaults_to_false() {
        let cfg = okta_config(vec![]);
        assert!(!cfg.disable_saml_url_check);
    }

    #[test]
    fn test_native_okta_disable_saml_url_check_true() {
        let cfg = okta_config(vec![(
            "disable_saml_url_check",
            Setting::String("true".to_string()),
        )]);
        assert!(cfg.disable_saml_url_check);
    }

    #[test]
    fn test_empty_user_returns_missing_parameter_error() {
        let settings = create_test_settings(vec![
            ("user", Setting::String("".to_string())),
            ("password", Setting::String("test_password".to_string())),
        ]);

        let result = LoginMethod::from_settings(&settings);
        assert!(result.is_err());
        let err_msg = result.unwrap_err().to_string();
        assert!(
            err_msg.contains("Missing required parameter") && err_msg.contains("user"),
            "Expected MissingParameter error for empty user, got: {err_msg}"
        );
    }

    #[test]
    fn test_client_info_uses_defaults_when_no_wrapper_settings() {
        let settings = create_test_settings(vec![(
            "host",
            Setting::String("test.snowflakecomputing.com".to_string()),
        )]);
        let info = ClientInfo::from_settings(&settings).unwrap();
        assert_eq!(info.client_app_id, env!("CARGO_PKG_NAME"));
        assert_eq!(info.application, env!("CARGO_PKG_NAME"));
        assert_eq!(info.version, env!("CARGO_PKG_VERSION"));
        assert!(info.runtime_name.is_none());
        assert!(info.runtime_version.is_none());
        assert!(info.compiler.is_none());
    }

    #[test]
    fn test_application_defaults_to_client_app_id_when_unset() {
        let settings = create_test_settings(vec![
            (
                "host",
                Setting::String("test.snowflakecomputing.com".to_string()),
            ),
            (
                "client_app_id",
                Setting::String("PythonConnector".to_string()),
            ),
        ]);
        let info = ClientInfo::from_settings(&settings).unwrap();
        assert_eq!(info.client_app_id, "PythonConnector");
        assert_eq!(info.application, "PythonConnector");
    }

    #[test]
    fn test_application_independent_of_client_app_id() {
        // Mirrors the old connector: internal_application_name → CLIENT_APP_ID,
        // application → CLIENT_ENVIRONMENT.APPLICATION. The two values are
        // independent.
        let settings = create_test_settings(vec![
            (
                "host",
                Setting::String("test.snowflakecomputing.com".to_string()),
            ),
            (
                "client_app_id",
                Setting::String("PythonConnector".to_string()),
            ),
            (
                "application",
                Setting::String("SNOWCLI.STAGE.COPY".to_string()),
            ),
        ]);
        let info = ClientInfo::from_settings(&settings).unwrap();
        assert_eq!(info.client_app_id, "PythonConnector");
        assert_eq!(info.application, "SNOWCLI.STAGE.COPY");
    }

    #[test]
    fn test_application_empty_falls_back_to_client_app_id() {
        let settings = create_test_settings(vec![
            (
                "host",
                Setting::String("test.snowflakecomputing.com".to_string()),
            ),
            ("client_app_id", Setting::String("JDBC".to_string())),
            ("application", Setting::String("   ".to_string())),
        ]);
        let info = ClientInfo::from_settings(&settings).unwrap();
        assert_eq!(info.client_app_id, "JDBC");
        assert_eq!(info.application, "JDBC");
    }

    #[test]
    fn test_client_info_with_custom_wrapper_identity() {
        let settings = create_test_settings(vec![
            (
                "host",
                Setting::String("test.snowflakecomputing.com".to_string()),
            ),
            ("client_app_id", Setting::String("JDBC".to_string())),
            ("client_app_version", Setting::String("3.21.0".to_string())),
            (
                "client_runtime_name",
                Setting::String("OpenJDK".to_string()),
            ),
            (
                "client_runtime_version",
                Setting::String("21.0.1".to_string()),
            ),
            (
                "client_compiler",
                Setting::String("javac 21.0.1".to_string()),
            ),
        ]);
        let info = ClientInfo::from_settings(&settings).unwrap();
        assert_eq!(info.client_app_id, "JDBC");
        assert_eq!(info.application, "JDBC");
        assert_eq!(info.version, "3.21.0");
        assert_eq!(info.runtime_name.as_deref(), Some("OpenJDK"));
        assert_eq!(info.runtime_version.as_deref(), Some("21.0.1"));
        assert_eq!(info.compiler.as_deref(), Some("javac 21.0.1"));
    }

    #[test]
    fn test_client_info_empty_strings_become_none() {
        let settings = create_test_settings(vec![
            (
                "host",
                Setting::String("test.snowflakecomputing.com".to_string()),
            ),
            ("client_runtime_name", Setting::String("".to_string())),
            ("client_runtime_version", Setting::String("  ".to_string())),
            ("client_compiler", Setting::String(" \t ".to_string())),
        ]);
        let info = ClientInfo::from_settings(&settings).unwrap();
        assert!(
            info.runtime_name.is_none(),
            "empty string should become None"
        );
        assert!(
            info.runtime_version.is_none(),
            "whitespace-only should become None"
        );
        assert!(info.compiler.is_none(), "whitespace+tab should become None");
    }

    #[test]
    fn test_oauth_legacy_authenticator_resolves_to_oauth_access_token() {
        let settings = create_test_settings(vec![
            ("user", Setting::String("alice".to_string())),
            ("token", Setting::String("legacy-bearer".to_string())),
            ("authenticator", Setting::String("OAUTH".to_string())),
        ]);
        match LoginMethod::from_settings(&settings).unwrap() {
            LoginMethod::OAuthAccessToken { username, token } => {
                assert_eq!(username, "alice");
                assert_eq!(token.reveal(), "legacy-bearer");
            }
            other => panic!("Expected OAuthAccessToken, got {other:?}"),
        }
    }

    #[test]
    fn test_oauth_legacy_requires_token() {
        let settings = create_test_settings(vec![
            ("user", Setting::String("alice".to_string())),
            ("authenticator", Setting::String("oauth".to_string())),
        ]);
        let err = LoginMethod::from_settings(&settings).unwrap_err();
        let msg = err.to_string();
        assert!(
            msg.contains("Missing required parameter") && msg.contains("token"),
            "Expected missing-token error, got: {msg}"
        );
    }

    #[test]
    fn test_oauth_authorization_code_authenticator_parses_config() {
        let settings = create_test_settings(vec![
            ("user", Setting::String("alice".to_string())),
            (
                "authenticator",
                Setting::String("OAUTH_AUTHORIZATION_CODE".to_string()),
            ),
            ("oauth_client_id", Setting::String("client-x".to_string())),
            (
                "oauth_client_secret",
                Setting::String("super-secret".to_string()),
            ),
            (
                "oauth_authorization_url",
                Setting::String("https://idp.example.com/oauth/authorize".to_string()),
            ),
            (
                "oauth_token_request_url",
                Setting::String("https://idp.example.com/oauth/token".to_string()),
            ),
            (
                "oauth_redirect_uri",
                Setting::String("http://127.0.0.1:9090/".to_string()),
            ),
            (
                "oauth_scope",
                Setting::String("session:role:DEV".to_string()),
            ),
            (
                "oauth_enable_single_use_refresh_tokens",
                Setting::Bool(true),
            ),
            ("oauth_disable_pkce", Setting::Bool(false)),
            ("oauth_enable_dpop", Setting::Bool(true)),
            ("client_store_temporary_credential", Setting::Bool(true)),
            ("authentication_timeout", Setting::Int(45)),
        ]);
        match LoginMethod::from_settings(&settings).unwrap() {
            LoginMethod::OAuthAuthorizationCode(cfg) => {
                assert_eq!(cfg.username, "alice");
                assert_eq!(cfg.client_id, "client-x");
                assert_eq!(cfg.client_secret.reveal(), "super-secret");
                assert_eq!(
                    cfg.authorization_url.as_ref().map(Url::as_str),
                    Some("https://idp.example.com/oauth/authorize")
                );
                assert_eq!(
                    cfg.token_url.as_ref().map(Url::as_str),
                    Some("https://idp.example.com/oauth/token")
                );
                assert_eq!(
                    cfg.redirect_uri.as_ref().map(Url::as_str),
                    Some("http://127.0.0.1:9090/")
                );
                assert_eq!(cfg.scope.as_deref(), Some("session:role:DEV"));
                assert!(cfg.enable_single_use_refresh_tokens);
                assert!(!cfg.disable_pkce);
                assert!(cfg.flow_options.enable_dpop);
                assert!(cfg.client_store_temporary_credential);
                assert_eq!(cfg.flow_options.authentication_timeout_secs, 45);
            }
            other => panic!("Expected OAuthAuthorizationCode, got {other:?}"),
        }
    }

    #[test]
    fn test_oauth_authorization_code_minimal_uses_defaults() {
        // Bare authenticator + user; ensures the Snowflake-as-IdP wiring
        // path can still construct the config with default URLs / empty
        // client credentials (LOCAL_APPLICATION substitution happens at
        // flow time when Snowflake is the IdP).
        let settings = create_test_settings(vec![
            ("user", Setting::String("alice".to_string())),
            (
                "authenticator",
                Setting::String("oauth_authorization_code".to_string()),
            ),
        ]);
        match LoginMethod::from_settings(&settings).unwrap() {
            LoginMethod::OAuthAuthorizationCode(cfg) => {
                assert!(cfg.client_id.is_empty(), "client_id default is empty");
                assert!(cfg.client_secret.reveal().is_empty());
                assert!(cfg.authorization_url.is_none());
                assert!(cfg.token_url.is_none());
                assert!(cfg.redirect_uri.is_none());
                assert!(cfg.scope.is_none());
                assert!(!cfg.enable_single_use_refresh_tokens);
                assert!(!cfg.disable_pkce);
                assert!(!cfg.flow_options.enable_dpop);
                assert!(!cfg.client_store_temporary_credential);
                assert_eq!(
                    cfg.flow_options.authentication_timeout_secs,
                    DEFAULT_AUTHENTICATION_TIMEOUT_SECS
                );
            }
            other => panic!("Expected OAuthAuthorizationCode, got {other:?}"),
        }
    }

    #[test]
    fn test_oauth_client_credentials_requires_token_url() {
        let settings = create_test_settings(vec![
            ("user", Setting::String("alice".to_string())),
            (
                "authenticator",
                Setting::String("OAUTH_CLIENT_CREDENTIALS".to_string()),
            ),
            ("oauth_client_id", Setting::String("cid".to_string())),
            ("oauth_client_secret", Setting::String("sss".to_string())),
            // oauth_token_request_url intentionally omitted
        ]);
        let err = LoginMethod::from_settings(&settings).unwrap_err();
        let msg = err.to_string();
        assert!(
            msg.contains("Missing required parameter") && msg.contains("oauth_token_request_url"),
            "Expected missing token URL error, got: {msg}"
        );
    }

    #[test]
    fn test_oauth_client_credentials_authenticator_parses_config() {
        let settings = create_test_settings(vec![
            ("user", Setting::String("svc-account".to_string())),
            (
                "authenticator",
                Setting::String("oauth_client_credentials".to_string()),
            ),
            ("oauth_client_id", Setting::String("cid".to_string())),
            ("oauth_client_secret", Setting::String("sss".to_string())),
            (
                "oauth_token_request_url",
                Setting::String("https://idp.example.com/oauth/token".to_string()),
            ),
            (
                "oauth_scope",
                Setting::String("session:role:READER".to_string()),
            ),
            ("oauth_enable_dpop", Setting::String("true".to_string())),
        ]);
        match LoginMethod::from_settings(&settings).unwrap() {
            LoginMethod::OAuthClientCredentials(cfg) => {
                assert_eq!(cfg.username, "svc-account");
                assert_eq!(cfg.client_id, "cid");
                assert_eq!(cfg.client_secret.reveal(), "sss");
                assert_eq!(
                    cfg.token_url.as_str(),
                    "https://idp.example.com/oauth/token"
                );
                assert_eq!(cfg.scope.as_deref(), Some("session:role:READER"));
                assert!(
                    cfg.flow_options.enable_dpop,
                    "string \"true\" should resolve to true"
                );
                assert_eq!(
                    cfg.flow_options.authentication_timeout_secs,
                    DEFAULT_AUTHENTICATION_TIMEOUT_SECS
                );
            }
            other => panic!("Expected OAuthClientCredentials, got {other:?}"),
        }
    }

    #[test]
    fn test_oauth_authenticator_with_invalid_url_fails_fast() {
        let settings = create_test_settings(vec![
            ("user", Setting::String("alice".to_string())),
            (
                "authenticator",
                Setting::String("OAUTH_AUTHORIZATION_CODE".to_string()),
            ),
            (
                "oauth_authorization_url",
                Setting::String("not a url".to_string()),
            ),
        ]);
        let err = LoginMethod::from_settings(&settings).unwrap_err();
        let msg = err.to_string();
        assert!(
            msg.contains("oauth_authorization_url") && msg.contains("Could not parse URL"),
            "Expected URL parse error, got: {msg}"
        );
    }

    #[test]
    fn test_oauth_disable_pkce_defaults_false_when_setting_omitted_with_real_idp_params() {
        // `test_oauth_authorization_code_minimal_uses_defaults` covers the
        // bare-minimum case (Snowflake-as-IdP). This sibling test pins
        // the same default with a fully-populated external-IdP config so
        // we don't regress the cross-driver default (PKCE is on for
        // everybody except Python's opt-out, and we are not Python).
        let settings = create_test_settings(vec![
            ("user", Setting::String("alice".to_string())),
            (
                "authenticator",
                Setting::String("OAUTH_AUTHORIZATION_CODE".to_string()),
            ),
            ("oauth_client_id", Setting::String("cid".to_string())),
            (
                "oauth_client_secret",
                Setting::String("super-secret".to_string()),
            ),
            (
                "oauth_authorization_url",
                Setting::String("https://idp.example.com/oauth/authorize".to_string()),
            ),
            (
                "oauth_token_request_url",
                Setting::String("https://idp.example.com/oauth/token".to_string()),
            ),
            // NOTE: oauth_disable_pkce intentionally omitted.
        ]);
        match LoginMethod::from_settings(&settings).unwrap() {
            LoginMethod::OAuthAuthorizationCode(cfg) => {
                assert!(
                    !cfg.disable_pkce,
                    "disable_pkce must default to false when the setting is omitted"
                );
            }
            other => panic!("Expected OAuthAuthorizationCode, got {other:?}"),
        }
    }

    #[test]
    fn test_detect_os_version_not_empty() {
        let version = crate::telemetry::environment::detect_os_version();
        assert!(!version.is_empty());
    }

    // ─── External Browser config tests ───────────────────────────────────

    fn external_browser_config(extras: Vec<(&str, Setting)>) -> (String, u64) {
        let mut base = vec![
            ("user", Setting::String("browser_user".to_string())),
            (
                "host",
                Setting::String("account.snowflakecomputing.com".to_string()),
            ),
            ("account", Setting::String("account".to_string())),
            (
                "authenticator",
                Setting::String("EXTERNALBROWSER".to_string()),
            ),
        ];
        base.extend(extras);
        let settings = create_test_settings(base);
        match LoginMethod::from_settings(&settings).unwrap() {
            LoginMethod::ExternalBrowser {
                username,
                authentication_timeout_secs,
            } => (username, authentication_timeout_secs),
            other => panic!("Expected ExternalBrowser, got {other:?}"),
        }
    }

    #[test]
    fn test_external_browser_happy_path() {
        let (user, timeout) = external_browser_config(vec![]);
        assert_eq!(user, "browser_user");
        assert_eq!(timeout, DEFAULT_AUTHENTICATION_TIMEOUT_SECS);
    }

    #[test]
    fn test_external_browser_custom_timeout() {
        let (_, timeout) = external_browser_config(vec![(
            "authentication_timeout",
            Setting::String("30".to_string()),
        )]);
        assert_eq!(timeout, 30);
    }

    #[test]
    fn test_external_browser_invalid_timeout_uses_default() {
        let (_, timeout) = external_browser_config(vec![(
            "authentication_timeout",
            Setting::String("abc".to_string()),
        )]);
        assert_eq!(timeout, DEFAULT_AUTHENTICATION_TIMEOUT_SECS);
    }

    #[test]
    fn test_external_browser_missing_user_fails() {
        let settings = create_test_settings(vec![
            (
                "host",
                Setting::String("account.snowflakecomputing.com".to_string()),
            ),
            ("account", Setting::String("account".to_string())),
            (
                "authenticator",
                Setting::String("EXTERNALBROWSER".to_string()),
            ),
        ]);
        let result = LoginMethod::from_settings(&settings);
        assert!(result.is_err());
        let err_msg = result.unwrap_err().to_string();
        assert!(
            err_msg.contains("user"),
            "Expected error about missing user, got: {err_msg}"
        );
    }

    // ── log_query_text / log_query_parameters resolvers ──────────────

    #[test]
    fn test_resolve_log_query_text_default_false() {
        let settings = create_test_settings(vec![]);
        assert!(!resolve_log_query_text(&settings));
        assert!(!resolve_log_query_parameters(&settings));
    }

    #[test]
    fn test_resolve_log_query_text_from_bool() {
        let settings = create_test_settings(vec![("log_query_text", Setting::Bool(true))]);
        assert!(resolve_log_query_text(&settings));
    }

    #[test]
    fn test_resolve_log_query_text_from_string_true() {
        let settings =
            create_test_settings(vec![("log_query_text", Setting::String("true".into()))]);
        assert!(resolve_log_query_text(&settings));
    }

    #[test]
    fn test_resolve_log_query_text_from_string_uppercase() {
        let settings =
            create_test_settings(vec![("log_query_text", Setting::String("TRUE".into()))]);
        assert!(resolve_log_query_text(&settings));
    }

    #[test]
    fn test_resolve_log_query_text_from_string_one() {
        let settings = create_test_settings(vec![("log_query_text", Setting::String("1".into()))]);
        assert!(resolve_log_query_text(&settings));
    }

    #[test]
    fn test_resolve_log_query_text_from_string_false() {
        let settings =
            create_test_settings(vec![("log_query_text", Setting::String("false".into()))]);
        assert!(!resolve_log_query_text(&settings));
    }

    #[test]
    fn test_resolve_log_query_text_from_int_one() {
        let settings = create_test_settings(vec![("log_query_text", Setting::Int(1))]);
        assert!(resolve_log_query_text(&settings));
    }

    #[test]
    fn test_resolve_log_query_text_from_int_zero() {
        let settings = create_test_settings(vec![("log_query_text", Setting::Int(0))]);
        assert!(!resolve_log_query_text(&settings));
    }

    #[test]
    fn test_resolve_log_query_parameters_independent_of_text_flag() {
        // The resolver itself just reads the boolean; the text-flag gating is
        // enforced by `query_log_fields`, not by the resolver.
        let settings = create_test_settings(vec![("log_query_parameters", Setting::Bool(true))]);
        assert!(resolve_log_query_parameters(&settings));
        assert!(!resolve_log_query_text(&settings));
    }

    #[test]
    fn test_query_parameters_from_settings_populates_new_flags() {
        let settings = create_test_settings(vec![
            (
                "host",
                Setting::String("test.snowflakecomputing.com".to_string()),
            ),
            ("log_query_text", Setting::Bool(true)),
            ("log_query_parameters", Setting::String("1".into())),
        ]);
        let params = QueryParameters::from_settings(&settings).unwrap();
        assert!(params.log_query_text);
        assert!(params.log_query_parameters);
    }
}
