use error_trace::ErrorTrace;
use snafu::{Location, Snafu};
use std::time::Duration;

pub use crate::apis::database_driver_v1::query::QueryResponseProcessingError;
pub use crate::apis::database_driver_v1::statement::StatementError;
use crate::chunks::ChunkError;
pub use crate::config::ConfigError;
use crate::config::ConfigErrorContext;
pub use crate::rest::snowflake::RestError;
use crate::rest::snowflake::master_token_terminal_detail;
pub use crate::rest::snowflake::workload_identity::AttestationError;
use crate::rest::snowflake::{
    SQLSTATE_CONNECTION_WAS_NOT_ESTABLISHED, SQLSTATE_TIMEOUT_EXPIRED, SnowflakeErrorContext,
};
use crate::tls::error::TlsError;
use crate::token_cache::TokenCacheError;

/// What the abort-request a cancelled operation fires on its own behalf achieved.
///
/// "No abort was issued" is `Option::None`, not a variant here.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CancellationAbortResult {
    /// The server acknowledged the abort. The request was *processed* — not a
    /// guarantee the query had stopped by the time this was recorded.
    Aborted,
    /// The server reported the query was not running, so there was nothing to
    /// abort.
    NotRunning,
    /// The abort was issued but its result is unknown: it failed, or it had not
    /// finished when the cancelled caller was released (see `CLEANUP_WAIT`).
    NotConfirmed,
}

#[derive(Debug, Snafu, ErrorTrace)]
#[snafu(visibility(pub(crate)))]
pub enum ApiError {
    #[snafu(display("Generic error"))]
    GenericError {
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("File transfers have been disabled."))]
    FileTransfersDisabled {
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
        #[snafu(source(from(ConfigError, Box::new)))]
        source: Box<ConfigError>,
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
    #[snafu(display("{}", master_token_terminal_detail(*master_token_gs_code)))]
    MasterTokenTerminal {
        /// The GS code the server sent (390113/390114/390115), or `None` when
        /// expiry was predicted from a locally-tracked deadline with no
        /// server round-trip.
        master_token_gs_code: Option<i32>,
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
        request_id: String,
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("Query cancel timed out after {timeout:?}"))]
    CancelTimeout {
        timeout: Duration,
        request_id: String,
        #[snafu(implicit)]
        location: Location,
    },
    /// The operation observed its [`OperationCtx`](crate::apis::operation_ctx::OperationCtx)
    /// token being cancelled and unwound cooperatively.
    ///
    /// Raised by the operation itself, not synthesised at the FFI boundary, so
    /// callers below the protobuf layer (Node, in-process Rust) see the same
    /// typed error the protobuf layer maps to `ERROR_KIND_CANCELLED`.
    #[snafu(display("Operation was cancelled"))]
    #[snafu(visibility(pub))]
    Cancelled {
        /// What the abort-request fired on cancellation achieved. `None` when no
        /// abort was issued — the operation submitted no query, or was cancelled
        /// before its query reached the server.
        ///
        /// Attached by the operation that owns the abort (`statement::AbortReport`),
        /// not by whoever raises the error.
        abort: Option<CancellationAbortResult>,
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("Failed to write upload stream chunk to spool buffer: {source}"))]
    SpoolBufferWrite {
        #[snafu(implicit)]
        location: Location,
        source: std::io::Error,
    },
    #[snafu(display(
        "Invalid workload_identity_provider: '{provider}'. Allowed values: {}",
        crate::config::rest_parameters::WifProvider::allowed_values()
    ))]
    InvalidWifProvider {
        provider: String,
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("Workload Identity Federation attestation failed: {source}"))]
    WorkloadIdentityAttestation {
        #[snafu(implicit)]
        location: Location,
        #[snafu(source(from(AttestationError, Box::new)))]
        source: Box<AttestationError>,
    },
}

impl ApiError {
    pub(crate) fn snowflake_context(&self) -> SnowflakeErrorContext {
        match self {
            ApiError::Query { source, .. }
            | ApiError::Login { source, .. }
            | ApiError::SessionRefresh { source, .. }
            | ApiError::TokenRequest { source, .. } => source.snowflake_context(),
            ApiError::QueryTimeout { request_id, .. }
            | ApiError::CancelTimeout { request_id, .. } => SnowflakeErrorContext {
                vendor_code: None,
                sql_state: Some(SQLSTATE_TIMEOUT_EXPIRED.to_string()),
                query_id: None,
                request_id: Some(request_id.clone()),
            },
            ApiError::MasterTokenTerminal {
                master_token_gs_code,
                ..
            } => SnowflakeErrorContext {
                vendor_code: *master_token_gs_code,
                sql_state: Some(SQLSTATE_CONNECTION_WAS_NOT_ESTABLISHED.to_string()),
                query_id: None,
                request_id: None,
            },
            // no wildcard - explicit empty arms
            ApiError::GenericError { .. }
            | ApiError::FileTransfersDisabled { .. }
            | ApiError::RuntimeCreation { .. }
            | ApiError::Configuration { .. }
            | ApiError::InvalidArgument { .. }
            | ApiError::ConnectionLock { .. }
            | ApiError::ConnectionNotInitialized { .. }
            | ApiError::ConnectionClosed { .. }
            | ApiError::TlsClientCreation { .. }
            | ApiError::StatementLocking { .. }
            | ApiError::DatabaseLocking { .. }
            | ApiError::QueryResponseProcess { .. }
            | ApiError::Statement { .. }
            | ApiError::HttpRequest { .. }
            | ApiError::Logout { .. }
            | ApiError::InvalidRefreshState { .. }
            | ApiError::TokenCacheInitialization { .. }
            | ApiError::ChunkFetch { .. }
            | ApiError::ArrowParse { .. }
            | ApiError::JsonChunkDecode { .. }
            | ApiError::BlockingTaskJoin { .. }
            | ApiError::InlineJsonEncode { .. }
            | ApiError::InvalidColumnMetadata { .. }
            | ApiError::Base64Decode { .. }
            | ApiError::UnsupportedQueryResultFormat { .. }
            | ApiError::StageBinding { .. }
            | ApiError::Cancelled { .. }
            | ApiError::SpoolBufferWrite { .. }
            | ApiError::InvalidWifProvider { .. }
            | ApiError::WorkloadIdentityAttestation { .. } => SnowflakeErrorContext::default(),
        }
    }

    pub(crate) fn parameter_context(&self) -> ConfigErrorContext {
        match self {
            ApiError::Configuration { source, .. } => source.exception_context(),
            ApiError::InvalidColumnMetadata { column, .. } => ConfigErrorContext {
                parameter: Some(format!("column: {column}")),
                ..ConfigErrorContext::default()
            },
            ApiError::InvalidWifProvider { provider, .. } => ConfigErrorContext {
                parameter: Some("provider".to_string()),
                parameter_value: Some(provider.clone()),
                ..ConfigErrorContext::default()
            },
            // no wildcard - explicit empty arms
            ApiError::GenericError { .. }
            | ApiError::FileTransfersDisabled { .. }
            | ApiError::RuntimeCreation { .. }
            | ApiError::InvalidArgument { .. }
            | ApiError::Login { .. }
            | ApiError::ConnectionLock { .. }
            | ApiError::ConnectionNotInitialized { .. }
            | ApiError::ConnectionClosed { .. }
            | ApiError::TlsClientCreation { .. }
            | ApiError::StatementLocking { .. }
            | ApiError::DatabaseLocking { .. }
            | ApiError::QueryResponseProcess { .. }
            | ApiError::SessionRefresh { .. }
            | ApiError::Statement { .. }
            | ApiError::Query { .. }
            | ApiError::HttpRequest { .. }
            | ApiError::TokenRequest { .. }
            | ApiError::MasterTokenTerminal { .. }
            | ApiError::Logout { .. }
            | ApiError::InvalidRefreshState { .. }
            | ApiError::TokenCacheInitialization { .. }
            | ApiError::ChunkFetch { .. }
            | ApiError::ArrowParse { .. }
            | ApiError::JsonChunkDecode { .. }
            | ApiError::BlockingTaskJoin { .. }
            | ApiError::InlineJsonEncode { .. }
            | ApiError::Base64Decode { .. }
            | ApiError::UnsupportedQueryResultFormat { .. }
            | ApiError::StageBinding { .. }
            | ApiError::QueryTimeout { .. }
            | ApiError::CancelTimeout { .. }
            | ApiError::Cancelled { .. }
            | ApiError::SpoolBufferWrite { .. }
            | ApiError::WorkloadIdentityAttestation { .. } => ConfigErrorContext::default(),
        }
    }
}
