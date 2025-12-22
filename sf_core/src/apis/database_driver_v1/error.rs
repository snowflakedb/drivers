use snafu::{Location, Snafu};

pub use crate::apis::database_driver_v1::query::QueryResponseProcessingError;
pub use crate::apis::database_driver_v1::statement::StatementError;
pub use crate::config::ConfigError;
pub use crate::rest::snowflake::RestError;
use crate::tls::error::TlsError;

#[derive(Debug, Snafu)]
#[snafu(visibility(pub(super)))]
pub enum ApiError {
    #[snafu(display("Generic error"))]
    GenericError {
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("Failed to create runtime"))]
    RuntimeCreation {
        #[snafu(implicit)]
        location: Location,
        source: std::io::Error,
    },
    #[snafu(display("Configuration error: {source}"))]
    Configuration {
        #[snafu(implicit)]
        location: Location,
        source: ConfigError,
    },
    #[snafu(display("Invalid argument: {argument}"))]
    InvalidArgument {
        argument: String,
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("Failed to login: {source}"))]
    Login {
        #[snafu(implicit)]
        location: Location,
        source: RestError,
    },
    #[snafu(display("Failed to lock connection"))]
    ConnectionLocking {
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("Connection not initialized"))]
    ConnectionNotInitialized {
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("TLS client creation failed: {source}"))]
    TlsClientCreation {
        source: TlsError,
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("Failed to lock statement"))]
    StatementLocking {
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("Failed to lock database"))]
    DatabaseLocking {
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("Failed to process query response: {source}"))]
    QueryResponseProcessing {
        #[snafu(implicit)]
        location: Location,
        #[snafu(source(from(QueryResponseProcessingError, Box::new)))]
        source: Box<QueryResponseProcessingError>,
    },
    #[snafu(display("Failed to refresh session: {source}"))]
    SessionRefresh {
        #[snafu(implicit)]
        location: Location,
        source: RestError,
    },
    #[snafu(display("Statement error: {source}"))]
    Statement {
        #[snafu(implicit)]
        location: Location,
        source: StatementError,
    },
    #[snafu(display("Query execution failed: {source}"))]
    Query {
        #[snafu(implicit)]
        location: Location,
        source: RestError,
    },
    #[snafu(display("Master token expired, full re-authentication required"))]
    MasterTokenExpired {
        #[snafu(implicit)]
        location: Location,
    },
}
