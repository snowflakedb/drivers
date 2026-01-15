//! Unified error types for the database driver API.

use crate::config::ConfigError;
use crate::rest::RestClientError;
use snafu::{Location, Snafu};

/// API errors that can occur during database driver operations.
#[derive(Debug, Snafu)]
#[snafu(visibility(pub))]
pub enum ApiError {
    #[snafu(display("Generic error: {message}"))]
    GenericError {
        message: String,
        #[snafu(implicit)]
        location: Location,
    },

    #[snafu(display("Runtime creation failed"))]
    RuntimeCreation {
        source: std::io::Error,
        #[snafu(implicit)]
        location: Location,
    },

    #[snafu(display("Configuration error"))]
    Configuration {
        source: ConfigError,
        #[snafu(implicit)]
        location: Location,
    },

    #[snafu(display("Invalid argument: {argument}"))]
    InvalidArgument {
        argument: String,
        #[snafu(implicit)]
        location: Location,
    },

    #[snafu(display("Login failed"))]
    Login {
        source: RestClientError,
        #[snafu(implicit)]
        location: Location,
    },

    #[snafu(display("Query failed"))]
    Query {
        source: RestClientError,
        #[snafu(implicit)]
        location: Location,
    },

    #[snafu(display("Connection locking failed"))]
    ConnectionLocking {
        #[snafu(implicit)]
        location: Location,
    },

    #[snafu(display("Statement locking failed"))]
    StatementLocking {
        #[snafu(implicit)]
        location: Location,
    },

    #[snafu(display("Database locking failed"))]
    DatabaseLocking {
        #[snafu(implicit)]
        location: Location,
    },

    #[snafu(display("Query response processing failed: {message}"))]
    QueryResponseProcessing {
        message: String,
        #[snafu(implicit)]
        location: Location,
    },

    #[snafu(display("Connection not initialized"))]
    ConnectionNotInitialized {
        #[snafu(implicit)]
        location: Location,
    },
}
