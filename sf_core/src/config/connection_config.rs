use std::collections::HashMap;
use std::fmt;
use std::fs;
use std::path::PathBuf;

use base64::{Engine as _, engine::general_purpose};
use openssl::pkey::PKey;
use snafu::OptionExt;
use url::Url;

use crate::config::ParamStore;
use crate::config::param_names::*;
use crate::config::rest_parameters::{
    ClientInfo, DEFAULT_AUTHENTICATION_TIMEOUT_SECS, LoginMethod, LoginParameters,
    NativeOktaConfig, OAuthAuthorizationCodeConfig, OAuthClientCredentialsConfig, OAuthFlowOptions,
    WifProvider, WorkloadIdentityConfig,
};
use crate::config::settings::{Setting, Settings};
use crate::config::{
    ConfigError, ConflictingParametersSnafu, InvalidParameterValueSnafu, MissingParameterSnafu,
    ValidationSnafu,
};
use crate::sensitive::SensitiveString;
use crate::tls::config::{ProxyConfig, TlsConfig, TlsVersion};

// ---------------------------------------------------------------------------
// Typed config structs
// ---------------------------------------------------------------------------

/// Fully validated, typed connection configuration.
#[derive(Debug)]
pub struct ConnectionConfig {
    pub server: ServerConfig,
    pub auth: AuthConfig,
    pub session: SessionContext,
    pub tls: TlsConfig,
    pub proxy: ProxyConfig,
    pub disable_parallel_user_prompt: bool,
    pub diagnostic: DiagnosticConfig,
}

/// Configuration for SnowCD-style connectivity diagnostics.
#[derive(Debug, Clone, Default)]
pub enum DiagnosticConfig {
    #[default]
    Disabled,
    Enabled {
        /// Directory where the diagnostic report file is written.
        log_path: Option<PathBuf>,
        /// Path to a pre-fetched `allowlist.json`; if absent the driver fetches
        /// the allowlist live via `system$allowlist()`.
        allowlist_path: Option<PathBuf>,
    },
}

#[derive(Debug)]
pub struct ServerConfig {
    pub account: String,
    pub server_url: String,
}

#[derive(Debug)]
pub enum AuthConfig {
    Password {
        user: String,
        password: SensitiveString,
        passcode_in_password: bool,
        passcode: Option<SensitiveString>,
    },
    Mfa {
        user: String,
        password: SensitiveString,
        passcode_in_password: bool,
        passcode: Option<SensitiveString>,
        client_store_temporary_credential: bool,
    },
    Jwt {
        user: String,
        private_key_pem: SensitiveString,
        passphrase: Option<SensitiveString>,
    },
    Pat {
        user: String,
        token: SensitiveString,
    },
    NativeOkta(NativeOktaConfig),
    ExternalBrowser {
        user: String,
        authentication_timeout_secs: u64,
        client_store_temporary_credential: bool,
    },
    /// Legacy pre-acquired OAuth access token (`AUTHENTICATOR=OAUTH` +
    /// raw `token=`). Forwarded unchanged to Snowflake
    /// (`AUTHENTICATOR=OAUTH`, `TOKEN=<access_token>`, no `OAUTH_TYPE`).
    OAuthAccessToken {
        user: String,
        token: SensitiveString,
    },
    /// OAuth 2.0 Authorization Code (with PKCE) flow.
    OAuthAuthorizationCode(Box<OAuthAuthorizationCodeConfig>),
    /// OAuth 2.0 Client Credentials flow, external IdP only.
    OAuthClientCredentials(OAuthClientCredentialsConfig),
    /// Pre-acquired session token + master token pair, bypassing normal login.
    SessionToken {
        session_token: SensitiveString,
        master_token: SensitiveString,
        master_validity_in_seconds: Option<u64>,
    },
    /// Workload Identity Federation. The driver fetches an identity token from
    /// the cloud provider's metadata service and presents it to GS under
    /// `AUTHENTICATOR=WORKLOAD_IDENTITY`.
    WorkloadIdentity(WorkloadIdentityConfig),
}

#[derive(Debug)]
pub struct SessionContext {
    pub database: Option<String>,
    pub schema: Option<String>,
    pub warehouse: Option<String>,
    pub role: Option<String>,
}

// ---------------------------------------------------------------------------
// Validation types — canonical definitions, re-exported by apis::validation
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ValidationSeverity {
    Error,
    Warning,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ValidationCode {
    Unspecified,
    MissingRequired,
    InvalidType,
    InvalidValue,
    UnknownParameter,
    DeprecatedParameter,
    ConflictingParameters,
    ConflictingWifParameters,
}

#[derive(Debug, Clone)]
pub struct ValidationIssue {
    pub severity: ValidationSeverity,
    pub parameter: String,
    pub message: String,
    pub code: ValidationCode,
}

impl fmt::Display for ValidationIssue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "[{:?}] {}: {}",
            self.severity, self.parameter, self.message
        )
    }
}

// ---------------------------------------------------------------------------
// Private key helpers (mirrored from rest_parameters.rs)
// ---------------------------------------------------------------------------

fn der_to_pem(der_bytes: &[u8]) -> Result<String, ConfigError> {
    let pkey = PKey::private_key_from_der(der_bytes).map_err(|e| {
        InvalidParameterValueSnafu {
            parameter: String::from(PRIVATE_KEY),
            value: "(binary data)".to_string(),
            explanation: format!("Could not parse DER private key: {e}"),
        }
        .build()
    })?;

    let pem_bytes = pkey.private_key_to_pem_pkcs8().map_err(|e| {
        InvalidParameterValueSnafu {
            parameter: String::from(PRIVATE_KEY),
            value: "(binary data)".to_string(),
            explanation: format!("Could not convert private key to PEM: {e}"),
        }
        .build()
    })?;

    String::from_utf8(pem_bytes).map_err(|e| {
        InvalidParameterValueSnafu {
            parameter: String::from(PRIVATE_KEY),
            value: "(binary data)".to_string(),
            explanation: format!("PEM output is not valid UTF-8: {e}"),
        }
        .build()
    })
}

fn read_private_key(settings: &ParamStore) -> Result<String, ConfigError> {
    let has_private_key = settings.get(PRIVATE_KEY).is_some();
    let has_private_key_file = settings.get_string(PRIVATE_KEY_FILE).is_some();

    if has_private_key && has_private_key_file {
        return ConflictingParametersSnafu {
            explanation:
                "Both 'private_key' and 'private_key_file' are set. Please provide only one."
                    .to_string(),
        }
        .fail();
    }

    // Bytes (DER from Python)
    if let Some(Setting::Bytes(private_key_bytes)) = settings.get(PRIVATE_KEY) {
        return der_to_pem(private_key_bytes);
    }

    // String (base64-encoded)
    if let Some(private_key_base64) = settings.get_string(PRIVATE_KEY) {
        let private_key_bytes = general_purpose::STANDARD
            .decode(&private_key_base64)
            .map_err(|e| {
                InvalidParameterValueSnafu {
                    parameter: String::from(PRIVATE_KEY),
                    value: "(redacted)".to_string(),
                    explanation: format!("Could not decode base64 private key: {e}"),
                }
                .build()
            })?;

        if private_key_bytes.starts_with(b"-----BEGIN") {
            return String::from_utf8(private_key_bytes).map_err(|e| {
                InvalidParameterValueSnafu {
                    parameter: String::from(PRIVATE_KEY),
                    value: "(redacted)".to_string(),
                    explanation: format!("Private key is not valid UTF-8: {e}"),
                }
                .build()
            });
        }

        return der_to_pem(&private_key_bytes);
    }

    // File path
    if let Some(private_key_file) = settings.get_string(PRIVATE_KEY_FILE) {
        let private_key = fs::read_to_string(&private_key_file).map_err(|e| {
            InvalidParameterValueSnafu {
                parameter: String::from(PRIVATE_KEY_FILE),
                value: private_key_file,
                explanation: format!("Could not read private key file: {e}"),
            }
            .build()
        })?;
        return Ok(private_key);
    }

    MissingParameterSnafu {
        parameter: "private_key or private_key_file".to_string(),
    }
    .fail()
}

fn has_private_key_params(settings: &ParamStore) -> bool {
    settings.get(PRIVATE_KEY).is_some() || settings.get_string(PRIVATE_KEY_FILE).is_some()
}

fn non_empty_string(settings: &ParamStore, key: crate::config::ParamKey) -> Option<String> {
    settings.get_string(key).filter(|value| !value.is_empty())
}

/// Characters permitted in a Snowflake account identifier: ASCII alphanumerics
/// plus `.`, `-`, and `_` (underscores are normalized to hyphens in the derived
/// host). Used by `validate_settings` to reject account values carrying
/// URL-significant characters before any host is derived (SNOW-3663586,
/// CWE-918).
fn is_allowed_account_char(c: char) -> bool {
    c.is_ascii_alphanumeric() || matches!(c, '.' | '-' | '_')
}

// ---------------------------------------------------------------------------
// Server URL derivation (mirrored from rest_parameters::get_server_url)
// ---------------------------------------------------------------------------

/// Resolve the effective protocol from settings.
///
/// `ssl` takes precedence when present because it has no registry default
/// and therefore is always an explicit user choice.  `protocol` serves as
/// the fallback (the fallback `"https"` is hardcoded here).
fn resolve_protocol(settings: &ParamStore) -> String {
    if let Some(ssl_on) = settings.get_bool(SSL) {
        return if ssl_on { "https" } else { "http" }.to_string();
    }
    settings
        .get_string(PROTOCOL)
        .unwrap_or_else(|| "https".to_string())
}

fn derive_server_url(settings: &ParamStore) -> Result<String, ConfigError> {
    if let Some(url) = settings.get_string(SERVER_URL) {
        return Ok(url);
    }

    let protocol = resolve_protocol(settings);
    let host = settings.get_string(HOST).context(MissingParameterSnafu {
        parameter: String::from(HOST),
    })?;
    if protocol != "https" && protocol != "http" {
        tracing::warn!("Unexpected protocol specified during server url construction: {protocol}");
    }

    let base_url = format!("{protocol}://{host}");
    if let Some(port) = settings.get_int(PORT) {
        return Ok(format!("{base_url}:{port}"));
    }

    Ok(base_url)
}

// ---------------------------------------------------------------------------
// TLS / CRL config building
// ---------------------------------------------------------------------------

/// Build the TLS config from settings.
///
/// Delegates to the single canonical parser [`TlsConfig::from_settings`]
/// (which itself parses the CRL config) rather than re-deriving each field
/// here, so a new TLS field is added in exactly one place and can't be
/// silently dropped on this path (design-discipline rule 2).
fn build_tls_config(settings: &ParamStore) -> Result<TlsConfig, ConfigError> {
    TlsConfig::from_settings(settings)
}

/// Thin wrapper around [`ProxyConfig::from_settings`].
fn build_proxy_config(settings: &ParamStore) -> ProxyConfig {
    ProxyConfig::from_settings(settings)
}

// ---------------------------------------------------------------------------
// Auth config building (mirrored from rest_parameters::LoginMethod)
// ---------------------------------------------------------------------------

fn parse_authentication_timeout(settings: &ParamStore) -> u64 {
    settings
        .get_int(AUTHENTICATION_TIMEOUT)
        .and_then(|v| u64::try_from(v).ok())
        .unwrap_or(DEFAULT_AUTHENTICATION_TIMEOUT_SECS)
}

fn build_auth_config(settings: &ParamStore) -> Result<AuthConfig, ConfigError> {
    // Session token auth takes precedence over all other authenticators.
    if let Some(session_token) = non_empty_string(settings, SESSION_TOKEN) {
        let master_token =
            non_empty_string(settings, MASTER_TOKEN).context(MissingParameterSnafu {
                parameter: String::from(MASTER_TOKEN),
            })?;
        return Ok(AuthConfig::SessionToken {
            session_token: session_token.into(),
            master_token: master_token.into(),
            master_validity_in_seconds: settings.get_u64(MASTER_VALIDITY_IN_SECONDS.as_str()),
        });
    }

    let authenticator = settings.get_string(AUTHENTICATOR).unwrap_or_default();
    let auth_upper = authenticator.to_ascii_uppercase();

    let use_jwt = auth_upper == "SNOWFLAKE_JWT"
        || (authenticator.is_empty() && has_private_key_params(settings));

    if use_jwt {
        return Ok(AuthConfig::Jwt {
            user: non_empty_string(settings, USER).context(MissingParameterSnafu {
                parameter: String::from(USER),
            })?,
            private_key_pem: SensitiveString::from(read_private_key(settings)?),
            passphrase: settings.get_sensitive_string(PRIVATE_KEY_PASSWORD),
        });
    }

    match auth_upper.as_str() {
        "SNOWFLAKE" | "SNOWFLAKE_PASSWORD" | "" => Ok(AuthConfig::Password {
            user: non_empty_string(settings, USER).context(MissingParameterSnafu {
                parameter: String::from(USER),
            })?,
            password: settings
                .get_sensitive_string(PASSWORD)
                .filter(|s| !s.reveal().is_empty())
                .context(MissingParameterSnafu {
                    parameter: String::from(PASSWORD),
                })?,
            passcode_in_password: settings.get_bool(PASSCODE_IN_PASSWORD).unwrap_or(false),
            passcode: settings.get_sensitive_string(PASSCODE),
        }),
        "USERNAME_PASSWORD_MFA" => Ok(AuthConfig::Mfa {
            user: non_empty_string(settings, USER).context(MissingParameterSnafu {
                parameter: String::from(USER),
            })?,
            password: settings
                .get_sensitive_string(PASSWORD)
                .filter(|s| !s.reveal().is_empty())
                .context(MissingParameterSnafu {
                    parameter: String::from(PASSWORD),
                })?,
            passcode_in_password: settings.get_bool(PASSCODE_IN_PASSWORD).unwrap_or(false),
            passcode: settings.get_sensitive_string(PASSCODE),
            client_store_temporary_credential: settings
                .get_bool(CLIENT_STORE_TEMPORARY_CREDENTIAL)
                .unwrap_or(false),
        }),
        "PROGRAMMATIC_ACCESS_TOKEN" => Ok(AuthConfig::Pat {
            // SNOW-3647715: `user` optional — PAT encodes the principal.
            user: non_empty_string(settings, USER).unwrap_or_default(),
            token: settings
                .get_sensitive_string(TOKEN)
                .context(MissingParameterSnafu {
                    parameter: String::from(TOKEN),
                })?,
        }),
        // ─── OAuth: legacy pre-acquired access token ─────────────────────
        // `AUTHENTICATOR=OAUTH` + raw `token=`. Forwarded unchanged to
        // Snowflake; LOGIN_NAME is always set (cross-driver consensus:
        // JDBC/Go/Python set username; .NET's empty-string quirk is not ported).
        "OAUTH" => Ok(AuthConfig::OAuthAccessToken {
            // SNOW-3647715: `user` optional — the token's claims identify
            // the Snowflake principal.
            user: non_empty_string(settings, USER).unwrap_or_default(),
            token: settings
                .get_sensitive_string(TOKEN)
                .context(MissingParameterSnafu {
                    parameter: String::from(TOKEN),
                })?,
        }),
        // ─── OAuth: Authorization Code (with PKCE) ───────────────────────
        // Snowflake-as-IdP defaults (LOCAL_APPLICATION substitution +
        // default endpoints) are applied at flow time.
        "OAUTH_AUTHORIZATION_CODE" => Ok(AuthConfig::OAuthAuthorizationCode(Box::new(
            OAuthAuthorizationCodeConfig::from_settings(settings)?,
        ))),
        // ─── OAuth: Client Credentials (external IdP only) ───────────────
        // client_id/client_secret/token_url are mandatory because
        // Snowflake's GS does not issue tokens for
        // grant_type=client_credentials.
        "OAUTH_CLIENT_CREDENTIALS" => Ok(AuthConfig::OAuthClientCredentials(
            OAuthClientCredentialsConfig::from_settings(settings)?,
        )),
        _ if auth_upper.starts_with("HTTPS://") => {
            let okta_url = Url::parse(&authenticator).map_err(|_| {
                InvalidParameterValueSnafu {
                    parameter: String::from(AUTHENTICATOR),
                    value: authenticator.clone(),
                    explanation: "The authenticator URL is not a valid URL".to_string(),
                }
                .build()
            })?;

            Ok(AuthConfig::NativeOkta(NativeOktaConfig {
                username: non_empty_string(settings, USER).context(MissingParameterSnafu {
                    parameter: String::from(USER),
                })?,
                okta_username: settings.get_string(OKTA_USERNAME),
                password: settings.get_sensitive_string(PASSWORD).context(
                    MissingParameterSnafu {
                        parameter: String::from(PASSWORD),
                    },
                )?,
                okta_url,
                disable_saml_url_check: settings.get_bool(DISABLE_SAML_URL_CHECK).unwrap_or(false),
                authentication_timeout_secs: parse_authentication_timeout(settings),
            }))
        }
        "EXTERNALBROWSER" => Ok(AuthConfig::ExternalBrowser {
            user: non_empty_string(settings, USER).context(MissingParameterSnafu {
                parameter: String::from(USER),
            })?,
            authentication_timeout_secs: parse_authentication_timeout(settings),
            client_store_temporary_credential: settings
                .get_bool(CLIENT_STORE_TEMPORARY_CREDENTIAL)
                .unwrap_or(false),
        }),
        "WORKLOAD_IDENTITY" => {
            let provider_str = settings
                .get_string(WORKLOAD_IDENTITY_PROVIDER)
                .filter(|s| !s.is_empty())
                .context(MissingParameterSnafu {
                    parameter: String::from(WORKLOAD_IDENTITY_PROVIDER),
                })?;
            let provider = WifProvider::parse_str(&provider_str).with_context(|| {
                InvalidParameterValueSnafu {
                    parameter: String::from(WORKLOAD_IDENTITY_PROVIDER),
                    value: provider_str.clone(),
                    explanation: format!("Allowed values: {}", WifProvider::allowed_values()),
                }
            })?;
            let entra_resource = settings
                .get_string(WORKLOAD_IDENTITY_ENTRA_RESOURCE)
                .filter(|s| !s.is_empty());
            let impersonation_path = settings
                .get_string(WORKLOAD_IDENTITY_IMPERSONATION_PATH)
                .filter(|s| !s.is_empty())
                .map(|s| {
                    s.split(',')
                        .map(|p| p.trim().to_string())
                        .filter(|p| !p.is_empty())
                        .collect()
                })
                .unwrap_or_default();
            let oidc_token = settings
                .get_sensitive_string(TOKEN)
                .filter(|s| !s.reveal().is_empty());
            Ok(AuthConfig::WorkloadIdentity(WorkloadIdentityConfig {
                provider,
                entra_resource,
                impersonation_path,
                oidc_token,
            }))
        }
        _ => InvalidParameterValueSnafu {
            parameter: String::from(AUTHENTICATOR),
            value: authenticator,
            explanation: crate::config::AUTHENTICATOR_ALLOWED_VALUES.to_string(),
        }
        .fail(),
    }
}

// ---------------------------------------------------------------------------
// ConnectionConfig::build
// ---------------------------------------------------------------------------

impl ConnectionConfig {
    /// Build a typed config from a resolved settings map.
    ///
    /// The input should come from `resolver::resolve` or `resolver::resolve_with_paths`.
    /// Runs `validate_settings` first and returns all validation errors
    /// collected (not just the first) via `ConfigError::Validation`.
    /// Runtime errors that go beyond static validation (e.g. base64
    /// decoding failures, file I/O) are still returned individually.
    pub fn build(settings: &ParamStore) -> Result<Self, ConfigError> {
        let issues = validate_settings(settings);
        let errors: Vec<_> = issues
            .into_iter()
            .filter(|i| i.severity == ValidationSeverity::Error)
            .collect();
        if !errors.is_empty() {
            return ValidationSnafu { issues: errors }.fail();
        }

        let account = settings
            .get_string(ACCOUNT)
            .context(MissingParameterSnafu {
                parameter: String::from(ACCOUNT),
            })?;
        let server_url = derive_server_url(settings)?;
        let auth = build_auth_config(settings)?;
        let tls = build_tls_config(settings)?;
        let proxy = build_proxy_config(settings);

        let session = SessionContext {
            database: settings.get_string(DATABASE),
            schema: settings.get_string(SCHEMA),
            warehouse: settings.get_string(WAREHOUSE),
            role: settings.get_string(ROLE),
        };

        let disable_parallel_user_prompt = settings
            .get_bool(DISABLE_PARALLEL_USER_PROMPT)
            .unwrap_or(true);

        Ok(Self {
            server: ServerConfig {
                account,
                server_url,
            },
            auth,
            session,
            tls,
            proxy,
            disable_parallel_user_prompt,
            diagnostic: if settings.get_bool(ENABLE_CONNECTION_DIAG).unwrap_or(false) {
                DiagnosticConfig::Enabled {
                    log_path: settings
                        .get_string(CONNECTION_DIAG_LOG_PATH)
                        .map(PathBuf::from),
                    allowlist_path: settings
                        .get_string(CONNECTION_DIAG_ALLOWLIST_PATH)
                        .map(PathBuf::from),
                }
            } else {
                DiagnosticConfig::Disabled
            },
        })
    }
}

fn login_method_from_auth_config(auth: &AuthConfig) -> LoginMethod {
    match auth {
        AuthConfig::Password {
            user,
            password,
            passcode_in_password,
            passcode,
        } => LoginMethod::Password {
            username: user.clone(),
            password: password.clone(),
            passcode_in_password: *passcode_in_password,
            passcode: passcode.clone(),
        },
        AuthConfig::Mfa {
            user,
            password,
            passcode_in_password,
            passcode,
            client_store_temporary_credential,
        } => LoginMethod::UserPasswordMfa {
            username: user.clone(),
            password: password.clone(),
            passcode_in_password: *passcode_in_password,
            passcode: passcode.clone(),
            client_store_temporary_credential: *client_store_temporary_credential,
        },
        AuthConfig::Jwt {
            user,
            private_key_pem,
            passphrase,
        } => LoginMethod::PrivateKey {
            username: user.clone(),
            private_key: private_key_pem.clone(),
            passphrase: passphrase.clone(),
        },
        AuthConfig::Pat { user, token } => LoginMethod::Pat {
            username: user.clone(),
            token: token.clone(),
        },
        AuthConfig::NativeOkta(okta) => LoginMethod::NativeOkta(NativeOktaConfig {
            username: okta.username.clone(),
            okta_username: okta.okta_username.clone(),
            password: okta.password.clone(),
            okta_url: okta.okta_url.clone(),
            disable_saml_url_check: okta.disable_saml_url_check,
            authentication_timeout_secs: okta.authentication_timeout_secs,
        }),
        AuthConfig::ExternalBrowser {
            user,
            authentication_timeout_secs,
            client_store_temporary_credential,
        } => LoginMethod::ExternalBrowser {
            username: user.clone(),
            authentication_timeout_secs: *authentication_timeout_secs,
            client_store_temporary_credential: *client_store_temporary_credential,
        },
        AuthConfig::OAuthAccessToken { user, token } => LoginMethod::OAuthAccessToken {
            username: user.clone(),
            token: token.clone(),
        },
        // Clone the whole config in one shot: the source and target are the
        // same `OAuthAuthorizationCodeConfig` type, so a field-by-field copy
        // would only add change-amplification risk (silently dropping any
        // future field). The launcher factory is a cheap `Arc` clone.
        AuthConfig::OAuthAuthorizationCode(cfg) => LoginMethod::OAuthAuthorizationCode(cfg.clone()),
        AuthConfig::OAuthClientCredentials(cfg) => {
            LoginMethod::OAuthClientCredentials(OAuthClientCredentialsConfig {
                username: cfg.username.clone(),
                client_id: cfg.client_id.clone(),
                client_secret: cfg.client_secret.clone(),
                token_url: cfg.token_url.clone(),
                scope: cfg.scope.clone(),
                credentials_in_body: cfg.credentials_in_body,
                flow_options: OAuthFlowOptions {
                    enable_dpop: cfg.flow_options.enable_dpop,
                    authentication_timeout_secs: cfg.flow_options.authentication_timeout_secs,
                },
            })
        }
        AuthConfig::SessionToken {
            session_token,
            master_token,
            master_validity_in_seconds,
        } => LoginMethod::SessionToken {
            session_token: session_token.clone(),
            master_token: master_token.clone(),
            master_validity_in_seconds: *master_validity_in_seconds,
        },
        AuthConfig::WorkloadIdentity(cfg) => {
            LoginMethod::WorkloadIdentity(WorkloadIdentityConfig {
                provider: cfg.provider,
                entra_resource: cfg.entra_resource.clone(),
                impersonation_path: cfg.impersonation_path.clone(),
                oidc_token: cfg.oidc_token.clone(),
            })
        }
    }
}

impl LoginParameters {
    /// Build Snowflake login parameters from a validated [`ConnectionConfig`].
    ///
    /// Session defaults (`database`, `schema`, `warehouse`, `role`) reflect the resolved
    /// connection seed at login time (`used_at_connect` session fields).
    pub fn from_connection_config(
        config: &ConnectionConfig,
        client_info: ClientInfo,
        session_parameters: Option<HashMap<String, String>>,
        spcs_token: Option<String>,
    ) -> Self {
        Self {
            account_name: config.server.account.clone(),
            login_method: login_method_from_auth_config(&config.auth),
            server_url: config.server.server_url.clone(),
            database: config.session.database.clone(),
            schema: config.session.schema.clone(),
            warehouse: config.session.warehouse.clone(),
            role: config.session.role.clone(),
            client_info,
            session_parameters,
            spcs_token,
            disable_parallel_user_prompt: config.disable_parallel_user_prompt,
        }
    }
}

// ---------------------------------------------------------------------------
// validate_settings – pre-flight check that collects all issues
// ---------------------------------------------------------------------------

/// Push an `InvalidValue` issue when `key` is present and non-empty but
/// cannot be parsed as a URL. Absent or empty values are intentionally
/// ignored — presence checks (when required) are handled separately by
/// the caller so that a missing-and-malformed value never produces both
/// `MissingRequired` *and* `InvalidValue` for the same parameter.
fn push_invalid_url_issue(
    settings: &ParamStore,
    key: crate::config::ParamKey,
    issues: &mut Vec<ValidationIssue>,
) {
    let Some(raw) = non_empty_string(settings, key) else {
        return;
    };
    if let Err(e) = Url::parse(&raw) {
        issues.push(ValidationIssue {
            severity: ValidationSeverity::Error,
            parameter: key.into(),
            message: format!("Invalid URL for '{key}': could not parse '{raw}': {e}"),
            code: ValidationCode::InvalidValue,
        });
    }
}

/// Validate settings without building the full config.
/// Returns a list of all issues found (errors and warnings).
pub fn validate_settings(settings: &ParamStore) -> Vec<ValidationIssue> {
    let mut issues = Vec::new();

    // TODO(sfc-gh-boler): Preserve the current compatibility-first coercion
    // behavior here, but stop reporting present non-coercible values as
    // MissingRequired. Reuse the same coercion rules used by option-setting
    // validation so coercible legacy config values remain accepted while truly
    // wrong-typed values can surface as InvalidType.

    // --- MissingRequired: account ---
    if settings.get_string(ACCOUNT).is_none() {
        issues.push(ValidationIssue {
            severity: ValidationSeverity::Error,
            parameter: ACCOUNT.into(),
            message: "Missing required parameter 'account'".into(),
            code: ValidationCode::MissingRequired,
        });
    }

    // --- InvalidValue: account identifier characters (SNOW-3663586, CWE-918) ---
    // `account` is interpolated into the derived host, so restrict it to the
    // characters a Snowflake account identifier actually uses — ASCII
    // alphanumerics plus `.`, `-`, and `_` (underscores are normalized to
    // hyphens in the host). Reject anything else before a host is derived or
    // contacted. An explicitly supplied `host`/`server_url` is the caller's own
    // endpoint choice and is validated as a URL elsewhere.
    if let Some(account) = settings.get_string(ACCOUNT)
        && let Some(invalid_character) = account.chars().find(|c| !is_allowed_account_char(*c))
    {
        issues.push(ValidationIssue {
            severity: ValidationSeverity::Error,
            parameter: ACCOUNT.into(),
            message: format!(
                "Invalid character {invalid_character:?} in account identifier '{account}'. \
                 Account identifiers may only contain letters, digits, '.', '-', and '_'."
            ),
            code: ValidationCode::InvalidValue,
        });
    }

    // --- Auth-specific checks based on authenticator ---
    let authenticator = settings.get_string(AUTHENTICATOR).unwrap_or_default();
    let auth_upper = authenticator.to_ascii_uppercase();

    // Session token auth is detected by the presence of `session_token` rather than
    // an authenticator string (build_auth_config checks SESSION_TOKEN first). Skip all
    // user/password requirements when both tokens are present.
    let has_session_token = non_empty_string(settings, SESSION_TOKEN).is_some();

    // --- MissingRequired: user ---
    // SNOW-3647715: token-based authenticators waive the `user`
    // requirement — the principal is encoded in the IdP-issued token
    // (or PAT) and resolved by GS at login time. WIF also waives the
    // user requirement since the cloud identity is resolved server-side.
    // Session token auth likewise carries no user identity requirement.
    let user_optional = has_session_token
        || matches!(
            auth_upper.as_str(),
            "OAUTH"
                | "OAUTH_AUTHORIZATION_CODE"
                | "OAUTH_CLIENT_CREDENTIALS"
                | "PROGRAMMATIC_ACCESS_TOKEN"
                | "WORKLOAD_IDENTITY"
        );
    if !user_optional && non_empty_string(settings, USER).is_none() {
        issues.push(ValidationIssue {
            severity: ValidationSeverity::Error,
            parameter: USER.into(),
            message: "Missing required parameter 'user'".into(),
            code: ValidationCode::MissingRequired,
        });
    }
    match auth_upper.as_str() {
        _ if has_session_token => {
            // Session token auth: user/password not required; build_auth_config handles it.
        }
        "" if has_private_key_params(settings) => {
            // Empty authenticator + private key params → auto-JWT, no password needed
        }
        "SNOWFLAKE" | "SNOWFLAKE_PASSWORD" if has_private_key_params(settings) => {
            issues.push(ValidationIssue {
                severity: ValidationSeverity::Error,
                parameter: AUTHENTICATOR.into(),
                message: "Cannot specify basic authenticator together with \
                          private key parameters; use 'SNOWFLAKE_JWT' or remove the private \
                          key parameters"
                    .into(),
                code: ValidationCode::ConflictingParameters,
            });
        }
        "SNOWFLAKE" | "SNOWFLAKE_PASSWORD" | "" => {
            if settings.get_string(PASSWORD).is_none_or(|s| s.is_empty()) {
                issues.push(ValidationIssue {
                    severity: ValidationSeverity::Error,
                    parameter: PASSWORD.into(),
                    message: "Missing required parameter 'password' for password authentication"
                        .into(),
                    code: ValidationCode::MissingRequired,
                });
            }
        }
        "USERNAME_PASSWORD_MFA" => {
            if settings.get_string(PASSWORD).is_none_or(|s| s.is_empty()) {
                issues.push(ValidationIssue {
                    severity: ValidationSeverity::Error,
                    parameter: PASSWORD.into(),
                    message: "Missing required parameter 'password' for MFA authentication".into(),
                    code: ValidationCode::MissingRequired,
                });
            }
        }
        "SNOWFLAKE_JWT" => {
            if !has_private_key_params(settings) {
                issues.push(ValidationIssue {
                    severity: ValidationSeverity::Error,
                    parameter: "private_key or private_key_file".into(),
                    message: "Missing required parameter: 'private_key' or 'private_key_file'"
                        .into(),
                    code: ValidationCode::MissingRequired,
                });
            }
        }
        "PROGRAMMATIC_ACCESS_TOKEN" => {
            if settings.get_string(TOKEN).is_none_or(|s| s.is_empty()) {
                issues.push(ValidationIssue {
                    severity: ValidationSeverity::Error,
                    parameter: TOKEN.into(),
                    message: "Missing required parameter 'token' for PAT authentication".into(),
                    code: ValidationCode::MissingRequired,
                });
            }
        }
        "OAUTH" => {
            // Legacy OAuth forwards a pre-acquired access token
            // verbatim; the only required payload-side parameter is
            // `token`. user/account are already validated above.
            if settings.get_string(TOKEN).is_none_or(|s| s.is_empty()) {
                issues.push(ValidationIssue {
                    severity: ValidationSeverity::Error,
                    parameter: TOKEN.into(),
                    message: "Missing required parameter 'token' for OAuth authentication".into(),
                    code: ValidationCode::MissingRequired,
                });
            }
        }
        // TODO(SNOW-3552175): The OAuth arms below duplicate URL-shape
        // checks already implemented (fail-fast) in
        // `rest_parameters::OAuth*Config::from_settings`. We re-do them
        // here so that pre-flight validation can report every issue in
        // a single pass instead of surfacing them one-at-a-time at
        // build time. The structural fix is to have
        // `*Config::from_settings` (and the other auth `from_settings`
        // impls) return `Vec<ValidationIssue>` instead of
        // `Result<_, ConfigError>` and let `build_auth_config`
        // aggregate — at that point this duplication can be removed
        // across all auth methods, not just OAuth. This also dovetails
        // with the typed `AuthenticationError` enum work in
        // SNOW-3549115.
        "OAUTH_AUTHORIZATION_CODE" => {
            // AC flow defaults to Snowflake-as-IdP when
            // client_id/secret are absent, so we only require `user`
            // here (already validated above). All three OAuth URL
            // parameters are optional, but when supplied they must be
            // parseable URLs — validate shape so a connection string
            // with multiple malformed URLs reports all of them at once.
            push_invalid_url_issue(settings, OAUTH_AUTHORIZATION_URL, &mut issues);
            push_invalid_url_issue(settings, OAUTH_TOKEN_REQUEST_URL, &mut issues);
            push_invalid_url_issue(settings, OAUTH_REDIRECT_URI, &mut issues);
        }
        "OAUTH_CLIENT_CREDENTIALS" => {
            // CC flow is external-IdP only: client_id, client_secret,
            // and oauth_token_request_url must be provided up-front
            // (Snowflake's GS does not mint CC tokens).
            if settings
                .get_string(OAUTH_CLIENT_ID)
                .is_none_or(|s| s.is_empty())
            {
                issues.push(ValidationIssue {
                    severity: ValidationSeverity::Error,
                    parameter: OAUTH_CLIENT_ID.into(),
                    message: "Missing required parameter 'oauth_client_id' for OAuth client \
                              credentials authentication"
                        .into(),
                    code: ValidationCode::MissingRequired,
                });
            }
            if settings
                .get_string(OAUTH_CLIENT_SECRET)
                .is_none_or(|s| s.is_empty())
            {
                issues.push(ValidationIssue {
                    severity: ValidationSeverity::Error,
                    parameter: OAUTH_CLIENT_SECRET.into(),
                    message: "Missing required parameter 'oauth_client_secret' for OAuth client \
                              credentials authentication"
                        .into(),
                    code: ValidationCode::MissingRequired,
                });
            }
            if settings
                .get_string(OAUTH_TOKEN_REQUEST_URL)
                .is_none_or(|s| s.is_empty())
            {
                issues.push(ValidationIssue {
                    severity: ValidationSeverity::Error,
                    parameter: OAUTH_TOKEN_REQUEST_URL.into(),
                    message: "Missing required parameter 'oauth_token_request_url' for OAuth \
                              client credentials authentication"
                        .into(),
                    code: ValidationCode::MissingRequired,
                });
            }
            // Shape-validate `oauth_token_request_url` in addition to
            // the presence check above. Absent/empty values were
            // already reported as `MissingRequired`, so this only adds
            // an `InvalidValue` issue when a non-empty value is
            // malformed.
            push_invalid_url_issue(settings, OAUTH_TOKEN_REQUEST_URL, &mut issues);
        }
        "EXTERNALBROWSER" => {
            // no validation required; user is already validated above.
        }
        "WORKLOAD_IDENTITY" => {
            // provider is required
            if settings
                .get_string(WORKLOAD_IDENTITY_PROVIDER)
                .is_none_or(|s| s.is_empty())
            {
                issues.push(ValidationIssue {
                    severity: ValidationSeverity::Error,
                    parameter: WORKLOAD_IDENTITY_PROVIDER.into(),
                    message: format!(
                        "Missing required parameter 'workload_identity_provider' for \
                              WORKLOAD_IDENTITY authentication. \
                              Allowed values: {}",
                        WifProvider::allowed_values()
                    ),
                    code: ValidationCode::MissingRequired,
                });
            } else if let Some(p) = settings.get_string(WORKLOAD_IDENTITY_PROVIDER)
                && WifProvider::parse_str(&p).is_none()
            {
                issues.push(ValidationIssue {
                    severity: ValidationSeverity::Error,
                    parameter: WORKLOAD_IDENTITY_PROVIDER.into(),
                    message: format!(
                        "Invalid workload_identity_provider '{p}'. \
                             Allowed values: {}",
                        WifProvider::allowed_values()
                    ),
                    code: ValidationCode::InvalidValue,
                });
            }
            // OIDC provider requires token
            if let Some(provider_str) = settings.get_string(WORKLOAD_IDENTITY_PROVIDER)
                && WifProvider::parse_str(&provider_str) == Some(WifProvider::Oidc)
                && settings.get_string(TOKEN).is_none_or(|s| s.is_empty())
            {
                issues.push(ValidationIssue {
                    severity: ValidationSeverity::Error,
                    parameter: TOKEN.into(),
                    message: "Missing required parameter 'token' for OIDC workload \
                                      identity authentication"
                        .into(),
                    code: ValidationCode::MissingRequired,
                });
            }
            // impersonation_path is unsupported for OIDC: OIDC has no notion of
            // impersonation the way cloud-metadata-based attestation does. Matches
            // legacy snowflake-connector-python/Node.js/.NET, which all reject this
            // client-side; JDBC/Go/C/PHP previously ignored it silently.
            if let Some(provider_str) = settings.get_string(WORKLOAD_IDENTITY_PROVIDER)
                && WifProvider::parse_str(&provider_str) == Some(WifProvider::Oidc)
                && settings
                    .get_string(WORKLOAD_IDENTITY_IMPERSONATION_PATH)
                    .is_some_and(|s| !s.is_empty())
            {
                issues.push(ValidationIssue {
                    severity: ValidationSeverity::Error,
                    parameter: WORKLOAD_IDENTITY_IMPERSONATION_PATH.into(),
                    message: "workload_identity_impersonation_path is currently only supported \
                              for GCP, AWS, and AZURE"
                        .into(),
                    code: ValidationCode::ConflictingWifParameters,
                });
            }
            // Azure impersonation is single-hop: exactly one SP client_id allowed
            if let Some(provider_str) = settings.get_string(WORKLOAD_IDENTITY_PROVIDER)
                && WifProvider::parse_str(&provider_str) == Some(WifProvider::Azure)
                && let Some(path_str) = settings.get_string(WORKLOAD_IDENTITY_IMPERSONATION_PATH)
                && !path_str.is_empty()
            {
                let hop_count = path_str.split(',').count();
                if hop_count != 1 {
                    issues.push(ValidationIssue {
                        severity: ValidationSeverity::Error,
                        parameter: WORKLOAD_IDENTITY_IMPERSONATION_PATH.into(),
                        message: format!(
                            "Azure WIF impersonation only supports a single service principal \
                             (single-hop). 'workload_identity_impersonation_path' must contain \
                             exactly one client_id, got {hop_count}."
                        ),
                        code: ValidationCode::InvalidValue,
                    });
                }
            }
        }
        _ if auth_upper.starts_with("HTTPS://") => {
            if Url::parse(&authenticator).is_err() {
                issues.push(ValidationIssue {
                    severity: ValidationSeverity::Error,
                    parameter: AUTHENTICATOR.into(),
                    message: format!(
                        "The authenticator URL '{}' is not a valid URL",
                        authenticator
                    ),
                    code: ValidationCode::InvalidValue,
                });
            }
            if settings.get_string(PASSWORD).is_none() {
                issues.push(ValidationIssue {
                    severity: ValidationSeverity::Error,
                    parameter: PASSWORD.into(),
                    message: "Missing required parameter 'password' for native Okta authentication"
                        .into(),
                    code: ValidationCode::MissingRequired,
                });
            }
        }
        _ => {
            issues.push(ValidationIssue {
                severity: ValidationSeverity::Error,
                parameter: AUTHENTICATOR.into(),
                message: format!(
                    "Invalid authenticator '{authenticator}'. {}",
                    crate::config::AUTHENTICATOR_ALLOWED_VALUES
                ),
                code: ValidationCode::InvalidValue,
            });
        }
    }

    // --- MissingRequired: host (when server_url is absent) ---
    if settings.get_string(SERVER_URL).is_none() && settings.get_string(HOST).is_none() {
        issues.push(ValidationIssue {
            severity: ValidationSeverity::Error,
            parameter: HOST.into(),
            message: "Missing required parameter 'host' (or 'server_url')".into(),
            code: ValidationCode::MissingRequired,
        });
    }

    // --- ConflictingParameters: ssl + protocol ---
    if settings.get_bool(SSL).is_some() && settings.get_string(PROTOCOL).is_some() {
        issues.push(ValidationIssue {
            severity: ValidationSeverity::Error,
            parameter: SSL.into(),
            message: "Both 'ssl' and 'protocol' are set. Please provide only one.".into(),
            code: ValidationCode::ConflictingParameters,
        });
    }

    // --- InvalidValue: protocol ---
    if let Some(protocol) = settings.get_string(PROTOCOL)
        && protocol != "http"
        && protocol != "https"
    {
        issues.push(ValidationIssue {
            severity: ValidationSeverity::Error,
            parameter: PROTOCOL.into(),
            message: format!("Invalid protocol '{protocol}'. Allowed values: 'http', 'https'"),
            code: ValidationCode::InvalidValue,
        });
    }

    // --- InvalidValue: crl_check_mode ---
    // TODO: make matching case-insensitive (e.g. "disabled", "Enabled")
    if let Some(mode) = settings.get_string(CRL_CHECK_MODE) {
        let valid = ["DISABLED", "ENABLED", "ADVISORY", "0", "1", "2"];
        if !valid.contains(&mode.as_str()) {
            issues.push(ValidationIssue {
                severity: ValidationSeverity::Error,
                parameter: CRL_CHECK_MODE.into(),
                message: format!(
                    "Invalid crl_check_mode '{mode}'. Allowed: DISABLED, ENABLED, ADVISORY, 0, 1, 2"
                ),
                code: ValidationCode::InvalidValue,
            });
        }
    }

    // --- InvalidValue / ConflictingParameters: min_tls_version / max_tls_version ---
    // Reuse the canonical parser so the accepted spellings can't drift from
    // `TlsConfig::from_settings` (design-discipline rule 2). Each bad value is
    // reported independently, then the [min, max] ordering, upholding this
    // function's "collect ALL errors" contract.
    let parse_tls = |key: crate::config::ParamKey| {
        settings
            .get_string(key)
            .map(|v| (v.clone(), TlsVersion::parse(&v, key.as_str())))
    };
    let min_parsed = parse_tls(MIN_TLS_VERSION);
    let max_parsed = parse_tls(MAX_TLS_VERSION);
    for (key, parsed) in [
        (MIN_TLS_VERSION, &min_parsed),
        (MAX_TLS_VERSION, &max_parsed),
    ] {
        if let Some((raw, Err(err))) = parsed {
            issues.push(ValidationIssue {
                severity: ValidationSeverity::Error,
                parameter: key.into(),
                message: format!("Invalid {key} '{raw}': {err}"),
                code: ValidationCode::InvalidValue,
            });
        }
    }
    if let (Some((_, Ok(min))), Some((_, Ok(max)))) = (&min_parsed, &max_parsed)
        && min > max
    {
        issues.push(ValidationIssue {
            severity: ValidationSeverity::Error,
            parameter: MAX_TLS_VERSION.into(),
            message: format!(
                "max_tls_version ({}) must be at least min_tls_version ({})",
                max.label(),
                min.label()
            ),
            code: ValidationCode::ConflictingParameters,
        });
    }

    // --- ConflictingParameters: private_key + private_key_file ---
    let has_pk = settings.get(PRIVATE_KEY).is_some();
    let has_pk_file = settings.get_string(PRIVATE_KEY_FILE).is_some();
    if has_pk && has_pk_file {
        issues.push(ValidationIssue {
            severity: ValidationSeverity::Error,
            parameter: PRIVATE_KEY.into(),
            message: "Both 'private_key' and 'private_key_file' are set. Please provide only one."
                .into(),
            code: ValidationCode::ConflictingParameters,
        });
    }

    // --- ConflictingParameters: WIF-only params require WORKLOAD_IDENTITY ---
    if !matches!(auth_upper.as_str(), "WORKLOAD_IDENTITY") {
        for param in [
            WORKLOAD_IDENTITY_PROVIDER,
            WORKLOAD_IDENTITY_ENTRA_RESOURCE,
            WORKLOAD_IDENTITY_IMPERSONATION_PATH,
        ] {
            if non_empty_string(settings, param).is_some() {
                issues.push(ValidationIssue {
                    severity: ValidationSeverity::Error,
                    parameter: param.into(),
                    message: format!(
                        "{param} was set but authenticator was not set to WORKLOAD_IDENTITY"
                    ),
                    code: ValidationCode::ConflictingWifParameters,
                });
            }
        }
    }

    // --- UnknownParameter: keys not in ParamRegistry ---
    let registry = crate::config::param_registry::registry();
    for key in settings.keys() {
        if !registry.is_known(key) {
            issues.push(ValidationIssue {
                severity: ValidationSeverity::Warning,
                parameter: key.clone(),
                message: format!("Unknown parameter '{key}'"),
                code: ValidationCode::UnknownParameter,
            });
        }
    }

    issues
}

// ---------------------------------------------------------------------------
// Unit tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::crl::config::CertRevocationCheckMode;

    fn settings_from(pairs: &[(&str, Setting)]) -> ParamStore {
        let mut settings = ParamStore::with_registry_defaults();
        for (key, value) in pairs {
            settings.insert((*key).to_string(), value.clone());
        }
        settings
    }

    fn minimal_password_settings() -> ParamStore {
        settings_from(&[
            ("account", Setting::String("myaccount".into())),
            ("user", Setting::String("myuser".into())),
            ("password", Setting::String("mypassword".into())),
            (
                "host",
                Setting::String("myaccount.snowflakecomputing.com".into()),
            ),
        ])
    }

    fn minimal_mfa_settings() -> ParamStore {
        let mut settings = minimal_password_settings();
        settings.insert(
            "authenticator".into(),
            Setting::String("USERNAME_PASSWORD_MFA".into()),
        );
        settings
    }

    // -- ConnectionConfig::build tests --

    #[test]
    fn build_minimal_password_auth_succeeds() {
        let settings = minimal_password_settings();
        let config = ConnectionConfig::build(&settings).unwrap();

        assert_eq!(config.server.account, "myaccount");
        assert!(
            config
                .server
                .server_url
                .contains("myaccount.snowflakecomputing.com")
        );
        match &config.auth {
            AuthConfig::Password { user, password, .. } => {
                assert_eq!(user, "myuser");
                assert_eq!(password.reveal(), "mypassword");
            }
            _ => panic!("Expected Password auth"),
        }
        assert_eq!(config.session.database, None);
        assert!(config.tls.verify_hostname);
    }

    #[test]
    fn build_missing_account_fails() {
        let settings = settings_from(&[
            ("user", Setting::String("u".into())),
            ("password", Setting::String("p".into())),
            ("host", Setting::String("h.com".into())),
        ]);
        let err = ConnectionConfig::build(&settings).unwrap_err();
        match err {
            ConfigError::Validation { ref issues, .. } => {
                assert!(
                    issues
                        .iter()
                        .any(|i| i.parameter == "account"
                            && i.code == ValidationCode::MissingRequired),
                    "Expected MissingRequired for 'account', got: {issues:?}"
                );
            }
            other => panic!("Expected Validation, got: {other}"),
        }
    }

    #[test]
    fn build_server_url_from_host_port_protocol() {
        let settings = settings_from(&[
            ("account", Setting::String("acct".into())),
            ("user", Setting::String("u".into())),
            ("password", Setting::String("p".into())),
            ("host", Setting::String("myhost.com".into())),
            ("port", Setting::Int(8443)),
            ("protocol", Setting::String("https".into())),
        ]);
        let config = ConnectionConfig::build(&settings).unwrap();
        assert_eq!(config.server.server_url, "https://myhost.com:8443");
    }

    #[test]
    fn build_server_url_direct_override() {
        let settings = settings_from(&[
            ("account", Setting::String("acct".into())),
            ("user", Setting::String("u".into())),
            ("password", Setting::String("p".into())),
            ("server_url", Setting::String("https://custom.url".into())),
        ]);
        let config = ConnectionConfig::build(&settings).unwrap();
        assert_eq!(config.server.server_url, "https://custom.url");
    }

    #[test]
    fn build_server_url_from_ssl_true() {
        let settings = settings_from(&[
            ("account", Setting::String("acct".into())),
            ("user", Setting::String("u".into())),
            ("password", Setting::String("p".into())),
            ("host", Setting::String("myhost.com".into())),
            ("ssl", Setting::Bool(true)),
        ]);
        let config = ConnectionConfig::build(&settings).unwrap();
        assert_eq!(config.server.server_url, "https://myhost.com");
    }

    #[test]
    fn build_server_url_from_ssl_false() {
        let settings = settings_from(&[
            ("account", Setting::String("acct".into())),
            ("user", Setting::String("u".into())),
            ("password", Setting::String("p".into())),
            ("host", Setting::String("myhost.com".into())),
            ("ssl", Setting::Bool(false)),
        ]);
        let config = ConnectionConfig::build(&settings).unwrap();
        assert_eq!(config.server.server_url, "http://myhost.com");
    }

    #[test]
    fn build_server_url_ssl_and_protocol_conflict() {
        let settings = settings_from(&[
            ("account", Setting::String("acct".into())),
            ("user", Setting::String("u".into())),
            ("password", Setting::String("p".into())),
            ("host", Setting::String("myhost.com".into())),
            ("protocol", Setting::String("http".into())),
            ("ssl", Setting::Bool(true)),
        ]);
        let err = ConnectionConfig::build(&settings).unwrap_err();
        match err {
            ConfigError::Validation { ref issues, .. } => {
                assert!(
                    issues
                        .iter()
                        .any(|i| i.code == ValidationCode::ConflictingParameters
                            && i.parameter == "ssl"),
                    "Expected ConflictingParameters for ssl + protocol, got: {issues:?}"
                );
            }
            other => panic!("Expected Validation, got: {other}"),
        }
    }

    #[test]
    fn build_proxy_config_empty_by_default() {
        let settings = minimal_password_settings();
        let config = ConnectionConfig::build(&settings).unwrap();
        assert!(config.proxy.host.is_none());
        assert!(config.proxy.port.is_none());
        assert!(config.proxy.user.is_none());
        assert!(config.proxy.password.is_none());
        assert!(config.proxy.no_proxy.is_none());
        assert!(!config.proxy.is_explicit());
    }

    #[test]
    fn build_proxy_config_populated() {
        let mut settings = minimal_password_settings();
        settings.insert(
            "proxy_host".into(),
            Setting::String("proxy.example.com".into()),
        );
        settings.insert("proxy_port".into(), Setting::Int(8080));
        settings.insert("proxy_user".into(), Setting::String("puser".into()));
        settings.insert("proxy_password".into(), Setting::String("ppass".into()));
        settings.insert(
            "no_proxy".into(),
            Setting::String("internal.example.com,*.local".into()),
        );

        let config = ConnectionConfig::build(&settings).unwrap();
        assert_eq!(config.proxy.host.as_deref(), Some("proxy.example.com"));
        assert_eq!(config.proxy.port, Some(8080));
        assert_eq!(config.proxy.user.as_deref(), Some("puser"));
        assert_eq!(
            config
                .proxy
                .password
                .as_ref()
                .map(|p| p.reveal().to_string()),
            Some("ppass".to_string())
        );
        assert_eq!(
            config.proxy.no_proxy.as_deref(),
            Some("internal.example.com,*.local")
        );
        assert!(config.proxy.is_explicit());
    }

    #[test]
    fn build_proxy_config_port_from_string() {
        // Per ParamStore::get_int, string values like `proxy_port = "8080"` from
        // TOML or DSN strings are coerced to ints. Verify that path works.
        let mut settings = minimal_password_settings();
        settings.insert("proxy_host".into(), Setting::String("p.example.com".into()));
        settings.insert("proxy_port".into(), Setting::String("8080".into()));
        let config = ConnectionConfig::build(&settings).unwrap();
        assert_eq!(config.proxy.port, Some(8080));
    }

    #[test]
    fn build_proxy_config_from_legacy_url_form() {
        // `PROXY=user:pass@host:port` (legacy ODBC) is parsed into the
        // typed fields.
        let mut settings = minimal_password_settings();
        settings.insert(
            "proxy".into(),
            Setting::String("http://puser:ppass@proxy.example.com:8080".into()),
        );
        let config = ConnectionConfig::build(&settings).unwrap();
        assert_eq!(config.proxy.host.as_deref(), Some("proxy.example.com"));
        assert_eq!(config.proxy.port, Some(8080));
        assert_eq!(config.proxy.user.as_deref(), Some("puser"));
        assert_eq!(
            config
                .proxy
                .password
                .as_ref()
                .map(|p| p.reveal().to_string()),
            Some("ppass".to_string())
        );
    }

    #[test]
    fn build_proxy_config_url_without_scheme_defaults_to_http() {
        let mut settings = minimal_password_settings();
        settings.insert(
            "proxy".into(),
            Setting::String("proxy.example.com:3128".into()),
        );
        let config = ConnectionConfig::build(&settings).unwrap();
        assert_eq!(config.proxy.host.as_deref(), Some("proxy.example.com"));
        assert_eq!(config.proxy.port, Some(3128));
    }

    #[test]
    fn build_proxy_config_individual_fields_override_url() {
        // When both forms are set, individual fields override the matching
        // URL components per-field. Customer can use the URL for host:port
        // and override only the password from a more secure source.
        let mut settings = minimal_password_settings();
        settings.insert(
            "proxy".into(),
            Setting::String("http://urluser:urlpass@url.example.com:1111".into()),
        );
        settings.insert(
            "proxy_password".into(),
            Setting::String("override_pass".into()),
        );
        let config = ConnectionConfig::build(&settings).unwrap();
        // host/port/user from URL kept; password overridden.
        assert_eq!(config.proxy.host.as_deref(), Some("url.example.com"));
        assert_eq!(config.proxy.port, Some(1111));
        assert_eq!(config.proxy.user.as_deref(), Some("urluser"));
        assert_eq!(
            config
                .proxy
                .password
                .as_ref()
                .map(|p| p.reveal().to_string()),
            Some("override_pass".to_string())
        );
    }

    #[test]
    fn build_proxy_config_url_percent_decodes_credentials() {
        // Round-trip the canonical encoding: user@corp / p:a/ss@1.
        let mut settings = minimal_password_settings();
        settings.insert(
            "proxy".into(),
            Setting::String("http://user%40corp:p%3Aa%2Fss%401@proxy.example.com:8080".into()),
        );
        let config = ConnectionConfig::build(&settings).unwrap();
        assert_eq!(config.proxy.user.as_deref(), Some("user@corp"));
        assert_eq!(
            config
                .proxy
                .password
                .as_ref()
                .map(|p| p.reveal().to_string()),
            Some("p:a/ss@1".to_string())
        );
    }

    #[test]
    fn build_proxy_config_use_proxy_env_default_false() {
        let settings = minimal_password_settings();
        let config = ConnectionConfig::build(&settings).unwrap();
        assert!(!config.proxy.use_proxy_env);
        assert!(config.proxy.allow_empty_proxy);
        assert!(!config.proxy.explicitly_disabled);
    }

    #[test]
    fn build_proxy_config_use_proxy_env_opt_in() {
        let mut settings = minimal_password_settings();
        settings.insert("use_proxy_env".into(), Setting::Bool(true));
        let config = ConnectionConfig::build(&settings).unwrap();
        assert!(config.proxy.use_proxy_env);
    }

    #[test]
    fn build_proxy_config_empty_proxy_with_allow_empty_proxy_disables() {
        // Legacy ODBC AllowEmptyProxy=true: PROXY="" → explicitly disable.
        let mut settings = minimal_password_settings();
        settings.insert("proxy".into(), Setting::String("".into()));
        let config = ConnectionConfig::build(&settings).unwrap();
        assert!(config.proxy.host.is_none());
        assert!(config.proxy.explicitly_disabled);
    }

    #[test]
    fn build_proxy_config_empty_proxy_when_disallowed_is_ignored() {
        let mut settings = minimal_password_settings();
        settings.insert("proxy".into(), Setting::String("".into()));
        settings.insert("allow_empty_proxy".into(), Setting::Bool(false));
        let config = ConnectionConfig::build(&settings).unwrap();
        assert!(!config.proxy.explicitly_disabled);
    }

    #[test]
    fn build_proxy_config_via_uppercase_alias() {
        // ODBC DSN strings deliver UPPERCASE keys; verify the registry alias
        // resolves them. Note: param_registry resolves canonical keys, but
        // ParamStore uses canonical names. This test inserts the canonical
        // forms (which the registry would have resolved aliases to upstream).
        let mut settings = minimal_password_settings();
        settings.insert("proxy_host".into(), Setting::String("p.example.com".into()));
        settings.insert("proxy_port".into(), Setting::Int(3128));
        let config = ConnectionConfig::build(&settings).unwrap();
        assert_eq!(config.proxy.host.as_deref(), Some("p.example.com"));
        assert_eq!(config.proxy.port, Some(3128));
    }

    #[test]
    fn build_session_context_populated() {
        let mut settings = minimal_password_settings();
        settings.insert("database".into(), Setting::String("mydb".into()));
        settings.insert("schema".into(), Setting::String("myschema".into()));
        settings.insert("warehouse".into(), Setting::String("mywh".into()));
        settings.insert("role".into(), Setting::String("myrole".into()));

        let config = ConnectionConfig::build(&settings).unwrap();
        assert_eq!(config.session.database.as_deref(), Some("mydb"));
        assert_eq!(config.session.schema.as_deref(), Some("myschema"));
        assert_eq!(config.session.warehouse.as_deref(), Some("mywh"));
        assert_eq!(config.session.role.as_deref(), Some("myrole"));
    }

    #[test]
    fn build_tls_booleans_from_bool_setting() {
        let mut settings = minimal_password_settings();
        settings.insert("verify_hostname".into(), Setting::Bool(false));
        settings.insert("verify_certificates".into(), Setting::Bool(false));

        let config = ConnectionConfig::build(&settings).unwrap();
        assert!(!config.tls.verify_hostname);
        assert!(!config.tls.verify_certificates);
    }

    #[test]
    fn build_tls_booleans_from_string_fallback() {
        let mut settings = minimal_password_settings();
        settings.insert("verify_hostname".into(), Setting::String("false".into()));
        settings.insert("verify_certificates".into(), Setting::String("true".into()));

        let config = ConnectionConfig::build(&settings).unwrap();
        assert!(!config.tls.verify_hostname);
        assert!(config.tls.verify_certificates);
    }

    #[test]
    fn tls_skip_verify_disables_both_checks() {
        let mut settings = minimal_password_settings();
        settings.insert("tls_skip_verify".into(), Setting::Bool(true));

        let config = ConnectionConfig::build(&settings).unwrap();
        assert!(!config.tls.verify_hostname);
        assert!(!config.tls.verify_certificates);
    }

    #[test]
    fn tls_skip_verify_overrides_individual_verify_flags() {
        let mut settings = minimal_password_settings();
        settings.insert("tls_skip_verify".into(), Setting::Bool(true));
        settings.insert("verify_hostname".into(), Setting::Bool(true));
        settings.insert("verify_certificates".into(), Setting::Bool(true));

        let config = ConnectionConfig::build(&settings).unwrap();
        assert!(!config.tls.verify_hostname);
        assert!(!config.tls.verify_certificates);
    }

    #[test]
    fn tls_skip_verify_canonicalizes_from_any_case_and_disables_verification() {
        use crate::config::param_registry::registry;

        for raw_key in ["tls_skip_verify", "TLS_SKIP_VERIFY", "Tls_Skip_Verify"] {
            let canonical = registry()
                .resolve(raw_key)
                .unwrap_or_else(|| panic!("{raw_key} should resolve"))
                .canonical_name;
            assert_eq!(canonical, "tls_skip_verify");

            let mut settings = minimal_password_settings();
            settings.insert(canonical.to_string(), Setting::Bool(true));

            let config = ConnectionConfig::build(&settings).unwrap();
            assert!(
                !config.tls.verify_hostname,
                "{raw_key} should disable hostname check"
            );
            assert!(
                !config.tls.verify_certificates,
                "{raw_key} should disable cert check"
            );
        }
    }

    #[test]
    fn tls_skip_verify_bypasses_crl_even_when_crl_check_mode_enabled() {
        // Locks the registry description's CRL claim against drift: tls_skip_verify forces
        // verify_certificates=false, and create_tls_client_with_config (tls/client.rs) then
        // early-returns an insecure client without installing the CRL verifier — so CRL is
        // bypassed even at crl_check_mode=ENABLED. The mode itself is left intact, just unused.
        let mut settings = minimal_password_settings();
        settings.insert("tls_skip_verify".into(), Setting::Bool(true));
        settings.insert("crl_check_mode".into(), Setting::String("ENABLED".into()));

        let config = ConnectionConfig::build(&settings).unwrap();
        assert!(!config.tls.verify_certificates);
        assert!(!config.tls.verify_hostname);
        assert!(matches!(
            config.tls.crl_config.check_mode,
            CertRevocationCheckMode::Enabled
        ));
    }

    #[test]
    fn build_pat_auth() {
        let settings = settings_from(&[
            ("account", Setting::String("acct".into())),
            ("user", Setting::String("u".into())),
            ("token", Setting::String("tok123".into())),
            (
                "authenticator",
                Setting::String("PROGRAMMATIC_ACCESS_TOKEN".into()),
            ),
            ("host", Setting::String("h.com".into())),
        ]);
        let config = ConnectionConfig::build(&settings).unwrap();
        match &config.auth {
            AuthConfig::Pat { user, token } => {
                assert_eq!(user, "u");
                assert_eq!(token.reveal(), "tok123");
            }
            _ => panic!("Expected Pat auth"),
        }
    }

    #[test]
    fn build_session_token_auth_without_user_succeeds() {
        // session_token + master_token bypass the user/password requirement;
        // no --user flag is needed (mirrors snowflake-cli's --session-token usage).
        let settings = settings_from(&[
            ("account", Setting::String("acct".into())),
            (
                "host",
                Setting::String("acct.snowflakecomputing.com".into()),
            ),
            ("session_token", Setting::String("sess_tok".into())),
            ("master_token", Setting::String("mstr_tok".into())),
        ]);
        let config = ConnectionConfig::build(&settings).unwrap();
        match &config.auth {
            AuthConfig::SessionToken {
                session_token,
                master_token,
                ..
            } => {
                assert_eq!(session_token.reveal(), "sess_tok");
                assert_eq!(master_token.reveal(), "mstr_tok");
            }
            _ => panic!("Expected SessionToken auth"),
        }
    }

    #[test]
    fn build_session_token_auth_with_user_succeeds() {
        // user is accepted but not required for session token auth.
        let settings = settings_from(&[
            ("account", Setting::String("acct".into())),
            (
                "host",
                Setting::String("acct.snowflakecomputing.com".into()),
            ),
            ("user", Setting::String("alice".into())),
            ("session_token", Setting::String("sess_tok".into())),
            ("master_token", Setting::String("mstr_tok".into())),
        ]);
        let config = ConnectionConfig::build(&settings).unwrap();
        assert!(matches!(config.auth, AuthConfig::SessionToken { .. }));
    }

    #[test]
    fn build_password_auth_with_snowflake_lowercase() {
        let settings = settings_from(&[
            ("account", Setting::String("acct".into())),
            ("user", Setting::String("u".into())),
            ("password", Setting::String("p".into())),
            ("authenticator", Setting::String("snowflake".into())),
            ("host", Setting::String("h.com".into())),
        ]);
        let config = ConnectionConfig::build(&settings).unwrap();
        match &config.auth {
            AuthConfig::Password { user, password, .. } => {
                assert_eq!(user, "u");
                assert_eq!(password.reveal(), "p");
            }
            _ => panic!("Expected Password auth for 'snowflake' authenticator"),
        }
    }

    #[test]
    fn build_password_auth_with_snowflake_mixed_case() {
        let settings = settings_from(&[
            ("account", Setting::String("acct".into())),
            ("user", Setting::String("u".into())),
            ("password", Setting::String("p".into())),
            ("authenticator", Setting::String("Snowflake".into())),
            ("host", Setting::String("h.com".into())),
        ]);
        let config = ConnectionConfig::build(&settings).unwrap();
        match &config.auth {
            AuthConfig::Password { .. } => {}
            _ => panic!("Expected Password auth for 'Snowflake' authenticator"),
        }
    }

    #[test]
    fn build_password_auth_with_snowflake_password_lowercase() {
        let settings = settings_from(&[
            ("account", Setting::String("acct".into())),
            ("user", Setting::String("u".into())),
            ("password", Setting::String("p".into())),
            (
                "authenticator",
                Setting::String("snowflake_password".into()),
            ),
            ("host", Setting::String("h.com".into())),
        ]);
        let config = ConnectionConfig::build(&settings).unwrap();
        match &config.auth {
            AuthConfig::Password { .. } => {}
            _ => panic!("Expected Password auth for 'snowflake_password' authenticator"),
        }
    }

    #[test]
    fn build_pat_auth_case_insensitive() {
        let settings = settings_from(&[
            ("account", Setting::String("acct".into())),
            ("user", Setting::String("u".into())),
            ("token", Setting::String("tok123".into())),
            (
                "authenticator",
                Setting::String("programmatic_access_token".into()),
            ),
            ("host", Setting::String("h.com".into())),
        ]);
        let config = ConnectionConfig::build(&settings).unwrap();
        match &config.auth {
            AuthConfig::Pat { user, token } => {
                assert_eq!(user, "u");
                assert_eq!(token.reveal(), "tok123");
            }
            _ => panic!("Expected Pat auth for lowercase 'programmatic_access_token'"),
        }
    }

    #[test]
    fn build_pat_auth_mixed_case() {
        let settings = settings_from(&[
            ("account", Setting::String("acct".into())),
            ("user", Setting::String("u".into())),
            ("token", Setting::String("tok123".into())),
            (
                "authenticator",
                Setting::String("Programmatic_Access_Token".into()),
            ),
            ("host", Setting::String("h.com".into())),
        ]);
        let config = ConnectionConfig::build(&settings).unwrap();
        match &config.auth {
            AuthConfig::Pat { .. } => {}
            _ => panic!("Expected Pat auth for mixed-case authenticator"),
        }
    }

    #[test]
    fn build_mfa_auth_case_insensitive() {
        let settings = settings_from(&[
            ("account", Setting::String("acct".into())),
            ("user", Setting::String("u".into())),
            ("password", Setting::String("p".into())),
            (
                "authenticator",
                Setting::String("username_password_mfa".into()),
            ),
            ("host", Setting::String("h.com".into())),
        ]);
        let config = ConnectionConfig::build(&settings).unwrap();
        match &config.auth {
            AuthConfig::Mfa { .. } => {}
            _ => panic!("Expected Mfa auth for lowercase 'username_password_mfa'"),
        }
    }

    #[test]
    fn build_jwt_auth_case_insensitive() {
        let settings = settings_from(&[
            ("account", Setting::String("acct".into())),
            ("user", Setting::String("u".into())),
            ("authenticator", Setting::String("snowflake_jwt".into())),
            ("private_key", Setting::String("some_key".into())),
            ("host", Setting::String("h.com".into())),
        ]);
        let result = ConnectionConfig::build(&settings);
        // Will fail at key parsing, but should NOT fail at authenticator recognition
        let err_msg = result.unwrap_err().to_string();
        assert!(
            !err_msg.contains("Invalid authenticator"),
            "Should recognize 'snowflake_jwt' (lowercase): {err_msg}"
        );
    }

    #[test]
    fn build_mfa_auth() {
        let config = ConnectionConfig::build(&minimal_mfa_settings()).unwrap();
        match &config.auth {
            AuthConfig::Mfa {
                user,
                password,
                passcode_in_password,
                passcode,
                client_store_temporary_credential,
            } => {
                assert_eq!(user, "myuser");
                assert_eq!(password.reveal(), "mypassword");
                assert!(!passcode_in_password);
                assert!(passcode.is_none());
                assert!(!client_store_temporary_credential);
            }
            other => panic!("Expected Mfa auth, got {other:?}"),
        }
    }

    #[test]
    fn build_mfa_auth_with_passcode() {
        let mut settings = minimal_mfa_settings();
        settings.insert("passcode".into(), Setting::String("123456".into()));

        let config = ConnectionConfig::build(&settings).unwrap();
        match &config.auth {
            AuthConfig::Mfa { passcode, .. } => {
                assert_eq!(
                    passcode.as_ref().map(|v| v.reveal().as_str()),
                    Some("123456")
                );
            }
            other => panic!("Expected Mfa auth, got {other:?}"),
        }
    }

    #[test]
    fn build_mfa_auth_with_passcode_in_password() {
        let mut settings = minimal_mfa_settings();
        settings.insert("passcodeInPassword".into(), Setting::String("true".into()));

        let config = ConnectionConfig::build(&settings).unwrap();
        match &config.auth {
            AuthConfig::Mfa {
                passcode_in_password,
                ..
            } => {
                assert!(*passcode_in_password);
            }
            other => panic!("Expected Mfa auth, got {other:?}"),
        }
    }

    #[test]
    fn build_mfa_auth_with_temporary_credential_caching() {
        let mut settings = minimal_mfa_settings();
        settings.insert(
            "client_store_temporary_credential".into(),
            Setting::String("1".into()),
        );

        let config = ConnectionConfig::build(&settings).unwrap();
        match &config.auth {
            AuthConfig::Mfa {
                client_store_temporary_credential,
                ..
            } => {
                assert!(*client_store_temporary_credential);
            }
            other => panic!("Expected Mfa auth, got {other:?}"),
        }
    }

    #[test]
    fn build_native_okta_auth() {
        let settings = settings_from(&[
            ("account", Setting::String("acct".into())),
            ("user", Setting::String("u".into())),
            ("password", Setting::String("p".into())),
            (
                "authenticator",
                Setting::String("https://example.okta.com".into()),
            ),
            ("host", Setting::String("h.com".into())),
        ]);
        let config = ConnectionConfig::build(&settings).unwrap();
        match &config.auth {
            AuthConfig::NativeOkta(cfg) => {
                assert_eq!(cfg.username, "u");
                assert_eq!(cfg.password.reveal(), "p");
                assert_eq!(cfg.okta_url.as_str(), "https://example.okta.com/");
                assert!(!cfg.disable_saml_url_check);
                assert_eq!(
                    cfg.authentication_timeout_secs,
                    DEFAULT_AUTHENTICATION_TIMEOUT_SECS
                );
            }
            other => panic!("Expected NativeOkta auth, got {other:?}"),
        }
    }

    #[test]
    fn build_conflicting_private_keys_fails() {
        let settings = settings_from(&[
            ("account", Setting::String("acct".into())),
            ("user", Setting::String("u".into())),
            ("authenticator", Setting::String("SNOWFLAKE_JWT".into())),
            ("private_key", Setting::String("some_key".into())),
            ("private_key_file", Setting::String("/path/to/key".into())),
            ("host", Setting::String("h.com".into())),
        ]);
        let err = ConnectionConfig::build(&settings).unwrap_err();
        match err {
            ConfigError::Validation { ref issues, .. } => {
                assert!(
                    issues
                        .iter()
                        .any(|i| i.code == ValidationCode::ConflictingParameters),
                    "Expected ConflictingParameters issue, got: {issues:?}"
                );
            }
            other => panic!("Expected Validation, got: {other}"),
        }
    }

    // -- validate_settings tests --

    #[test]
    fn validate_missing_account_reports_issue() {
        let settings = settings_from(&[
            ("user", Setting::String("u".into())),
            ("password", Setting::String("p".into())),
        ]);
        let issues = validate_settings(&settings);
        let account_issues: Vec<_> = issues
            .iter()
            .filter(|i| i.parameter == "account" && i.code == ValidationCode::MissingRequired)
            .collect();
        assert!(!account_issues.is_empty());
    }

    // SNOW-3663586 (CWE-918): account identifiers carrying characters outside
    // the allow-list must be rejected before the host is derived.
    #[test]
    fn validate_account_with_url_metacharacters_reports_issue() {
        let invalid_account_names = [
            "acct/x", "acct?x", "acct#x", r"acct\x", "acct@x", "acct:x", "acct x", "acct%x",
        ];
        for account in invalid_account_names {
            let settings = settings_from(&[
                ("account", Setting::String(account.into())),
                ("user", Setting::String("u".into())),
                ("password", Setting::String("p".into())),
            ]);
            let issues = validate_settings(&settings);
            assert!(
                issues
                    .iter()
                    .any(|i| i.parameter == "account" && i.code == ValidationCode::InvalidValue),
                "Expected InvalidValue for account {account:?}, got: {issues:?}"
            );
        }
    }

    // Legitimate account identifier shapes must not be flagged: bare locators,
    // region/cloud-qualified, org-account (hyphen), and underscore-bearing
    // accounts (underscores are later normalized to hyphens in the host).
    #[test]
    fn validate_legitimate_account_identifiers_pass() {
        let valid = [
            "myaccount",
            "myaccount.us-east-1",
            "driverspreprod6.preprod6.us-west-2.aws",
            "myorg-myaccount",
            "my_account",
        ];
        for account in valid {
            let settings = settings_from(&[
                ("account", Setting::String(account.into())),
                ("user", Setting::String("u".into())),
                ("password", Setting::String("p".into())),
                ("host", Setting::String("h.com".into())),
            ]);
            let issues = validate_settings(&settings);
            assert!(
                !issues
                    .iter()
                    .any(|i| i.parameter == "account" && i.code == ValidationCode::InvalidValue),
                "Account {account:?} should be valid, got: {issues:?}"
            );
        }
    }

    #[test]
    fn validate_empty_user_reports_issue() {
        let settings = settings_from(&[
            ("account", Setting::String("acct".into())),
            ("user", Setting::String(String::new())),
            ("password", Setting::String("p".into())),
            ("host", Setting::String("h.com".into())),
        ]);

        let issues = validate_settings(&settings);
        assert!(
            issues
                .iter()
                .any(|i| i.parameter == "user" && i.code == ValidationCode::MissingRequired),
            "Expected empty user to be treated as missing, got: {issues:?}"
        );
    }

    #[test]
    fn validate_empty_password_reports_issue() {
        let settings = settings_from(&[
            ("account", Setting::String("acct".into())),
            ("user", Setting::String("user".into())),
            ("password", Setting::String(String::new())),
            ("host", Setting::String("h.com".into())),
        ]);

        let issues = validate_settings(&settings);
        assert!(
            issues
                .iter()
                .any(|i| i.parameter == "password" && i.code == ValidationCode::MissingRequired),
            "Expected empty password to be treated as missing, got: {issues:?}"
        );
    }

    #[test]
    fn validate_returns_all_issues_not_just_first() {
        let settings = ParamStore::with_registry_defaults();
        let issues = validate_settings(&settings);
        let error_count = issues
            .iter()
            .filter(|i| i.severity == ValidationSeverity::Error)
            .count();
        assert!(
            error_count >= 2,
            "Expected at least account+user errors, got {error_count}"
        );
    }

    #[test]
    fn validate_conflicting_private_keys() {
        let settings = settings_from(&[
            ("account", Setting::String("acct".into())),
            ("user", Setting::String("u".into())),
            ("authenticator", Setting::String("SNOWFLAKE_JWT".into())),
            ("private_key", Setting::String("k".into())),
            ("private_key_file", Setting::String("/f".into())),
        ]);
        let issues = validate_settings(&settings);
        let conflict: Vec<_> = issues
            .iter()
            .filter(|i| i.code == ValidationCode::ConflictingParameters)
            .collect();
        assert_eq!(conflict.len(), 1);
    }

    #[test]
    fn validate_rejects_invalid_min_tls_version() {
        let mut settings = minimal_password_settings();
        settings.insert("min_tls_version".into(), Setting::String("tls99".into()));
        let issues = validate_settings(&settings);
        let bad: Vec<_> = issues
            .iter()
            .filter(|i| i.parameter == "min_tls_version" && i.code == ValidationCode::InvalidValue)
            .collect();
        assert_eq!(bad.len(), 1, "invalid min_tls_version should be reported");
    }

    #[test]
    fn validate_rejects_min_tls_version_above_max() {
        let mut settings = minimal_password_settings();
        settings.insert("min_tls_version".into(), Setting::String("tls13".into()));
        settings.insert("max_tls_version".into(), Setting::String("tls12".into()));
        let issues = validate_settings(&settings);
        let conflict: Vec<_> = issues
            .iter()
            .filter(|i| {
                i.parameter == "max_tls_version" && i.code == ValidationCode::ConflictingParameters
            })
            .collect();
        assert_eq!(
            conflict.len(),
            1,
            "min > max should be reported as a conflict"
        );
    }

    #[test]
    fn validate_accepts_canonical_tls_versions() {
        let mut settings = minimal_password_settings();
        settings.insert("min_tls_version".into(), Setting::String("tls12".into()));
        settings.insert("max_tls_version".into(), Setting::String("tls13".into()));
        let issues = validate_settings(&settings);
        assert!(
            !issues
                .iter()
                .any(|i| matches!(i.parameter.as_str(), "min_tls_version" | "max_tls_version")),
            "valid tls12/tls13 should not produce issues, got: {issues:?}"
        );
    }

    #[test]
    fn validate_explicit_password_auth_with_private_key_is_conflict() {
        let settings = settings_from(&[
            ("account", Setting::String("acct".into())),
            ("user", Setting::String("u".into())),
            (
                "authenticator",
                Setting::String("SNOWFLAKE_PASSWORD".into()),
            ),
            ("private_key", Setting::String("k".into())),
            ("host", Setting::String("h.com".into())),
        ]);
        let issues = validate_settings(&settings);
        assert!(
            issues
                .iter()
                .any(|i| i.code == ValidationCode::ConflictingParameters
                    && i.parameter == "authenticator"),
            "Expected ConflictingParameters for SNOWFLAKE_PASSWORD + private_key, got: {issues:?}"
        );
    }

    #[test]
    fn validate_invalid_protocol() {
        let settings = settings_from(&[
            ("account", Setting::String("acct".into())),
            ("user", Setting::String("u".into())),
            ("password", Setting::String("p".into())),
            ("protocol", Setting::String("ftp".into())),
        ]);
        let issues = validate_settings(&settings);
        let proto_issues: Vec<_> = issues
            .iter()
            .filter(|i| i.parameter == "protocol" && i.code == ValidationCode::InvalidValue)
            .collect();
        assert_eq!(proto_issues.len(), 1);
    }

    #[test]
    fn validate_invalid_crl_check_mode() {
        let settings = settings_from(&[
            ("account", Setting::String("acct".into())),
            ("user", Setting::String("u".into())),
            ("password", Setting::String("p".into())),
            ("crl_check_mode", Setting::String("INVALID".into())),
        ]);
        let issues = validate_settings(&settings);
        let crl_issues: Vec<_> = issues
            .iter()
            .filter(|i| i.parameter == "crl_check_mode" && i.code == ValidationCode::InvalidValue)
            .collect();
        assert_eq!(crl_issues.len(), 1);
    }

    #[test]
    fn validate_valid_crl_check_modes() {
        for mode in &["DISABLED", "ENABLED", "ADVISORY", "0", "1", "2"] {
            let settings = settings_from(&[
                ("account", Setting::String("acct".into())),
                ("user", Setting::String("u".into())),
                ("password", Setting::String("p".into())),
                ("crl_check_mode", Setting::String(mode.to_string())),
            ]);
            let issues = validate_settings(&settings);
            let crl_issues: Vec<_> = issues
                .iter()
                .filter(|i| {
                    i.parameter == "crl_check_mode" && i.code == ValidationCode::InvalidValue
                })
                .collect();
            assert!(
                crl_issues.is_empty(),
                "crl_check_mode '{mode}' should be valid"
            );
        }
    }

    #[test]
    fn validate_unknown_parameter_warns() {
        let settings = settings_from(&[
            ("account", Setting::String("acct".into())),
            ("user", Setting::String("u".into())),
            ("password", Setting::String("p".into())),
            ("totally_bogus_param", Setting::String("x".into())),
        ]);
        let issues = validate_settings(&settings);
        let unknown: Vec<_> = issues
            .iter()
            .filter(|i| {
                i.parameter == "totally_bogus_param"
                    && i.code == ValidationCode::UnknownParameter
                    && i.severity == ValidationSeverity::Warning
            })
            .collect();
        assert_eq!(unknown.len(), 1);
    }

    #[test]
    fn validate_invalid_authenticator() {
        let settings = settings_from(&[
            ("account", Setting::String("acct".into())),
            ("user", Setting::String("u".into())),
            (
                "authenticator",
                Setting::String("BOGUS_AUTHENTICATOR".into()),
            ),
        ]);
        let issues = validate_settings(&settings);
        let auth_issues: Vec<_> = issues
            .iter()
            .filter(|i| i.parameter == "authenticator" && i.code == ValidationCode::InvalidValue)
            .collect();
        assert_eq!(auth_issues.len(), 1);
    }

    #[test]
    fn validate_mfa_missing_password_reports_issue() {
        let settings = settings_from(&[
            ("account", Setting::String("acct".into())),
            ("user", Setting::String("u".into())),
            (
                "authenticator",
                Setting::String("USERNAME_PASSWORD_MFA".into()),
            ),
            ("host", Setting::String("h.com".into())),
        ]);

        let issues = validate_settings(&settings);
        assert!(
            issues.iter().any(|i| {
                i.parameter == "password"
                    && i.code == ValidationCode::MissingRequired
                    && i.message.contains("MFA authentication")
            }),
            "Expected missing password issue for MFA auth, got: {issues:?}"
        );
    }

    #[test]
    fn validate_invalid_authenticator_mentions_mfa() {
        let settings = settings_from(&[
            ("account", Setting::String("acct".into())),
            ("user", Setting::String("u".into())),
            (
                "authenticator",
                Setting::String("BOGUS_AUTHENTICATOR".into()),
            ),
            ("host", Setting::String("h.com".into())),
        ]);

        let issues = validate_settings(&settings);
        let auth_issue = issues
            .iter()
            .find(|i| i.parameter == "authenticator" && i.code == ValidationCode::InvalidValue)
            .expect("expected invalid authenticator issue");

        assert!(
            auth_issue.message.contains("username_password_mfa"),
            "Expected MFA authenticator in message, got: {}",
            auth_issue.message
        );
    }

    #[test]
    fn typed_mfa_auth_matches_legacy_login_method() {
        let mut settings = minimal_mfa_settings();
        settings.insert("passcode".into(), Setting::String("123456".into()));
        settings.insert("passcodeInPassword".into(), Setting::String("true".into()));
        settings.insert(
            "client_store_temporary_credential".into(),
            Setting::String("true".into()),
        );

        let typed =
            login_method_from_auth_config(&ConnectionConfig::build(&settings).unwrap().auth);
        let legacy = LoginMethod::from_settings(&settings).unwrap();

        match (typed, legacy) {
            (
                LoginMethod::UserPasswordMfa {
                    username: typed_username,
                    password: typed_password,
                    passcode_in_password: typed_passcode_in_password,
                    passcode: typed_passcode,
                    client_store_temporary_credential: typed_cache,
                },
                LoginMethod::UserPasswordMfa {
                    username: legacy_username,
                    password: legacy_password,
                    passcode_in_password: legacy_passcode_in_password,
                    passcode: legacy_passcode,
                    client_store_temporary_credential: legacy_cache,
                },
            ) => {
                assert_eq!(typed_username, legacy_username);
                assert_eq!(typed_password.reveal(), legacy_password.reveal());
                assert_eq!(typed_passcode_in_password, legacy_passcode_in_password);
                assert_eq!(typed_cache, legacy_cache);
                assert_eq!(
                    typed_passcode.as_ref().map(|v| v.reveal().as_str()),
                    legacy_passcode.as_ref().map(|v| v.reveal().as_str())
                );
            }
            (typed, legacy) => {
                panic!(
                    "Expected matching MFA login methods, got typed={typed:?}, legacy={legacy:?}"
                )
            }
        }
    }

    #[test]
    fn validate_clean_password_auth_no_errors() {
        let settings = settings_from(&[
            ("account", Setting::String("acct".into())),
            ("user", Setting::String("u".into())),
            ("password", Setting::String("p".into())),
            ("host", Setting::String("h.com".into())),
        ]);
        let issues = validate_settings(&settings);
        let errors: Vec<_> = issues
            .iter()
            .filter(|i| i.severity == ValidationSeverity::Error)
            .collect();
        assert!(errors.is_empty(), "Expected no errors: {errors:?}");
    }

    #[test]
    fn validate_missing_host_without_server_url() {
        let settings = settings_from(&[
            ("account", Setting::String("acct".into())),
            ("user", Setting::String("u".into())),
            ("password", Setting::String("p".into())),
        ]);
        let issues = validate_settings(&settings);
        let host_issues: Vec<_> = issues
            .iter()
            .filter(|i| i.parameter == "host" && i.code == ValidationCode::MissingRequired)
            .collect();
        assert_eq!(host_issues.len(), 1);
    }

    #[test]
    fn validate_server_url_satisfies_host_requirement() {
        let settings = settings_from(&[
            ("account", Setting::String("acct".into())),
            ("user", Setting::String("u".into())),
            ("password", Setting::String("p".into())),
            ("server_url", Setting::String("https://custom.url".into())),
        ]);
        let issues = validate_settings(&settings);
        let host_issues: Vec<_> = issues
            .iter()
            .filter(|i| i.parameter == "host" && i.code == ValidationCode::MissingRequired)
            .collect();
        assert!(host_issues.is_empty());
    }

    // -- validate_settings: OAuth URL-shape tests --

    fn oauth_ac_base_settings() -> Vec<(&'static str, Setting)> {
        vec![
            ("account", Setting::String("acct".into())),
            ("user", Setting::String("u".into())),
            ("host", Setting::String("h.com".into())),
            (
                "authenticator",
                Setting::String("OAUTH_AUTHORIZATION_CODE".into()),
            ),
        ]
    }

    fn oauth_cc_base_settings() -> Vec<(&'static str, Setting)> {
        vec![
            ("account", Setting::String("acct".into())),
            ("user", Setting::String("u".into())),
            ("host", Setting::String("h.com".into())),
            (
                "authenticator",
                Setting::String("OAUTH_CLIENT_CREDENTIALS".into()),
            ),
            ("oauth_client_id", Setting::String("id".into())),
            ("oauth_client_secret", Setting::String("secret".into())),
        ]
    }

    #[test]
    fn validate_oauth_ac_missing_urls_are_allowed() {
        // AC falls back to Snowflake-as-IdP when URLs are absent,
        // so missing oauth_*_url parameters must not produce errors.
        let settings = settings_from(&oauth_ac_base_settings());
        let issues = validate_settings(&settings);
        let url_errors: Vec<_> = issues
            .iter()
            .filter(|i| {
                i.severity == ValidationSeverity::Error
                    && matches!(
                        i.parameter.as_str(),
                        "oauth_authorization_url"
                            | "oauth_token_request_url"
                            | "oauth_redirect_uri"
                    )
            })
            .collect();
        assert!(
            url_errors.is_empty(),
            "Expected no OAuth URL errors when URLs are absent, got: {url_errors:?}"
        );
    }

    #[test]
    fn validate_oauth_ac_valid_urls_no_url_errors() {
        let mut pairs = oauth_ac_base_settings();
        pairs.extend([
            (
                "oauth_authorization_url",
                Setting::String("https://idp.example.com/authorize".into()),
            ),
            (
                "oauth_token_request_url",
                Setting::String("https://idp.example.com/token".into()),
            ),
            (
                "oauth_redirect_uri",
                Setting::String("http://localhost:8080/callback".into()),
            ),
        ]);
        let settings = settings_from(&pairs);
        let issues = validate_settings(&settings);
        let url_errors: Vec<_> = issues
            .iter()
            .filter(|i| {
                i.code == ValidationCode::InvalidValue
                    && matches!(
                        i.parameter.as_str(),
                        "oauth_authorization_url"
                            | "oauth_token_request_url"
                            | "oauth_redirect_uri"
                    )
            })
            .collect();
        assert!(
            url_errors.is_empty(),
            "Expected no URL InvalidValue errors for well-formed URLs, got: {url_errors:?}"
        );
    }

    #[test]
    fn validate_oauth_ac_invalid_authorization_url_reports_invalid_value() {
        let mut pairs = oauth_ac_base_settings();
        pairs.push((
            "oauth_authorization_url",
            Setting::String("not a url".into()),
        ));
        let settings = settings_from(&pairs);
        let issues = validate_settings(&settings);
        assert!(
            issues
                .iter()
                .any(|i| i.parameter == "oauth_authorization_url"
                    && i.code == ValidationCode::InvalidValue),
            "Expected InvalidValue for malformed oauth_authorization_url, got: {issues:?}"
        );
    }

    #[test]
    fn validate_oauth_ac_collects_all_three_url_shape_errors() {
        // Regression test for the Option A fix: a connection string
        // with three malformed AC URLs must report three issues in a
        // single pre-flight pass, not just the first.
        let mut pairs = oauth_ac_base_settings();
        pairs.extend([
            (
                "oauth_authorization_url",
                Setting::String("bad-auth-url".into()),
            ),
            (
                "oauth_token_request_url",
                Setting::String("bad-token-url".into()),
            ),
            ("oauth_redirect_uri", Setting::String("bad-redirect".into())),
        ]);
        let settings = settings_from(&pairs);
        let issues = validate_settings(&settings);

        let url_errors: Vec<_> = issues
            .iter()
            .filter(|i| {
                i.code == ValidationCode::InvalidValue
                    && matches!(
                        i.parameter.as_str(),
                        "oauth_authorization_url"
                            | "oauth_token_request_url"
                            | "oauth_redirect_uri"
                    )
            })
            .collect();
        assert_eq!(
            url_errors.len(),
            3,
            "Expected one InvalidValue issue per malformed URL, got: {issues:?}"
        );
    }

    #[test]
    fn validate_oauth_cc_missing_required_params_reports_three_issues() {
        // Pre-existing presence-check behaviour: CC mandates
        // client_id, client_secret, oauth_token_request_url. All
        // three must be reported together.
        let settings = settings_from(&[
            ("account", Setting::String("acct".into())),
            ("user", Setting::String("u".into())),
            ("host", Setting::String("h.com".into())),
            (
                "authenticator",
                Setting::String("OAUTH_CLIENT_CREDENTIALS".into()),
            ),
        ]);
        let issues = validate_settings(&settings);

        for missing in [
            "oauth_client_id",
            "oauth_client_secret",
            "oauth_token_request_url",
        ] {
            assert!(
                issues
                    .iter()
                    .any(|i| i.parameter == missing && i.code == ValidationCode::MissingRequired),
                "Expected MissingRequired for '{missing}', got: {issues:?}"
            );
        }
    }

    #[test]
    fn validate_oauth_cc_missing_token_url_does_not_report_invalid_value() {
        // When oauth_token_request_url is absent we must surface
        // MissingRequired only — never both MissingRequired *and*
        // InvalidValue for the same parameter.
        let settings = settings_from(&oauth_cc_base_settings());
        let issues = validate_settings(&settings);

        let invalid_value_issues: Vec<_> = issues
            .iter()
            .filter(|i| {
                i.parameter == "oauth_token_request_url" && i.code == ValidationCode::InvalidValue
            })
            .collect();
        assert!(
            invalid_value_issues.is_empty(),
            "Absent oauth_token_request_url must not produce InvalidValue, got: {issues:?}"
        );
    }

    #[test]
    fn validate_oauth_cc_invalid_token_url_reports_invalid_value() {
        let mut pairs = oauth_cc_base_settings();
        pairs.push((
            "oauth_token_request_url",
            Setting::String("not a url".into()),
        ));
        let settings = settings_from(&pairs);
        let issues = validate_settings(&settings);

        // Presence check is satisfied (value is non-empty), so
        // MissingRequired must not fire for this parameter.
        assert!(
            !issues
                .iter()
                .any(|i| i.parameter == "oauth_token_request_url"
                    && i.code == ValidationCode::MissingRequired),
            "MissingRequired must not fire when value is present, got: {issues:?}"
        );
        assert!(
            issues
                .iter()
                .any(|i| i.parameter == "oauth_token_request_url"
                    && i.code == ValidationCode::InvalidValue),
            "Expected InvalidValue for malformed oauth_token_request_url, got: {issues:?}"
        );
    }

    #[test]
    fn get_bool_rejects_unrecognized_string() {
        let settings = settings_from(&[
            ("account", Setting::String("acct".into())),
            ("user", Setting::String("u".into())),
            ("password", Setting::String("p".into())),
            ("host", Setting::String("h.com".into())),
            ("verify_hostname", Setting::String("ture".into())),
        ]);
        let config = ConnectionConfig::build(&settings).unwrap();
        // Unrecognized string falls through to default (true), not silently false
        assert!(config.tls.verify_hostname);
    }

    #[test]
    fn crl_check_mode_builds_correct_enum() {
        for (input, expected) in [
            ("DISABLED", CertRevocationCheckMode::Disabled),
            ("0", CertRevocationCheckMode::Disabled),
            ("ENABLED", CertRevocationCheckMode::Enabled),
            ("1", CertRevocationCheckMode::Enabled),
            ("ADVISORY", CertRevocationCheckMode::Advisory),
            ("2", CertRevocationCheckMode::Advisory),
        ] {
            let mut settings = minimal_password_settings();
            settings.insert("crl_check_mode".into(), Setting::String(input.into()));
            let config = ConnectionConfig::build(&settings).unwrap();
            assert_eq!(
                config.tls.crl_config.check_mode, expected,
                "crl_check_mode '{input}' should produce {expected:?}"
            );
        }
    }

    #[test]
    fn build_succeeds_with_account_only_no_host() {
        use crate::config::path_resolver::ConfigPaths;
        use crate::config::resolver;

        let mut explicit = ParamStore::new();
        explicit.insert("account".into(), Setting::String("myaccount".into()));
        explicit.insert("user".into(), Setting::String("myuser".into()));
        explicit.insert("password".into(), Setting::String("mypassword".into()));

        let paths = ConfigPaths {
            config_file: None,
            connections_file: None,
        };
        let resolved = resolver::resolve_with_paths(&explicit, &paths, false).unwrap();
        let config = ConnectionConfig::build(&resolved).unwrap();

        assert_eq!(config.server.account, "myaccount");
        assert_eq!(
            config.server.server_url,
            "https://myaccount.snowflakecomputing.com"
        );
    }

    // ── WIF tests ──────────────────────────────────────────────────────

    fn wif_base_settings(provider: &str) -> Vec<(&'static str, Setting)> {
        vec![
            ("account", Setting::String("acct".into())),
            (
                "host",
                Setting::String("acct.snowflakecomputing.com".into()),
            ),
            ("authenticator", Setting::String("WORKLOAD_IDENTITY".into())),
            (
                "workload_identity_provider",
                Setting::String(provider.into()),
            ),
        ]
    }

    #[test]
    fn validate_wif_missing_provider_reports_issue() {
        let settings = settings_from(&[
            ("account", Setting::String("acct".into())),
            (
                "host",
                Setting::String("acct.snowflakecomputing.com".into()),
            ),
            ("authenticator", Setting::String("WORKLOAD_IDENTITY".into())),
        ]);
        let issues = validate_settings(&settings);
        assert!(
            issues.iter().any(|i| {
                i.parameter == "workload_identity_provider"
                    && i.code == ValidationCode::MissingRequired
            }),
            "Expected MissingRequired for missing workload_identity_provider, got: {issues:?}"
        );
    }

    #[test]
    fn validate_wif_invalid_provider_reports_issue() {
        let settings = settings_from(&[
            ("account", Setting::String("acct".into())),
            (
                "host",
                Setting::String("acct.snowflakecomputing.com".into()),
            ),
            ("authenticator", Setting::String("WORKLOAD_IDENTITY".into())),
            (
                "workload_identity_provider",
                Setting::String("CLOUD9".into()),
            ),
        ]);
        let issues = validate_settings(&settings);
        assert!(
            issues.iter().any(|i| {
                i.parameter == "workload_identity_provider"
                    && i.code == ValidationCode::InvalidValue
            }),
            "Expected InvalidValue for unknown provider, got: {issues:?}"
        );
    }

    #[test]
    fn validate_wif_oidc_requires_token() {
        let settings = settings_from(&wif_base_settings("OIDC"));
        let issues = validate_settings(&settings);
        assert!(
            issues
                .iter()
                .any(|i| { i.parameter == "token" && i.code == ValidationCode::MissingRequired }),
            "Expected MissingRequired for token with OIDC provider, got: {issues:?}"
        );
    }

    #[test]
    fn validate_wif_oidc_with_impersonation_path_emits_error() {
        let mut pairs = wif_base_settings("OIDC");
        pairs.push(("token", Setting::String("my-oidc-jwt".into())));
        pairs.push((
            "workload_identity_impersonation_path",
            Setting::String("sa@project.iam.gserviceaccount.com".into()),
        ));
        let settings = settings_from(&pairs);
        let issues = validate_settings(&settings);
        assert!(
            issues.iter().any(|i| {
                i.parameter == "workload_identity_impersonation_path"
                    && i.severity == ValidationSeverity::Error
                    && i.code == ValidationCode::ConflictingWifParameters
            }),
            "Expected ConflictingWifParameters error for impersonation_path with OIDC, got: {issues:?}"
        );
    }

    #[test]
    fn validate_wif_aws_with_impersonation_path_no_conflict() {
        let mut pairs = wif_base_settings("AWS");
        pairs.push((
            "workload_identity_impersonation_path",
            Setting::String("arn:aws:iam::123:role/A".into()),
        ));
        let settings = settings_from(&pairs);
        let issues = validate_settings(&settings);
        assert!(
            issues
                .iter()
                .all(|i| i.parameter != "workload_identity_impersonation_path"),
            "AWS + impersonation_path should not conflict, got: {issues:?}"
        );
    }

    #[test]
    fn validate_wif_user_not_required() {
        let settings = settings_from(&wif_base_settings("AWS"));
        let issues = validate_settings(&settings);
        let user_errors: Vec<_> = issues
            .iter()
            .filter(|i| i.parameter == "user" && i.code == ValidationCode::MissingRequired)
            .collect();
        assert!(
            user_errors.is_empty(),
            "WORKLOAD_IDENTITY should not require 'user', got: {user_errors:?}"
        );
    }

    #[test]
    fn validate_wif_providers_accepted_case_insensitive() {
        for provider in &[
            "aws", "AWS", "azure", "Azure", "AZURE", "gcp", "GCP", "oidc", "OIDC",
        ] {
            let mut settings = wif_base_settings(provider);
            if provider.eq_ignore_ascii_case("OIDC") {
                settings.push(("token", Setting::String("tok".into())));
            }
            let settings = settings_from(&settings);
            let issues = validate_settings(&settings);
            let provider_errors: Vec<_> = issues
                .iter()
                .filter(|i| {
                    i.parameter == "workload_identity_provider"
                        && i.code == ValidationCode::InvalidValue
                })
                .collect();
            assert!(
                provider_errors.is_empty(),
                "Provider '{provider}' should be valid (case-insensitive), got: {issues:?}"
            );
        }
    }

    #[test]
    fn build_wif_aws_auth() {
        let settings = settings_from(&wif_base_settings("AWS"));
        let config = ConnectionConfig::build(&settings).unwrap();
        match &config.auth {
            AuthConfig::WorkloadIdentity(cfg) => {
                assert_eq!(
                    cfg.provider,
                    crate::config::rest_parameters::WifProvider::Aws
                );
                assert!(cfg.impersonation_path.is_empty());
                assert!(cfg.entra_resource.is_none());
                assert!(cfg.oidc_token.is_none());
            }
            other => panic!("Expected WorkloadIdentity auth, got {other:?}"),
        }
    }

    #[test]
    fn build_wif_aws_with_impersonation_path() {
        let mut pairs = wif_base_settings("AWS");
        pairs.push((
            "workload_identity_impersonation_path",
            Setting::String("arn:aws:iam::123:role/A,arn:aws:iam::456:role/B".into()),
        ));
        let settings = settings_from(&pairs);
        let config = ConnectionConfig::build(&settings).unwrap();
        match &config.auth {
            AuthConfig::WorkloadIdentity(cfg) => {
                assert_eq!(
                    cfg.impersonation_path,
                    vec![
                        "arn:aws:iam::123:role/A".to_string(),
                        "arn:aws:iam::456:role/B".to_string(),
                    ]
                );
            }
            other => panic!("Expected WorkloadIdentity auth, got {other:?}"),
        }
    }

    #[test]
    fn build_wif_gcp_auth() {
        let settings = settings_from(&wif_base_settings("GCP"));
        let config = ConnectionConfig::build(&settings).unwrap();
        match &config.auth {
            AuthConfig::WorkloadIdentity(cfg) => {
                assert_eq!(
                    cfg.provider,
                    crate::config::rest_parameters::WifProvider::Gcp
                );
                assert!(cfg.impersonation_path.is_empty());
                assert!(cfg.entra_resource.is_none());
                assert!(cfg.oidc_token.is_none());
            }
            other => panic!("Expected WorkloadIdentity auth, got {other:?}"),
        }
    }

    #[test]
    fn build_wif_gcp_auth_with_impersonation_path() {
        let mut pairs = wif_base_settings("GCP");
        pairs.push((
            "workload_identity_impersonation_path",
            Setting::String(
                "sa-a@proj.iam.gserviceaccount.com,sa-b@proj.iam.gserviceaccount.com".into(),
            ),
        ));
        let settings = settings_from(&pairs);
        let config = ConnectionConfig::build(&settings).unwrap();
        match &config.auth {
            AuthConfig::WorkloadIdentity(cfg) => {
                assert_eq!(
                    cfg.provider,
                    crate::config::rest_parameters::WifProvider::Gcp
                );
                assert_eq!(
                    cfg.impersonation_path,
                    vec![
                        "sa-a@proj.iam.gserviceaccount.com".to_string(),
                        "sa-b@proj.iam.gserviceaccount.com".to_string(),
                    ]
                );
            }
            other => panic!("Expected WorkloadIdentity auth, got {other:?}"),
        }
    }

    #[test]
    fn build_wif_oidc_with_token() {
        let mut pairs = wif_base_settings("OIDC");
        pairs.push(("token", Setting::String("my-oidc-jwt".into())));
        let settings = settings_from(&pairs);
        let config = ConnectionConfig::build(&settings).unwrap();
        match &config.auth {
            AuthConfig::WorkloadIdentity(cfg) => {
                assert_eq!(
                    cfg.provider,
                    crate::config::rest_parameters::WifProvider::Oidc
                );
                assert_eq!(
                    cfg.oidc_token.as_ref().map(|t| t.reveal().to_string()),
                    Some("my-oidc-jwt".to_string())
                );
            }
            other => panic!("Expected WorkloadIdentity auth, got {other:?}"),
        }
    }

    #[test]
    fn validate_wif_error_message_mentions_allowed_values() {
        let settings = settings_from(&[
            ("account", Setting::String("acct".into())),
            (
                "host",
                Setting::String("acct.snowflakecomputing.com".into()),
            ),
            ("authenticator", Setting::String("WORKLOAD_IDENTITY".into())),
        ]);
        let issues = validate_settings(&settings);
        assert!(
            issues.iter().any(|i| {
                i.message.contains("WORKLOAD_IDENTITY")
                    || i.message.contains("workload_identity_provider")
            }),
            "Error message should mention workload_identity_provider: {issues:?}"
        );
    }

    #[test]
    fn build_wif_azure_with_entra_resource() {
        let mut pairs = wif_base_settings("AZURE");
        pairs.push((
            "workload_identity_entra_resource",
            Setting::String("api://my-custom-app".into()),
        ));
        let settings = settings_from(&pairs);
        let config = ConnectionConfig::build(&settings).unwrap();
        match &config.auth {
            AuthConfig::WorkloadIdentity(cfg) => {
                assert_eq!(
                    cfg.provider,
                    crate::config::rest_parameters::WifProvider::Azure
                );
                assert_eq!(cfg.entra_resource.as_deref(), Some("api://my-custom-app"));
            }
            other => panic!("Expected WorkloadIdentity auth, got {other:?}"),
        }
    }

    #[test]
    fn validate_azure_impersonation_single_hop_accepted() {
        let mut pairs = wif_base_settings("AZURE");
        pairs.push((
            "workload_identity_impersonation_path",
            Setting::String("my-sp-client-id".into()),
        ));
        let settings = settings_from(&pairs);
        let config = ConnectionConfig::build(&settings).unwrap();
        match &config.auth {
            AuthConfig::WorkloadIdentity(cfg) => {
                assert_eq!(cfg.impersonation_path, vec!["my-sp-client-id".to_string()]);
            }
            other => panic!("Expected WorkloadIdentity auth, got {other:?}"),
        }
        let issues = validate_settings(&settings);
        assert!(
            !issues
                .iter()
                .any(|i| i.parameter == "workload_identity_impersonation_path"),
            "Single-hop Azure impersonation should pass validation, got: {issues:?}"
        );
    }

    #[test]
    fn validate_azure_impersonation_multi_hop_rejected() {
        let mut pairs = wif_base_settings("AZURE");
        pairs.push((
            "workload_identity_impersonation_path",
            Setting::String("sp-client-id-1,sp-client-id-2".into()),
        ));
        let settings = settings_from(&pairs);
        let issues = validate_settings(&settings);
        assert!(
            issues.iter().any(|i| {
                i.parameter == "workload_identity_impersonation_path"
                    && i.code == ValidationCode::InvalidValue
                    && i.message.contains("single-hop")
            }),
            "Multi-hop Azure impersonation should fail validation with InvalidValue, got: {issues:?}"
        );
    }

    #[test]
    fn validate_azure_no_impersonation_accepted() {
        let settings = settings_from(&wif_base_settings("AZURE"));
        let issues = validate_settings(&settings);
        assert!(
            !issues
                .iter()
                .any(|i| i.parameter == "workload_identity_impersonation_path"),
            "Azure without impersonation should pass, got: {issues:?}"
        );
    }

    // ── WIF cross-param guard: WIF-only params require WORKLOAD_IDENTITY ──

    #[test]
    fn validate_wif_provider_with_non_wif_auth_emits_error() {
        let settings = settings_from(&[
            ("account", Setting::String("acct".into())),
            ("user", Setting::String("u".into())),
            ("password", Setting::String("p".into())),
            ("authenticator", Setting::String("snowflake".into())),
            ("workload_identity_provider", Setting::String("aws".into())),
        ]);
        let issues = validate_settings(&settings);
        assert!(
            issues.iter().any(|i| {
                i.parameter == "workload_identity_provider"
                    && i.code == ValidationCode::ConflictingWifParameters
                    && i.severity == ValidationSeverity::Error
            }),
            "Expected ConflictingWifParameters warning for workload_identity_provider, got: {issues:?}"
        );
    }

    #[test]
    fn validate_wif_entra_resource_with_non_wif_auth_emits_error() {
        let settings = settings_from(&[
            ("account", Setting::String("acct".into())),
            ("user", Setting::String("u".into())),
            ("password", Setting::String("p".into())),
            (
                "workload_identity_entra_resource",
                Setting::String("https://resource.example.com".into()),
            ),
        ]);
        let issues = validate_settings(&settings);
        assert!(
            issues.iter().any(|i| {
                i.parameter == "workload_identity_entra_resource"
                    && i.code == ValidationCode::ConflictingWifParameters
            }),
            "Expected ConflictingWifParameters for workload_identity_entra_resource, got: {issues:?}"
        );
    }

    #[test]
    fn validate_wif_impersonation_path_with_non_wif_auth_emits_error() {
        let settings = settings_from(&[
            ("account", Setting::String("acct".into())),
            ("user", Setting::String("u".into())),
            ("password", Setting::String("p".into())),
            (
                "workload_identity_impersonation_path",
                Setting::String("arn:aws:iam::123:role/A".into()),
            ),
        ]);
        let issues = validate_settings(&settings);
        assert!(
            issues.iter().any(|i| {
                i.parameter == "workload_identity_impersonation_path"
                    && i.code == ValidationCode::ConflictingWifParameters
            }),
            "Expected ConflictingWifParameters for workload_identity_impersonation_path, got: {issues:?}"
        );
    }

    #[test]
    fn validate_wif_params_absent_authenticator_emits_error() {
        // absent authenticator defaults to non-WIF → guard must fire
        let settings = settings_from(&[
            ("account", Setting::String("acct".into())),
            ("user", Setting::String("u".into())),
            ("workload_identity_provider", Setting::String("gcp".into())),
        ]);
        let issues = validate_settings(&settings);
        assert!(
            issues.iter().any(|i| {
                i.parameter == "workload_identity_provider"
                    && i.code == ValidationCode::ConflictingWifParameters
            }),
            "Expected ConflictingWifParameters when authenticator absent, got: {issues:?}"
        );
    }

    #[test]
    fn validate_wif_params_with_wif_auth_no_conflict_error() {
        // WIF params + WORKLOAD_IDENTITY auth → no ConflictingWifParameters
        let mut pairs = wif_base_settings("AWS");
        pairs.push((
            "workload_identity_entra_resource",
            Setting::String("https://resource.example.com".into()),
        ));
        pairs.push((
            "workload_identity_impersonation_path",
            Setting::String("arn:aws:iam::123:role/A".into()),
        ));
        let settings = settings_from(&pairs);
        let issues = validate_settings(&settings);
        assert!(
            !issues
                .iter()
                .any(|i| i.code == ValidationCode::ConflictingWifParameters
                    && [
                        "workload_identity_provider",
                        "workload_identity_entra_resource",
                        "workload_identity_impersonation_path"
                    ]
                    .contains(&i.parameter.as_str())),
            "WIF params with WORKLOAD_IDENTITY auth should not emit ConflictingWifParameters, got: {issues:?}"
        );
    }
}
