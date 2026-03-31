use std::collections::HashMap;
use std::fs;

use url::Url;

use crate::config::InvalidParameterValueSnafu;
use crate::config::settings::Setting;
use crate::config::settings::Settings;
use crate::config::{ConfigError, ConflictingParametersSnafu, MissingParameterSnafu};
use crate::crl::config::CrlConfig;
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

#[derive(Clone)]
pub struct QueryParameters {
    pub server_url: String,
    pub client_info: ClientInfo,
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
        })
    }
}
#[derive(Clone)]
pub struct ClientInfo {
    pub application: String,
    pub version: String,
    pub os: String,
    pub os_version: String,
    pub ocsp_mode: Option<String>,
    pub crl_config: CrlConfig,
    pub tls_config: TlsConfig,
}

impl ClientInfo {
    pub fn from_settings(settings: &dyn Settings) -> Result<Self, ConfigError> {
        let crl_config = CrlConfig::from_settings(settings)?;
        let tls_config = TlsConfig::from_settings(settings)?;

        // TODO: ClientInfo should be dynamically created based on the real hardware and
        // the wrapper client type
        let client_info = ClientInfo {
            application: settings
                .get_string("client_app_id")
                .unwrap_or_else(|| "PythonConnector".to_string()),
            version: "3.15.0".to_string(),
            os: "Darwin".to_string(),
            os_version: "macOS-15.5-arm64-arm-64bit".to_string(),
            ocsp_mode: Some("FAIL_OPEN".to_string()),
            crl_config,
            tls_config,
        };
        Ok(client_info)
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

    fn non_empty_string(settings: &dyn Settings, key: &str) -> Option<String> {
        settings.get_string(key).filter(|s| !s.is_empty())
    }

    pub fn from_settings(settings: &dyn Settings) -> Result<Self, ConfigError> {
        let authenticator = settings.get_string("authenticator").unwrap_or_default();

        // Auto-detect JWT authentication if private key params are present
        // and authenticator is not explicitly set to something else
        let use_jwt = authenticator == "SNOWFLAKE_JWT"
            || (authenticator.is_empty() && Self::has_private_key_params(settings));

        if use_jwt {
            return Ok(Self::PrivateKey {
                username: Self::non_empty_string(settings, "user")
                    .context(MissingParameterSnafu { parameter: "user" })?,
                private_key: Self::read_private_key(settings)?.into(),
                passphrase: settings
                    .get_string("private_key_password")
                    .map(SensitiveString::from),
            });
        }

        match authenticator.as_str() {
            "SNOWFLAKE_PASSWORD" | "" => Ok(Self::Password {
                username: Self::non_empty_string(settings, "user")
                    .context(MissingParameterSnafu { parameter: "user" })?,
                password: Self::non_empty_string(settings, "password")
                    .context(MissingParameterSnafu { parameter: "password" })?
                    .into(),
            }),
            "PROGRAMMATIC_ACCESS_TOKEN" => Ok(Self::Pat {
                username: Self::non_empty_string(settings, "user")
                    .context(MissingParameterSnafu { parameter: "user" })?,
                token: Self::non_empty_string(settings, "token")
                    .context(MissingParameterSnafu { parameter: "token" })?
                    .into(),
            }),
            _ if authenticator.to_ascii_lowercase().starts_with("https://") => {
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

                let username = Self::non_empty_string(settings, "user")
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
            "USERNAME_PASSWORD_MFA" => Ok(Self::UserPasswordMfa {
                username: Self::non_empty_string(settings, "user")
                    .context(MissingParameterSnafu { parameter: "user" })?,
                password: Self::non_empty_string(settings, "password")
                    .context(MissingParameterSnafu { parameter: "password" })?
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
            _ => InvalidParameterValueSnafu {
                parameter: "authenticator",
                value: authenticator,
                explanation: "Allowed values are SNOWFLAKE_JWT, SNOWFLAKE_PASSWORD, PROGRAMMATIC_ACCESS_TOKEN, USERNAME_PASSWORD_MFA, or an https:// URL for native Okta SSO",
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
}
