use crate::{auth::AuthError, config::ConfigError};

#[derive(Debug)]
pub enum RestError {
    AuthError(AuthError),
    MissingParameter(String),
    InvalidArgument(String),
    InvalidSnowflakeResponse(String),
    Internal(String),
    Status(reqwest::StatusCode),
    BadConfig(ConfigError),
}

impl std::fmt::Display for RestError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RestError::AuthError(e) => write!(f, "Auth error: {e:?}"),
            RestError::MissingParameter(s) => write!(f, "Missing parameter: {s}"),
            RestError::InvalidArgument(s) => write!(f, "Invalid argument: {s}"),
            RestError::InvalidSnowflakeResponse(s) => write!(f, "Invalid Snowflake response: {s}"),
            RestError::Internal(s) => write!(f, "Internal error: {s}"),
            RestError::Status(s) => write!(f, "Status: {s}"),
            RestError::BadConfig(e) => write!(f, "Bad config: {e:?}"),
        }
    }
}

impl std::error::Error for RestError {}

impl From<ConfigError> for RestError {
    fn from(error: ConfigError) -> Self {
        RestError::BadConfig(error)
    }
}
