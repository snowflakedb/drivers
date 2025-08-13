use std::fs;

use crate::config::ConfigError;
use crate::config::settings::Setting;
use crate::config::settings::Settings;

fn get_server_url(settings: &dyn Settings) -> Result<String, ConfigError> {
    if let Some(Setting::String(value)) = settings.get("server_url") {
        return Ok(value.clone());
    }

    let protocol = settings
        .get_string("protocol")
        .unwrap_or("https".to_string());
    let host = settings
        .get_string("host")
        .ok_or_else(|| ConfigError::MissingParameter("host".to_string()))?;
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
pub struct ClientInfo {
    pub application: String,
    pub version: String,
    pub os: String,
    pub os_version: String,
    pub ocsp_mode: Option<String>,
}

impl ClientInfo {
    pub fn from_settings(_settings: &dyn Settings) -> Result<Self, ConfigError> {
        let client_info = ClientInfo {
            application: "PythonConnector".to_string(),
            version: "3.15.0".to_string(),
            os: "Darwin".to_string(),
            os_version: "macOS-15.5-arm64-arm-64bit".to_string(),
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
                    return Err(ConfigError::MissingParameter("account".to_string()));
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
}

impl LoginMethod {
    fn read_private_key(settings: &dyn Settings) -> Result<String, ConfigError> {
        if let Some(private_key_file) = settings.get_string("private_key_file") {
            let private_key = fs::read_to_string(private_key_file).map_err(|e| {
                ConfigError::InvalidArgument(format!("Could not read private key file: {e}"))
            })?;
            Ok(private_key)
        } else {
            Err(ConfigError::MissingParameter(
                "private_key_file".to_string(),
            ))
        }
    }

    pub fn from_settings(settings: &dyn Settings) -> Result<Self, ConfigError> {
        match settings
            .get_string("authenticator")
            .unwrap_or_default()
            .as_str()
        {
            "SNOWFLAKE_JWT" => Ok(Self::PrivateKey {
                username: settings
                    .get_string("user")
                    .ok_or(ConfigError::MissingParameter("user".to_string()))?,
                private_key: Self::read_private_key(settings)?,
                passphrase: settings.get_string("private_key_password"),
            }),
            "SNOWFLAKE_PASSWORD" | "" => Ok(Self::Password {
                username: settings
                    .get_string("user")
                    .ok_or(ConfigError::MissingParameter("user".to_string()))?,
                password: settings
                    .get_string("password")
                    .ok_or(ConfigError::MissingParameter("password".to_string()))?,
            }),
            _ => Err(ConfigError::InvalidArgument(
                "Invalid authenticator".to_string(),
            )),
        }
    }
}
