use std::collections::HashMap;
use std::fmt;
use std::fs;
use std::path::PathBuf;

use base64::{Engine as _, engine::general_purpose};
use chrono::Duration;
use openssl::pkey::PKey;
use snafu::OptionExt;
use url::Url;

use crate::config::ParamStore;
use crate::config::param_names::*;
use crate::config::rest_parameters::{
    ClientInfo, DEFAULT_AUTHENTICATION_TIMEOUT_SECS, LoginMethod, LoginParameters, NativeOktaConfig,
};
use crate::config::settings::Setting;
use crate::config::{
    ConfigError, ConflictingParametersSnafu, InvalidParameterValueSnafu, MissingParameterSnafu,
    ValidationFailedSnafu,
};
use crate::crl::config::{CertRevocationCheckMode, CrlConfig};
use crate::sensitive::SensitiveString;
use crate::tls::config::TlsConfig;

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

// ---------------------------------------------------------------------------
// Server URL derivation (mirrored from rest_parameters::get_server_url)
// ---------------------------------------------------------------------------

fn derive_server_url(settings: &ParamStore) -> Result<String, ConfigError> {
    if let Some(url) = settings.get_string(SERVER_URL) {
        return Ok(url);
    }

    let protocol = settings
        .get_string(PROTOCOL)
        .unwrap_or_else(|| "https".to_string());
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
// TLS / CRL config building (mirrored from tls::config / crl::config)
// ---------------------------------------------------------------------------

fn build_crl_config(settings: &ParamStore) -> CrlConfig {
    // TODO: make matching case-insensitive (e.g. "disabled", "Enabled")
    let check_mode = match settings.get_string(CRL_CHECK_MODE).as_deref() {
        Some("0") | Some("DISABLED") | None => CertRevocationCheckMode::Disabled,
        Some("1") | Some("ENABLED") => CertRevocationCheckMode::Enabled,
        Some("2") | Some("ADVISORY") => CertRevocationCheckMode::Advisory,
        Some(other) => {
            tracing::warn!("Unknown crl_check_mode: {other}, using DISABLED");
            CertRevocationCheckMode::Disabled
        }
    };

    let enable_disk_caching = settings.get_bool(CRL_ENABLE_DISK_CACHING).unwrap_or(true);
    let enable_memory_caching = settings.get_bool(CRL_ENABLE_MEMORY_CACHING).unwrap_or(true);
    let cache_dir = settings.get_string(CRL_CACHE_DIR).map(PathBuf::from);
    let validity_time = settings
        .get_int(CRL_VALIDITY_TIME)
        .map(Duration::days)
        .unwrap_or(Duration::days(10));
    let allow_certificates_without_crl_url = settings
        .get_bool(CRL_ALLOW_CERTIFICATES_WITHOUT_CRL_URL)
        .unwrap_or(false);
    let http_timeout = settings
        .get_int(CRL_HTTP_TIMEOUT)
        .map(Duration::seconds)
        .unwrap_or(Duration::seconds(30));
    let connection_timeout = settings
        .get_int(CRL_CONNECTION_TIMEOUT)
        .map(Duration::seconds)
        .unwrap_or(Duration::seconds(10));

    CrlConfig {
        check_mode,
        enable_disk_caching,
        enable_memory_caching,
        cache_dir,
        validity_time,
        allow_certificates_without_crl_url,
        http_timeout,
        connection_timeout,
    }
}

fn build_tls_config(settings: &ParamStore) -> TlsConfig {
    let crl_config = build_crl_config(settings);
    let custom_root_store_path = settings
        .get_string(CUSTOM_ROOT_STORE_PATH)
        .map(PathBuf::from);
    let verify_hostname = settings.get_bool(VERIFY_HOSTNAME).unwrap_or(true);
    let verify_certificates = settings.get_bool(VERIFY_CERTIFICATES).unwrap_or(true);

    TlsConfig {
        crl_config,
        custom_root_store_path,
        verify_hostname,
        verify_certificates,
    }
}

// ---------------------------------------------------------------------------
// Auth config building (mirrored from rest_parameters::LoginMethod)
// ---------------------------------------------------------------------------

fn build_auth_config(settings: &ParamStore) -> Result<AuthConfig, ConfigError> {
    let authenticator = settings.get_string(AUTHENTICATOR).unwrap_or_default();

    let use_jwt = authenticator == "SNOWFLAKE_JWT"
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

    match authenticator.as_str() {
        "SNOWFLAKE_PASSWORD" | "" => Ok(AuthConfig::Password {
            user: non_empty_string(settings, USER).context(MissingParameterSnafu {
                parameter: String::from(USER),
            })?,
            password: settings.get_sensitive_string(PASSWORD).context(
                MissingParameterSnafu {
                    parameter: String::from(PASSWORD),
                },
            )?,
        }),
        "USERNAME_PASSWORD_MFA" => Ok(AuthConfig::Mfa {
            user: non_empty_string(settings, USER).context(MissingParameterSnafu {
                parameter: String::from(USER),
            })?,
            password: settings.get_sensitive_string(PASSWORD).context(
                MissingParameterSnafu {
                    parameter: String::from(PASSWORD),
                },
            )?,
            passcode_in_password: settings.get_bool(PASSCODE_IN_PASSWORD).unwrap_or(false),
            passcode: settings.get_sensitive_string(PASSCODE),
            client_store_temporary_credential: settings
                .get_bool(CLIENT_STORE_TEMPORARY_CREDENTIAL)
                .unwrap_or(false),
        }),
        "PROGRAMMATIC_ACCESS_TOKEN" => Ok(AuthConfig::Pat {
            user: non_empty_string(settings, USER).context(MissingParameterSnafu {
                parameter: String::from(USER),
            })?,
            token: settings.get_sensitive_string(TOKEN)
                .context(MissingParameterSnafu {
                    parameter: String::from(TOKEN),
                })?,
        }),
        _ if authenticator.to_ascii_lowercase().starts_with("https://") => {
            let okta_url = Url::parse(&authenticator).map_err(|_| {
                InvalidParameterValueSnafu {
                    parameter: String::from(AUTHENTICATOR),
                    value: authenticator.clone(),
                    explanation: "The authenticator URL is not a valid URL".to_string(),
                }
                .build()
            })?;

            let authentication_timeout_secs = settings
                .get_int(AUTHENTICATION_TIMEOUT)
                .and_then(|v| u64::try_from(v).ok())
                .unwrap_or(DEFAULT_AUTHENTICATION_TIMEOUT_SECS);

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
                disable_saml_url_check: settings
                    .get_bool(DISABLE_SAML_URL_CHECK)
                    .unwrap_or(false),
                authentication_timeout_secs,
            }))
        }
        _ => InvalidParameterValueSnafu {
            parameter: String::from(AUTHENTICATOR),
            value: authenticator,
            explanation: "Allowed values are SNOWFLAKE_JWT, SNOWFLAKE_PASSWORD, PROGRAMMATIC_ACCESS_TOKEN, USERNAME_PASSWORD_MFA, or an https:// URL for native Okta SSO".to_string(),
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
    /// collected (not just the first) via `ConfigError::ValidationFailed`.
    /// Runtime errors that go beyond static validation (e.g. base64
    /// decoding failures, file I/O) are still returned individually.
    pub fn build(settings: &ParamStore) -> Result<Self, ConfigError> {
        let issues = validate_settings(settings);
        let errors: Vec<_> = issues
            .into_iter()
            .filter(|i| i.severity == ValidationSeverity::Error)
            .collect();
        if !errors.is_empty() {
            return ValidationFailedSnafu { issues: errors }.fail();
        }

        let account = settings
            .get_string(ACCOUNT)
            .context(MissingParameterSnafu {
                parameter: String::from(ACCOUNT),
            })?;
        let server_url = derive_server_url(settings)?;
        let auth = build_auth_config(settings)?;
        let tls = build_tls_config(settings);

        let session = SessionContext {
            database: settings.get_string(DATABASE),
            schema: settings.get_string(SCHEMA),
            warehouse: settings.get_string(WAREHOUSE),
            role: settings.get_string(ROLE),
        };

        Ok(Self {
            server: ServerConfig {
                account,
                server_url,
            },
            auth,
            session,
            tls,
        })
    }
}

fn login_method_from_auth_config(auth: &AuthConfig) -> LoginMethod {
    match auth {
        AuthConfig::Password { user, password } => LoginMethod::Password {
            username: user.clone(),
            password: password.clone(),
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
        }
    }
}

// ---------------------------------------------------------------------------
// validate_settings – pre-flight check that collects all issues
// ---------------------------------------------------------------------------

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

    // --- MissingRequired: user ---
    if non_empty_string(settings, USER).is_none() {
        issues.push(ValidationIssue {
            severity: ValidationSeverity::Error,
            parameter: USER.into(),
            message: "Missing required parameter 'user'".into(),
            code: ValidationCode::MissingRequired,
        });
    }

    // --- Auth-specific checks based on authenticator ---
    let authenticator = settings.get_string(AUTHENTICATOR).unwrap_or_default();
    match authenticator.as_str() {
        "" if has_private_key_params(settings) => {
            // Empty authenticator + private key params → auto-JWT, no password needed
        }
        "SNOWFLAKE_PASSWORD" if has_private_key_params(settings) => {
            issues.push(ValidationIssue {
                severity: ValidationSeverity::Error,
                parameter: AUTHENTICATOR.into(),
                message: "Cannot specify 'SNOWFLAKE_PASSWORD' authenticator together with \
                          private key parameters; use 'SNOWFLAKE_JWT' or remove the private \
                          key parameters"
                    .into(),
                code: ValidationCode::ConflictingParameters,
            });
        }
        "SNOWFLAKE_PASSWORD" | "" => {
            if settings.get_string(PASSWORD).is_none() {
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
            if settings.get_string(PASSWORD).is_none() {
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
            if settings.get_string(TOKEN).is_none() {
                issues.push(ValidationIssue {
                    severity: ValidationSeverity::Error,
                    parameter: TOKEN.into(),
                    message: "Missing required parameter 'token' for PAT authentication".into(),
                    code: ValidationCode::MissingRequired,
                });
            }
        }
        other if other.to_ascii_lowercase().starts_with("https://") => {
            if Url::parse(other).is_err() {
                issues.push(ValidationIssue {
                    severity: ValidationSeverity::Error,
                    parameter: AUTHENTICATOR.into(),
                    message: format!("The authenticator URL '{other}' is not a valid URL"),
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
        other => {
            issues.push(ValidationIssue {
                severity: ValidationSeverity::Error,
                parameter: AUTHENTICATOR.into(),
                message: format!(
                    "Invalid authenticator '{other}'. Allowed: SNOWFLAKE_PASSWORD, SNOWFLAKE_JWT, PROGRAMMATIC_ACCESS_TOKEN, USERNAME_PASSWORD_MFA, or an https:// URL for native Okta SSO"
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

    fn settings_from(pairs: &[(&str, Setting)]) -> ParamStore {
        let mut settings = ParamStore::new();
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
            AuthConfig::Password { user, password } => {
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
            ConfigError::ValidationFailed { ref issues, .. } => {
                assert!(
                    issues
                        .iter()
                        .any(|i| i.parameter == "account"
                            && i.code == ValidationCode::MissingRequired),
                    "Expected MissingRequired for 'account', got: {issues:?}"
                );
            }
            other => panic!("Expected ValidationFailed, got: {other}"),
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
            ConfigError::ValidationFailed { ref issues, .. } => {
                assert!(
                    issues
                        .iter()
                        .any(|i| i.code == ValidationCode::ConflictingParameters),
                    "Expected ConflictingParameters issue, got: {issues:?}"
                );
            }
            other => panic!("Expected ValidationFailed, got: {other}"),
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
    fn validate_returns_all_issues_not_just_first() {
        let settings = ParamStore::new();
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
            ("authenticator", Setting::String("OAUTH".into())),
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
            ("authenticator", Setting::String("OAUTH".into())),
            ("host", Setting::String("h.com".into())),
        ]);

        let issues = validate_settings(&settings);
        let auth_issue = issues
            .iter()
            .find(|i| i.parameter == "authenticator" && i.code == ValidationCode::InvalidValue)
            .expect("expected invalid authenticator issue");

        assert!(
            auth_issue.message.contains("USERNAME_PASSWORD_MFA"),
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
}
