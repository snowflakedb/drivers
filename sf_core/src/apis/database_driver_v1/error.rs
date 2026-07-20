use error_trace::ErrorTrace;
use snafu::{Location, Snafu};
use std::time::Duration;

pub use crate::apis::database_driver_v1::query::QueryResponseProcessingError;
pub use crate::apis::database_driver_v1::statement::StatementError;
use crate::chunks::ChunkError;
pub use crate::config::ConfigError;
pub use crate::rest::snowflake::RestError;
use crate::tls::error::TlsError;
use crate::token_cache::TokenCacheError;

#[derive(Debug, Snafu, ErrorTrace)]
#[snafu(visibility(pub(crate)))]
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
    #[snafu(display("Failed to login"))]
    Login {
        #[snafu(implicit)]
        location: Location,
        #[snafu(source(from(RestError, Box::new)))]
        source: Box<RestError>,
    },
    #[snafu(display("Failed to lock connection"))]
    ConnectionLock {
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("Connection not initialized"))]
    ConnectionNotInitialized {
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("Connection is closed"))]
    ConnectionClosed {
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("TLS client creation failed: {source}"))]
    TlsClientCreation {
        #[snafu(source(from(TlsError, Box::new)))]
        source: Box<TlsError>,
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
    QueryResponseProcess {
        #[snafu(implicit)]
        location: Location,
        #[snafu(source(from(QueryResponseProcessingError, Box::new)))]
        source: Box<QueryResponseProcessingError>,
    },
    #[snafu(display("Failed to refresh session: {source}"))]
    SessionRefresh {
        #[snafu(implicit)]
        location: Location,
        #[snafu(source(from(RestError, Box::new)))]
        source: Box<RestError>,
    },
    #[snafu(display("Statement error: {source}"))]
    Statement {
        #[snafu(implicit)]
        location: Location,
        source: StatementError,
    },
    #[snafu(display("{source}"))]
    Query {
        #[snafu(implicit)]
        location: Location,
        #[snafu(source(from(RestError, Box::new)))]
        source: Box<RestError>,
    },
    #[snafu(display("HTTP request failed: {context}: {source}"))]
    HttpRequest {
        context: String,
        #[snafu(implicit)]
        location: Location,
        source: reqwest::Error,
    },
    #[snafu(display("Token request failed: {source}"))]
    TokenRequest {
        #[snafu(implicit)]
        location: Location,
        #[snafu(source(from(RestError, Box::new)))]
        source: Box<RestError>,
    },
    #[snafu(display("Master token expired, full re-authentication required"))]
    MasterTokenExpired {
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("Logout failed: {message}"))]
    Logout {
        message: String,
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("Invalid refresh state: {message}"))]
    InvalidRefreshState {
        message: String,
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display(
        "MFA token caching was requested but the token cache failed to initialize: {source}"
    ))]
    TokenCacheInitialization {
        source: TokenCacheError,
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("Failed to fetch chunk data"))]
    ChunkFetch {
        source: ChunkError,
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("Failed to parse Arrow IPC data"))]
    ArrowParse {
        source: arrow::error::ArrowError,
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("Failed to decode JSON chunk data"))]
    JsonChunkDecode {
        source: arrow::error::ArrowError,
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("Background chunk-decode task failed to join"))]
    BlockingTaskJoin {
        source: tokio::task::JoinError,
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("Failed to encode inline JSON rowset as Arrow IPC"))]
    InlineJsonEncode {
        #[snafu(implicit)]
        location: Location,
        source: ChunkError,
    },
    #[snafu(display("Invalid column metadata for '{column}'"))]
    InvalidColumnMetadata {
        column: String,
        #[snafu(implicit)]
        location: Location,
        source: crate::rest::snowflake::query_response::QueryResponseError,
    },
    #[snafu(display("Failed to decode base64 chunk data"))]
    Base64Decode {
        source: base64::DecodeError,
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("Unsupported queryResultFormat reported by the server: '{format}'"))]
    UnsupportedQueryResultFormat {
        format: String,
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("Stage binding failed: {source}"))]
    StageBinding {
        #[snafu(source(from(crate::stage_binding::StageBindingError, Box::new)))]
        source: Box<crate::stage_binding::StageBindingError>,
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("Query timed out after {budget:?}"))]
    QueryTimeout {
        budget: Duration,
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("Operation cancelled"))]
    Cancelled {
        #[snafu(implicit)]
        location: Location,
    },
}

impl ApiError {
    pub(crate) fn is_cancelled(&self) -> bool {
        match self {
            ApiError::Cancelled { .. } => true,
            ApiError::Login { source, .. }
            | ApiError::Query { source, .. }
            | ApiError::TokenRequest { source, .. }
            | ApiError::SessionRefresh { source, .. } => source.is_cancelled(),
            ApiError::ChunkFetch { source, .. } | ApiError::InlineJsonEncode { source, .. } => {
                source.is_cancelled()
            }
            ApiError::QueryResponseProcess { source, .. } => source.is_cancelled(),
            ApiError::StageBinding { source, .. } => source.is_cancelled(),
            _ => false,
        }
    }
}
