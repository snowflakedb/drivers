use std::fs;

use crate::config::InvalidParameterValueSnafu;
use crate::config::settings::Setting;
use crate::config::settings::Settings;
use crate::config::{ConfigError, MissingParameterSnafu};
use snafu::OptionExt;

fn get_server_url(settings: &dyn Settings) -> Result<String, ConfigError> {
    if let Some(Setting::String(value)) = settings.get("server_url") {
        return Ok(value.clone());
    }

    let protocol = settings
        .get_string("protocol")
        .unwrap_or("https".to_string());

    // Prefer explicit host; otherwise derive from account
    let host = match settings.get_string("host") {
        Some(h) => h,
        None => {
            let account = settings
                .get_string("account")
                .context(MissingParameterSnafu {
                    parameter: "account",
                })?;
            format!("{account}.snowflakecomputing.com")
        }
    };

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
}

impl ClientInfo {
    pub fn from_settings(settings: &dyn Settings) -> Result<Self, ConfigError> {
        // Check for client_app_id override, default to "ODBC" to match our driver type
        // This is critical for Snowflake feature detection (e.g., multi-statement support requires ODBC 2.21.8+)
        let application = settings
            .get_string("client_app_id")
            .unwrap_or_else(|| "ODBC".to_string());

        let client_info = ClientInfo {
            application,
            version: "3.13.0".to_string(), // Match the version we report in SQLGetInfo
            os: std::env::consts::OS.to_string(),
            os_version: format!("{}-{}", std::env::consts::OS, std::env::consts::ARCH),
            ocsp_mode: Some("FAIL_OPEN".to_string()),
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
}

impl LoginParameters {
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
        })
    }
}

pub enum LoginMethod {
    Password {
        username: String,
        password: String,
    },
    PrivateKey {
        username: String,
        private_key: String,
        passphrase: Option<String>,
    },
    Pat {
        username: String,
        token: String,
    },
    OAuth {
        username: String,
        token: String,
    },
    ExternalBrowser {
        username: String,
        token: String,
    },
}

impl LoginMethod {
    fn read_private_key(settings: &dyn Settings) -> Result<String, ConfigError> {
        if let Some(private_key_file) = settings.get_string("private_key_file") {
            let private_key = fs::read_to_string(private_key_file.clone()).map_err(|e| {
                InvalidParameterValueSnafu {
                    parameter: "private_key_file",
                    value: private_key_file,
                    explanation: format!("Could not read private key file: {e}"),
                }
                .build()
            })?;
            Ok(private_key)
        } else {
            MissingParameterSnafu {
                parameter: "private_key_file",
            }
            .fail()?
        }
    }

    pub fn from_settings(settings: &dyn Settings) -> Result<Self, ConfigError> {
        let authenticator = settings.get_string("authenticator").unwrap_or_default();
        match authenticator.as_str() {
            "SNOWFLAKE_JWT" => Ok(Self::PrivateKey {
                username: settings
                    .get_string("user")
                    .context(MissingParameterSnafu { parameter: "user" })?,
                private_key: Self::read_private_key(settings)?,
                passphrase: settings.get_string("private_key_password"),
            }),
            "SNOWFLAKE_PASSWORD" | "" => Ok(Self::Password {
                username: settings
                    .get_string("user")
                    .context(MissingParameterSnafu { parameter: "user" })?,
                password: settings
                    .get_string("password")
                    .context(MissingParameterSnafu {
                        parameter: "password",
                    })?,
            }),
            "PROGRAMMATIC_ACCESS_TOKEN" => Ok(Self::Pat {
                username: settings
                    .get_string("user")
                    .context(MissingParameterSnafu { parameter: "user" })?,
                token: settings
                    .get_string("token")
                    .context(MissingParameterSnafu { parameter: "token" })?,
            }),
            "OAUTH" => Ok(Self::OAuth {
                username: settings
                    .get_string("user")
                    .context(MissingParameterSnafu { parameter: "user" })?,
                token: settings
                    .get_string("token")
                    .context(MissingParameterSnafu { parameter: "token" })?,
            }),
            "EXTERNALBROWSER" => Ok(Self::ExternalBrowser {
                username: settings
                    .get_string("user")
                    .context(MissingParameterSnafu { parameter: "user" })?,
                token: settings
                    .get_string("token")
                    .context(MissingParameterSnafu { parameter: "token" })?,
            }),
            _ => InvalidParameterValueSnafu {
                parameter: "authenticator",
                value: authenticator,
                explanation: "Allowed values: SNOWFLAKE_JWT, SNOWFLAKE_PASSWORD, PROGRAMMATIC_ACCESS_TOKEN, OAUTH, EXTERNALBROWSER",
            }
            .fail()?,
        }
    }
}
