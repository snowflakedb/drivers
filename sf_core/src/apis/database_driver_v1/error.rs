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
    #[snafu(display("Query cancel timed out after {timeout:?}"))]
    CancelTimeout {
        timeout: std::time::Duration,
        #[snafu(implicit)]
        location: Location,
    },
    /// The operation observed its [`OperationCtx`](crate::apis::operation_ctx::OperationCtx)
    /// token being cancelled and unwound cooperatively.
    ///
    /// Raised by the operation itself, not synthesised at the FFI boundary, so
    /// callers below the protobuf layer (Node, in-process Rust) see the same
    /// typed error the protobuf layer maps to `STATUS_CODE_CANCELLED`.
    #[snafu(display("Operation was cancelled"))]
    #[snafu(visibility(pub))]
    Cancelled {
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("Failed to write upload stream chunk to spool buffer: {source}"))]
    SpoolBufferWrite {
        #[snafu(implicit)]
        location: Location,
        source: std::io::Error,
    },
}

/// Wrapper-neutral structured metadata extracted from an [`ApiError`].
///
/// Fields remain `None` when the underlying error does not carry that
/// diagnostic. Wrappers can translate the values into their native error
/// types without matching sf_core's internal error variants.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ErrorDiagnostics {
    /// Snowflake's numeric server error code.
    pub vendor_code: Option<i32>,
    /// SQLSTATE supplied by the server or inferred from the vendor code when
    /// sf_core has a canonical mapping. Server values are returned verbatim.
    pub sql_state: Option<String>,
    /// Server-assigned Snowflake query ID.
    pub query_id: Option<String>,
    /// Client-generated request ID used to submit the query.
    pub request_id: Option<String>,
}

impl ApiError {
    /// Returns structured diagnostics suitable for any language or driver wrapper.
    ///
    /// The server-provided SQLSTATE is authoritative. When it is absent for a
    /// query error, sf_core falls back to its vendor-code mapping. Known login
    /// credential failures use the authorization SQLSTATE (`28000`). No
    /// diagnostic is inferred from human-readable message text.
    #[must_use]
    pub fn diagnostics(&self) -> ErrorDiagnostics {
        let (vendor_code, sql_state) = match self {
            Self::Query { source, .. } => match source.as_ref() {
                RestError::QueryFailed {
                    code, sql_state, ..
                } => (*code, sql_state.clone()),
                RestError::AsyncQuery {
                    source: crate::rest::snowflake::error::SfError::SnowflakeBody { code, .. },
                    ..
                } => (Some(*code), None),
                _ => (None, None),
            },
            Self::Login { source, .. } => match source.as_ref() {
                RestError::LoginError { code, .. }
                    if *code != crate::rest::snowflake::GS_CODE_UNAVAILABLE =>
                {
                    (
                        Some(*code),
                        crate::rest::snowflake::CREDENTIAL_REJECTION_GS_CODES
                            .contains(code)
                            .then(|| {
                                crate::rest::snowflake::SQLSTATE_AUTHORIZATION_FAILURE.to_owned()
                            }),
                    )
                }
                _ => (None, None),
            },
            _ => (None, None),
        };
        let sql_state = sql_state.or_else(|| {
            vendor_code
                .and_then(crate::rest::snowflake::sql_state::sql_state_from_code)
                .map(str::to_owned)
        });

        let (query_id, request_id) = match self {
            Self::Query { source, .. } => match source.as_ref() {
                RestError::QueryFailed {
                    query_id,
                    request_id,
                    ..
                } => (
                    query_id.clone(),
                    request_id.as_ref().map(ToString::to_string),
                ),
                RestError::AsyncQuery {
                    query_id,
                    request_id,
                    ..
                } => (
                    query_id.as_ref().map(ToString::to_string),
                    request_id.as_ref().map(ToString::to_string),
                ),
                _ => (None, None),
            },
            _ => (None, None),
        };

        ErrorDiagnostics {
            vendor_code,
            sql_state,
            query_id,
            request_id,
        }
    }
}
