use crate::{auth::AuthError, config::ConfigError};
use snafu::{Location, Snafu};

#[derive(Debug, Snafu)]
#[snafu(visibility(pub(crate)))]
pub enum RestError {
    #[snafu(display("Authentication failed"))]
    Auth {
        source: AuthError,
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("Missing required parameter: {parameter}"))]
    MissingParameter {
        parameter: String,
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("Invalid argument: {argument}"))]
    InvalidArgument {
        argument: String,
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("Invalid Snowflake response: {message}"))]
    InvalidSnowflakeResponse {
        message: String,
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("Internal error: {message}"))]
    Internal {
        message: String,
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("HTTP status error: {status}"))]
    Status {
        status: reqwest::StatusCode,
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("Configuration error"))]
    BadConfig {
        source: ConfigError,
        #[snafu(implicit)]
        location: Location,
    },
}
