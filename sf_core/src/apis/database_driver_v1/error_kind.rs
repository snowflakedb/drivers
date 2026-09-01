//! Wrapper-facing classification of [`ApiError`].

use super::error::{ApiError, CancellationAbortResult, QueryResponseProcessingError};
use crate::compression_types::CompressionTypeError;
use crate::config::ConfigErrorClass;
use crate::config::connection_config::ValidationCode;
use crate::file_manager::FileManagerError;
use crate::rest::snowflake::RestError;

/// Classification of an [`ApiError`] for wrapper exception mapping.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorKind {
    Unspecified,
    AuthenticationError,
    NotImplemented,
    InvalidArgument,
    Io,
    Cancelled,
    InternalError,
    MissingParameter,
    InvalidParameterValue,
    LoginError,
    LocalFileNotFound,
    RemoteFileNotFound,
    UnsupportedCompression,
    QueryFailed,
    Timeout,
    StageBinding,
}

impl ApiError {
    pub fn kind(&self) -> ErrorKind {
        kind_of(self)
    }

    pub fn vendor_code(&self) -> Option<i32> {
        self.snowflake_context().vendor_code
    }

    pub fn sql_state(&self) -> Option<String> {
        self.snowflake_context().sql_state
    }

    pub fn query_id(&self) -> Option<String> {
        self.snowflake_context().query_id
    }

    pub fn request_id(&self) -> Option<String> {
        self.snowflake_context().request_id
    }

    pub fn parameter(&self) -> Option<String> {
        self.parameter_context().parameter
    }

    pub fn parameter_value(&self) -> Option<String> {
        self.parameter_context().parameter_value
    }

    pub fn validation_code(&self) -> Option<ValidationCode> {
        self.parameter_context().validation_code
    }

    pub fn reauthentication_required(&self) -> bool {
        reauthentication_required(self)
    }

    pub fn root_cause(&self) -> Option<String> {
        extract_root_cause(self)
    }

    pub fn cancellation_abort_outcome(&self) -> Option<CancellationAbortResult> {
        match self {
            ApiError::Cancelled { abort, .. } => *abort,
            _ => None,
        }
    }
}

fn kind_from_config_class(value: ConfigErrorClass) -> ErrorKind {
    match value {
        ConfigErrorClass::MissingParameter => ErrorKind::MissingParameter,
        ConfigErrorClass::InvalidParameterValue => ErrorKind::InvalidParameterValue,
        ConfigErrorClass::InternalError => ErrorKind::InternalError,
    }
}

fn kind_from_query_response(value: &QueryResponseProcessingError) -> ErrorKind {
    match value {
        QueryResponseProcessingError::FileUpload { source, .. }
        | QueryResponseProcessingError::FileDownload { source, .. } => match source {
            FileManagerError::NoFilesMatched { .. } => ErrorKind::LocalFileNotFound,
            FileManagerError::CompressionType {
                source: CompressionTypeError::UnsupportedCompressionType { .. },
                ..
            } => ErrorKind::UnsupportedCompression,
            // A too-large source file / stage object is an input
            // error, not a driver fault — surface it as
            // `InvalidArgument` rather than `InternalError`.
            s if s.is_file_too_large() => ErrorKind::InvalidArgument,
            // Everything else here (Io, UploadBatch, cloud
            // transport errors, ...) is an environmental/transfer
            // failure, not an internal driver bug — `Io` maps to
            // `OperationalError` on the Python side, matching the
            // reference connector's own classification for the
            // same class of failure.
            _ => ErrorKind::Io,
        },
        QueryResponseProcessingError::RemoteFileNotFound { .. } => ErrorKind::RemoteFileNotFound,
        // no wildcard - explicit empty arms
        QueryResponseProcessingError::UploadResultsConversion { .. }
        | QueryResponseProcessingError::DownloadResultsConversion { .. }
        | QueryResponseProcessingError::BatchRead { .. }
        | QueryResponseProcessingError::UnsupportedCommand { .. }
        | QueryResponseProcessingError::FileTransferPreparation { .. } => ErrorKind::InternalError,
    }
}

/// Classify a REST failure that escaped through `ApiError::Query`.
fn kind_from_query_rest_error(err: &RestError) -> ErrorKind {
    match err {
        RestError::QueryFailed { .. } => ErrorKind::QueryFailed,
        RestError::OperationTimeout { .. } => ErrorKind::Timeout,
        RestError::Communication { .. } | RestError::HttpRetry { .. } => ErrorKind::Io,
        RestError::Authentication { .. }
        | RestError::NativeOkta { .. }
        | RestError::ExternalBrowser { .. }
        | RestError::OAuthFlow { .. }
        | RestError::WorkloadIdentityAttestation { .. }
        | RestError::LoginError { .. }
        | RestError::SessionRefresh { .. }
        | RestError::SessionRefreshFailed { .. }
        | RestError::SessionExpired { .. }
        | RestError::MasterTokenTerminal { .. }
        | RestError::TokenRequestHttp { .. }
        | RestError::TokenRequestFailed { .. } => ErrorKind::AuthenticationError,
        RestError::InvalidSnowflakeResponse { .. }
        | RestError::RequestConstruction { .. }
        | RestError::CrlValidation { .. }
        | RestError::UrlJoin { .. }
        | RestError::Heartbeat { .. }
        | RestError::MissingResponseField { .. }
        | RestError::Logout { .. }
        | RestError::InvalidUrl { .. }
        | RestError::PayloadEncode { .. }
        | RestError::AsyncPollResultNotFound { .. }
        | RestError::MissingResultUrl { .. }
        | RestError::MissingQueryId { .. } => ErrorKind::InternalError,
    }
}

fn kind_of(error: &ApiError) -> ErrorKind {
    match error {
        ApiError::Configuration { .. } => kind_from_config_class(error.parameter_context().class),
        ApiError::QueryResponseProcess { source, .. } => kind_from_query_response(source),

        ApiError::InvalidColumnMetadata { .. } => ErrorKind::InvalidArgument,
        ApiError::InvalidWifProvider { .. } => ErrorKind::InvalidParameterValue,
        // Use InvalidParameterValue so Python callers see ProgrammingError,
        // matching the legacy connector's exception class for this function.
        ApiError::WorkloadIdentityAttestation { .. } => ErrorKind::InvalidParameterValue,

        ApiError::Login { source, .. } => match source.as_ref() {
            RestError::LoginError {
                reauthentication_required: true,
                ..
            } => ErrorKind::AuthenticationError,
            RestError::LoginError { .. } => ErrorKind::LoginError,
            RestError::OperationTimeout { .. } => ErrorKind::Timeout,
            _ => ErrorKind::AuthenticationError,
        },
        ApiError::TlsClientCreation { .. }
        | ApiError::SessionRefresh { .. }
        | ApiError::MasterTokenTerminal { .. }
        | ApiError::TokenCacheInitialization { .. }
        | ApiError::TokenRequest { .. } => ErrorKind::AuthenticationError,

        ApiError::InvalidArgument { .. } | ApiError::ConnectionClosed { .. } => {
            ErrorKind::InvalidArgument
        }

        ApiError::HttpRequest { .. } | ApiError::FileTransfersDisabled { .. } => ErrorKind::Io,
        ApiError::StageBinding { .. } => ErrorKind::StageBinding,
        ApiError::QueryTimeout { .. } | ApiError::CancelTimeout { .. } => ErrorKind::Timeout,
        ApiError::Cancelled { .. } => ErrorKind::Cancelled,

        ApiError::Query { source, .. } => kind_from_query_rest_error(source),
        ApiError::Statement { .. }
        | ApiError::ConnectionLock { .. }
        | ApiError::StatementLocking { .. }
        | ApiError::DatabaseLocking { .. }
        | ApiError::ConnectionNotInitialized { .. }
        | ApiError::InvalidRefreshState { .. }
        | ApiError::Logout { .. }
        | ApiError::RuntimeCreation { .. }
        | ApiError::ChunkFetch { .. }
        | ApiError::ArrowParse { .. }
        | ApiError::JsonChunkDecode { .. }
        | ApiError::BlockingTaskJoin { .. }
        | ApiError::InlineJsonEncode { .. }
        | ApiError::Base64Decode { .. }
        | ApiError::UnsupportedQueryResultFormat { .. }
        // Local temp-file / in-memory spool I/O failure while buffering a
        // chunked upload-stream chunk — a driver-side fault, not caller input.
        | ApiError::SpoolBufferWrite { .. } => ErrorKind::InternalError,
    }
}

fn reauthentication_required(error: &ApiError) -> bool {
    match error {
        ApiError::MasterTokenTerminal { .. } => true,
        ApiError::Login { source, .. } => matches!(
            source.as_ref(),
            RestError::LoginError {
                reauthentication_required: true,
                ..
            }
        ),
        _ => false,
    }
}

/// Walk the `source()` chain to the deepest error and return its message.
/// Returns `None` when the error has no source (i.e. the message itself is
/// already the root cause).
fn extract_root_cause(error: &dyn std::error::Error) -> Option<String> {
    let mut deepest: Option<&dyn std::error::Error> = None;
    let mut current = error.source();
    while let Some(cause) = current {
        deepest = Some(cause);
        current = cause.source();
    }
    deepest.map(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rest::snowflake::{QueryIds, RestError};
    use snafu::location;

    fn loc() -> snafu::Location {
        location!()
    }

    fn query(source: RestError) -> ApiError {
        ApiError::Query {
            location: loc(),
            source: Box::new(source),
        }
    }

    #[test]
    fn invalid_argument_is_the_public_constructor() {
        let err = ApiError::invalid_argument("multi-statement results are not supported yet");
        assert_eq!(err.kind(), ErrorKind::InvalidArgument);
        assert!(err.to_string().contains("multi-statement"));
        let ApiError::InvalidArgument { location, .. } = &err else {
            panic!("expected InvalidArgument");
        };
        assert!(
            location.file.contains("error_kind.rs"),
            "track_caller should record this test, got {}",
            location.file
        );
    }

    #[test]
    fn cancelled_projects_kind() {
        let err = ApiError::Cancelled {
            abort: None,
            location: loc(),
        };
        assert_eq!(err.kind(), ErrorKind::Cancelled);
        assert!(!err.reauthentication_required());
    }

    #[test]
    fn query_failed_projects_query_failed_kind() {
        let err = query(RestError::QueryFailed {
            message: "boom".to_owned(),
            code: Some(1003),
            sql_state: Some("42000".to_owned()),
            ids: QueryIds::default(),
            location: loc(),
            query_context: None,
        });
        assert_eq!(err.kind(), ErrorKind::QueryFailed);
    }

    #[test]
    fn query_operation_timeout_projects_timeout_kind() {
        let err = query(RestError::OperationTimeout {
            operation: "query".to_owned(),
            budget: std::time::Duration::from_secs(1),
            ids: QueryIds::default(),
            location: loc(),
        });
        assert_eq!(err.kind(), ErrorKind::Timeout);
    }

    #[test]
    fn query_http_retry_projects_io_kind() {
        let err = query(RestError::HttpRetry {
            context: "query",
            ids: QueryIds::default(),
            source: crate::http::retry::HttpError::MaxAttempts {
                attempts: 3,
                last_status: reqwest::StatusCode::SERVICE_UNAVAILABLE,
                location: loc(),
            },
            location: loc(),
        });
        assert_eq!(err.kind(), ErrorKind::Io);
    }

    #[test]
    fn query_session_expired_projects_authentication_kind() {
        let err = query(RestError::SessionExpired { location: loc() });
        assert_eq!(err.kind(), ErrorKind::AuthenticationError);
    }

    #[test]
    fn login_without_reauth_projects_login_error_kind() {
        let err = ApiError::Login {
            location: loc(),
            source: Box::new(RestError::LoginError {
                message: "bad password".to_owned(),
                code: 390100,
                reauthentication_required: false,
                location: loc(),
            }),
        };
        assert_eq!(err.kind(), ErrorKind::LoginError);
        assert!(!err.reauthentication_required());
    }

    #[test]
    fn login_with_reauth_projects_authentication_kind() {
        let err = ApiError::Login {
            location: loc(),
            source: Box::new(RestError::LoginError {
                message: "reauth".to_owned(),
                code: 390195,
                reauthentication_required: true,
                location: loc(),
            }),
        };
        assert_eq!(err.kind(), ErrorKind::AuthenticationError);
        assert!(err.reauthentication_required());
    }
}
