use std::collections::HashMap;
use std::fmt;
use std::fs;
use std::path::PathBuf;

use base64::{Engine as _, engine::general_purpose};
use chrono::Duration;
use openssl::pkey::PKey;
use snafu::OptionExt;

use crate::config::settings::Setting;
use crate::config::{
    ConfigError, ConflictingParametersSnafu, InvalidParameterValueSnafu, MissingParameterSnafu,
    ValidationFailedSnafu,
};
use crate::crl::config::{CertRevocationCheckMode, CrlConfig};
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
        password: String,
    },
    Jwt {
        user: String,
        private_key_pem: String,
        passphrase: Option<String>,
    },
    Pat {
        user: String,
        token: String,
    },
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
// Setting helpers – extract typed values from HashMap<String, Setting>
// ---------------------------------------------------------------------------

fn get_string(settings: &HashMap<String, Setting>, key: &str) -> Option<String> {
    match settings.get(key)? {
        Setting::String(s) => Some(s.clone()),
        _ => None,
    }
}

fn get_int(settings: &HashMap<String, Setting>, key: &str) -> Option<i64> {
    match settings.get(key)? {
        Setting::Int(i) => Some(*i),
        _ => None,
    }
}

/// Read a boolean parameter.  Checks `Setting::Bool` first, then falls back
/// to `Setting::String` with `"true"` / `"false"` parsing for backward
/// compatibility with TOML-loaded values.  Unrecognized strings (typos like
/// `"ture"`) return `None` so the caller falls through to its default rather
/// than silently degrading to `false`.
fn get_bool(settings: &HashMap<String, Setting>, key: &str) -> Option<bool> {
    match settings.get(key)? {
        Setting::Bool(b) => Some(*b),
        Setting::String(s) => match s.to_lowercase().as_str() {
            "true" => Some(true),
            "false" => Some(false),
            _ => None,
        },
        _ => None,
    }
}

// ---------------------------------------------------------------------------
// Private key helpers (mirrored from rest_parameters.rs)
// ---------------------------------------------------------------------------

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

fn read_private_key(settings: &HashMap<String, Setting>) -> Result<String, ConfigError> {
    let has_private_key = settings.get("private_key").is_some();
    let has_private_key_file = get_string(settings, "private_key_file").is_some();

    if has_private_key && has_private_key_file {
        return ConflictingParametersSnafu {
            explanation:
                "Both 'private_key' and 'private_key_file' are set. Please provide only one."
                    .to_string(),
        }
        .fail();
    }

    // Bytes (DER from Python)
    if let Some(Setting::Bytes(private_key_bytes)) = settings.get("private_key") {
        return der_to_pem(private_key_bytes);
    }

    // String (base64-encoded)
    if let Some(private_key_base64) = get_string(settings, "private_key") {
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

        if private_key_bytes.starts_with(b"-----BEGIN") {
            return String::from_utf8(private_key_bytes).map_err(|e| {
                InvalidParameterValueSnafu {
                    parameter: "private_key",
                    value: "(redacted)".to_string(),
                    explanation: format!("Private key is not valid UTF-8: {e}"),
                }
                .build()
            });
        }

        return der_to_pem(&private_key_bytes);
    }

    // File path
    if let Some(private_key_file) = get_string(settings, "private_key_file") {
        let private_key = fs::read_to_string(&private_key_file).map_err(|e| {
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
    .fail()
}

fn has_private_key_params(settings: &HashMap<String, Setting>) -> bool {
    settings.get("private_key").is_some() || get_string(settings, "private_key_file").is_some()
}

// ---------------------------------------------------------------------------
// Server URL derivation (mirrored from rest_parameters::get_server_url)
// ---------------------------------------------------------------------------

fn derive_server_url(settings: &HashMap<String, Setting>) -> Result<String, ConfigError> {
    if let Some(url) = get_string(settings, "server_url") {
        return Ok(url);
    }

    let protocol = get_string(settings, "protocol").unwrap_or_else(|| "https".to_string());
    let host = get_string(settings, "host").context(MissingParameterSnafu { parameter: "host" })?;
    if protocol != "https" && protocol != "http" {
        tracing::warn!(
            "Unexpected protocol specified during server url construction: {protocol}"
        );
    }

    let base_url = format!("{protocol}://{host}");
    if let Some(port) = get_int(settings, "port") {
        return Ok(format!("{base_url}:{port}"));
    }

    Ok(base_url)
}

// ---------------------------------------------------------------------------
// TLS / CRL config building (mirrored from tls::config / crl::config)
// ---------------------------------------------------------------------------

fn build_crl_config(settings: &HashMap<String, Setting>) -> CrlConfig {
    // TODO: make matching case-insensitive (e.g. "disabled", "Enabled")
    let check_mode = match get_string(settings, "crl_check_mode").as_deref() {
        Some("0") | Some("DISABLED") | None => CertRevocationCheckMode::Disabled,
        Some("1") | Some("ENABLED") => CertRevocationCheckMode::Enabled,
        Some("2") | Some("ADVISORY") => CertRevocationCheckMode::Advisory,
        Some(other) => {
            tracing::warn!("Unknown crl_check_mode: {other}, using DISABLED");
            CertRevocationCheckMode::Disabled
        }
    };

    let enable_disk_caching = get_bool(settings, "crl_enable_disk_caching").unwrap_or(true);
    let enable_memory_caching = get_bool(settings, "crl_enable_memory_caching").unwrap_or(true);
    let cache_dir = get_string(settings, "crl_cache_dir").map(PathBuf::from);
    let validity_time = get_int(settings, "crl_validity_time")
        .map(Duration::days)
        .unwrap_or(Duration::days(10));
    let allow_certificates_without_crl_url =
        get_bool(settings, "crl_allow_certificates_without_crl_url").unwrap_or(false);
    let http_timeout = get_int(settings, "crl_http_timeout")
        .map(Duration::seconds)
        .unwrap_or(Duration::seconds(30));
    let connection_timeout = get_int(settings, "crl_connection_timeout")
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

fn build_tls_config(settings: &HashMap<String, Setting>) -> TlsConfig {
    let crl_config = build_crl_config(settings);
    let custom_root_store_path = get_string(settings, "custom_root_store_path").map(PathBuf::from);
    let verify_hostname = get_bool(settings, "verify_hostname").unwrap_or(true);
    let verify_certificates = get_bool(settings, "verify_certificates").unwrap_or(true);

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

fn build_auth_config(settings: &HashMap<String, Setting>) -> Result<AuthConfig, ConfigError> {
    let authenticator = get_string(settings, "authenticator").unwrap_or_default();

    let use_jwt = authenticator == "SNOWFLAKE_JWT"
        || (authenticator.is_empty() && has_private_key_params(settings));

    if use_jwt {
        return Ok(AuthConfig::Jwt {
            user: get_string(settings, "user")
                .context(MissingParameterSnafu { parameter: "user" })?,
            private_key_pem: read_private_key(settings)?,
            passphrase: get_string(settings, "private_key_password"),
        });
    }

    match authenticator.as_str() {
        "SNOWFLAKE_PASSWORD" | "" => Ok(AuthConfig::Password {
            user: get_string(settings, "user")
                .context(MissingParameterSnafu { parameter: "user" })?,
            password: get_string(settings, "password").context(MissingParameterSnafu {
                parameter: "password",
            })?,
        }),
        "PROGRAMMATIC_ACCESS_TOKEN" => Ok(AuthConfig::Pat {
            user: get_string(settings, "user")
                .context(MissingParameterSnafu { parameter: "user" })?,
            token: get_string(settings, "token")
                .context(MissingParameterSnafu { parameter: "token" })?,
        }),
        _ => InvalidParameterValueSnafu {
            parameter: "authenticator",
            value: authenticator,
            explanation:
                "Allowed values are SNOWFLAKE_JWT, SNOWFLAKE_PASSWORD, and PROGRAMMATIC_ACCESS_TOKEN",
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
    /// The input should come from `ConfigResolver::resolve()`.
    /// Runs `validate_settings` first and returns all validation errors
    /// collected (not just the first) via `ConfigError::ValidationFailed`.
    /// Runtime errors that go beyond static validation (e.g. base64
    /// decoding failures, file I/O) are still returned individually.
    pub fn build(settings: &HashMap<String, Setting>) -> Result<Self, ConfigError> {
        let issues = validate_settings(settings);
        let errors: Vec<_> = issues
            .into_iter()
            .filter(|i| i.severity == ValidationSeverity::Error)
            .collect();
        if !errors.is_empty() {
            return ValidationFailedSnafu { issues: errors }.fail();
        }

        let account = get_string(settings, "account")
            .context(MissingParameterSnafu { parameter: "account" })?;
        let server_url = derive_server_url(settings)?;
        let auth = build_auth_config(settings)?;
        let tls = build_tls_config(settings);

        let session = SessionContext {
            database: get_string(settings, "database"),
            schema: get_string(settings, "schema"),
            warehouse: get_string(settings, "warehouse"),
            role: get_string(settings, "role"),
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

// ---------------------------------------------------------------------------
// validate_settings – pre-flight check that collects all issues
// ---------------------------------------------------------------------------

/// Validate settings without building the full config.
/// Returns a list of all issues found (errors and warnings).
pub fn validate_settings(settings: &HashMap<String, Setting>) -> Vec<ValidationIssue> {
    let mut issues = Vec::new();

    // --- MissingRequired: account ---
    if get_string(settings, "account").is_none() {
        issues.push(ValidationIssue {
            severity: ValidationSeverity::Error,
            parameter: "account".into(),
            message: "Missing required parameter 'account'".into(),
            code: ValidationCode::MissingRequired,
        });
    }

    // --- MissingRequired: user ---
    if get_string(settings, "user").is_none() {
        issues.push(ValidationIssue {
            severity: ValidationSeverity::Error,
            parameter: "user".into(),
            message: "Missing required parameter 'user'".into(),
            code: ValidationCode::MissingRequired,
        });
    }

    // --- Auth-specific checks based on authenticator ---
    let authenticator = get_string(settings, "authenticator").unwrap_or_default();
    match authenticator.as_str() {
        "SNOWFLAKE_PASSWORD" | "" if has_private_key_params(settings) => {
            // Auto-detect JWT: private key params present, no password needed
        }
        "SNOWFLAKE_PASSWORD" | "" => {
            if get_string(settings, "password").is_none() {
                issues.push(ValidationIssue {
                    severity: ValidationSeverity::Error,
                    parameter: "password".into(),
                    message: "Missing required parameter 'password' for password authentication"
                        .into(),
                    code: ValidationCode::MissingRequired,
                });
            }
        }
        "SNOWFLAKE_JWT" => {
            if !has_private_key_params(settings) {
                issues.push(ValidationIssue {
                    severity: ValidationSeverity::Error,
                    parameter: "private_key".into(),
                    message:
                        "Missing 'private_key' or 'private_key_file' for JWT authentication".into(),
                    code: ValidationCode::MissingRequired,
                });
            }
        }
        "PROGRAMMATIC_ACCESS_TOKEN" => {
            if get_string(settings, "token").is_none() {
                issues.push(ValidationIssue {
                    severity: ValidationSeverity::Error,
                    parameter: "token".into(),
                    message: "Missing required parameter 'token' for PAT authentication".into(),
                    code: ValidationCode::MissingRequired,
                });
            }
        }
        other => {
            issues.push(ValidationIssue {
                severity: ValidationSeverity::Error,
                parameter: "authenticator".into(),
                message: format!(
                    "Invalid authenticator '{other}'. Allowed: SNOWFLAKE_PASSWORD, SNOWFLAKE_JWT, PROGRAMMATIC_ACCESS_TOKEN"
                ),
                code: ValidationCode::InvalidValue,
            });
        }
    }

    // --- MissingRequired: host (when server_url is absent) ---
    if get_string(settings, "server_url").is_none() && get_string(settings, "host").is_none() {
        issues.push(ValidationIssue {
            severity: ValidationSeverity::Error,
            parameter: "host".into(),
            message: "Missing required parameter 'host' (or 'server_url')".into(),
            code: ValidationCode::MissingRequired,
        });
    }

    // --- InvalidValue: protocol ---
    if let Some(protocol) = get_string(settings, "protocol") {
        if protocol != "http" && protocol != "https" {
            issues.push(ValidationIssue {
                severity: ValidationSeverity::Error,
                parameter: "protocol".into(),
                message: format!(
                    "Invalid protocol '{protocol}'. Allowed values: 'http', 'https'"
                ),
                code: ValidationCode::InvalidValue,
            });
        }
    }

    // --- InvalidValue: crl_check_mode ---
    // TODO: make matching case-insensitive (e.g. "disabled", "Enabled")
    if let Some(mode) = get_string(settings, "crl_check_mode") {
        let valid = ["DISABLED", "ENABLED", "ADVISORY", "0", "1", "2"];
        if !valid.contains(&mode.as_str()) {
            issues.push(ValidationIssue {
                severity: ValidationSeverity::Error,
                parameter: "crl_check_mode".into(),
                message: format!(
                    "Invalid crl_check_mode '{mode}'. Allowed: DISABLED, ENABLED, ADVISORY, 0, 1, 2"
                ),
                code: ValidationCode::InvalidValue,
            });
        }
    }

    // --- ConflictingParameters: private_key + private_key_file ---
    let has_pk = settings.get("private_key").is_some();
    let has_pk_file = get_string(settings, "private_key_file").is_some();
    if has_pk && has_pk_file {
        issues.push(ValidationIssue {
            severity: ValidationSeverity::Error,
            parameter: "private_key".into(),
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

    fn settings_from(pairs: &[(&str, Setting)]) -> HashMap<String, Setting> {
        pairs
            .iter()
            .map(|(k, v)| (k.to_string(), v.clone()))
            .collect()
    }

    fn minimal_password_settings() -> HashMap<String, Setting> {
        settings_from(&[
            ("account", Setting::String("myaccount".into())),
            ("user", Setting::String("myuser".into())),
            ("password", Setting::String("mypassword".into())),
            ("host", Setting::String("myaccount.snowflakecomputing.com".into())),
        ])
    }

    // -- ConnectionConfig::build tests --

    #[test]
    fn build_minimal_password_auth_succeeds() {
        let settings = minimal_password_settings();
        let config = ConnectionConfig::build(&settings).unwrap();

        assert_eq!(config.server.account, "myaccount");
        assert!(config.server.server_url.contains("myaccount.snowflakecomputing.com"));
        match &config.auth {
            AuthConfig::Password { user, password } => {
                assert_eq!(user, "myuser");
                assert_eq!(password, "mypassword");
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
            ("authenticator", Setting::String("PROGRAMMATIC_ACCESS_TOKEN".into())),
            ("host", Setting::String("h.com".into())),
        ]);
        let config = ConnectionConfig::build(&settings).unwrap();
        match &config.auth {
            AuthConfig::Pat { user, token } => {
                assert_eq!(user, "u");
                assert_eq!(token, "tok123");
            }
            _ => panic!("Expected Pat auth"),
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
    fn validate_returns_all_issues_not_just_first() {
        let settings: HashMap<String, Setting> = HashMap::new();
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
