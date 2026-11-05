use crate::rows::RowDecodeError;
use arrow::error::ArrowError;
use napi::bindgen_prelude::*;
use sf_core::apis::database_driver_v1::{ApiError, ErrorKind};
use sf_core::chunks::ChunkError;
use sf_core::http::retry::HttpError;
use sf_core::rest::snowflake::RestError;
use std::future::Future;
use std::sync::Arc;

pub trait ToJsError {
    fn to_js_error(&self, env: Env) -> napi::Error;
}

#[derive(Clone, Debug)]
pub(crate) enum BridgeError {
    Core(Arc<ApiError>),
    UnusableConnection(ConnectionOperation, UnusableConnection),
    InvalidQueryId(String),
    InvalidRequestId(String),
    QueryStatusFailed {
        query_id: String,
        error_code: Option<i32>,
        error_message: Option<String>,
    },
    AlreadyConnected,
    AlreadyConnecting,
    ConnectionTerminated,
    QueryIdNoData(String),
    QueryIdNotSuccess {
        query_id: String,
        status: String,
    },
    RowDecode(Arc<RowDecodeError>),
    Message(String),
}

#[derive(Clone, Copy, Debug)]
pub(crate) enum UnusableConnection {
    NeverEstablished,
    Terminated,
}

#[derive(Clone, Copy, Debug)]
pub(crate) enum ConnectionOperation {
    Request,
    Destroy,
}

enum ErrorCode {
    /// Snowflake error codes are six digits wide; the core parses them to integers.
    Server(i32),
    Driver(i32),
}

struct ClientError {
    name: Option<&'static str>,
    message: String,
    code: Option<ErrorCode>,
    sql_state: Option<String>,
    cause: Option<String>,
    is_fatal: bool,
}

impl ClientError {
    fn of(error: &ApiError) -> Self {
        if is_snowflake_unreachable(error) {
            return Self::of_network_unreachable(error.root_cause());
        }
        match chunk_download_failure(error) {
            Some(ChunkDownloadFailure::Unreachable) => {
                return Self::of_cloud_storage_unreachable(error.root_cause());
            }
            Some(ChunkDownloadFailure::ErrorStatus(status)) => {
                return Self::of_cloud_storage_error_status(status);
            }
            None => {}
        }
        if matches!(error.kind(), ErrorKind::Cancelled) {
            return Self::of_cancelled();
        }
        Self {
            name: error_name(error),
            message: error.to_string(),
            code: error.vendor_code().map(ErrorCode::Server),
            sql_state: error.sql_state(),
            cause: error.root_cause(),
            is_fatal: error.terminates_session(),
        }
    }

    fn of_bridge(error: &BridgeError) -> Self {
        match error {
            BridgeError::Core(error) => Self::of(error),
            BridgeError::UnusableConnection(operation, connection) => {
                Self::of_unusable_connection(*operation, *connection)
            }
            BridgeError::InvalidQueryId(query_id) => Self::of_invalid_id("queryId", query_id),
            BridgeError::InvalidRequestId(request_id) => {
                Self::of_invalid_id("requestId", request_id)
            }
            BridgeError::QueryStatusFailed {
                query_id,
                error_code,
                error_message,
            } => Self::of_query_status_error(query_id, *error_code, error_message),
            BridgeError::AlreadyConnected => Self::of_already_connected(),
            BridgeError::AlreadyConnecting => Self::of_already_connecting(),
            BridgeError::ConnectionTerminated => Self::of_connection_terminated(),
            BridgeError::QueryIdNoData(query_id) => Self::of_query_id_no_data(query_id),
            BridgeError::QueryIdNotSuccess { query_id, status } => {
                Self::of_query_id_not_success(query_id, status)
            }
            BridgeError::RowDecode(error) => Self::unnamed(error.to_string()),
            BridgeError::Message(message) => Self::unnamed(message.clone()),
        }
    }

    /// The old driver's SQL-state table covers only the 405xxx and 407xxx families.
    fn of_unusable_connection(
        operation: ConnectionOperation,
        connection: UnusableConnection,
    ) -> Self {
        let (code, message, sql_state, is_fatal) = match (operation, connection) {
            (ConnectionOperation::Request, UnusableConnection::NeverEstablished) => (
                407001,
                "Unable to perform operation because a connection was never established.",
                Some("08003"),
                false,
            ),
            (ConnectionOperation::Request, UnusableConnection::Terminated) => (
                407002,
                "Unable to perform operation using terminated connection.",
                Some("08003"),
                true,
            ),
            (ConnectionOperation::Destroy, UnusableConnection::NeverEstablished) => {
                (406501, "Not connected, so nothing to destroy.", None, false)
            }
            (ConnectionOperation::Destroy, UnusableConnection::Terminated) => {
                (406502, "Already disconnected.", None, false)
            }
        };
        Self {
            name: Some("ClientError"),
            message: message.to_string(),
            code: Some(ErrorCode::Driver(code)),
            sql_state: sql_state.map(str::to_string),
            cause: None,
            is_fatal,
        }
    }

    fn of_query_status_error(
        query_id: &str,
        error_code: Option<i32>,
        error_message: &Option<String>,
    ) -> Self {
        Self {
            name: Some("OperationFailedError"),
            message: error_message
                .clone()
                .unwrap_or_else(|| format!("Query {query_id} failed")),
            code: error_code.map(ErrorCode::Server),
            sql_state: None,
            cause: None,
            is_fatal: false,
        }
    }

    fn of_invalid_id(field: &str, value: &str) -> Self {
        Self {
            name: Some("InvalidParameterError"),
            message: format!("Invalid {field}: {value}"),
            // TODO: move error codes enum to bridge
            code: Some(ErrorCode::Driver(460001)),
            sql_state: None,
            cause: None,
            is_fatal: false,
        }
    }

    fn of_already_connected() -> Self {
        Self {
            name: Some("ClientError"),
            message: "Already connected.".to_string(),
            code: Some(ErrorCode::Driver(405502)),
            sql_state: Some("08002".to_string()),
            cause: None,
            is_fatal: false,
        }
    }

    fn of_already_connecting() -> Self {
        Self {
            name: Some("ClientError"),
            message: "Connection already in progress.".to_string(),
            code: Some(ErrorCode::Driver(405501)),
            sql_state: Some("08002".to_string()),
            cause: None,
            is_fatal: false,
        }
    }

    fn of_connection_terminated() -> Self {
        Self {
            name: Some("ClientError"),
            message: "Connection already terminated. Cannot connect again.".to_string(),
            code: Some(ErrorCode::Driver(405503)),
            sql_state: Some("08003".to_string()),
            cause: None,
            is_fatal: false,
        }
    }

    fn of_network_unreachable(cause: Option<String>) -> Self {
        Self {
            name: Some("NetworkError"),
            message: "Network error. Could not reach Snowflake.".to_string(),
            code: Some(ErrorCode::Driver(401001)),
            sql_state: None,
            cause,
            is_fatal: false,
        }
    }

    fn of_cloud_storage_unreachable(cause: Option<String>) -> Self {
        Self {
            name: Some("NetworkError"),
            message: "Network error. Could not reach S3/Blob.".to_string(),
            code: Some(ErrorCode::Driver(402001)),
            sql_state: None,
            cause,
            is_fatal: false,
        }
    }

    fn of_cloud_storage_error_status(status: String) -> Self {
        Self {
            name: Some("LargeResultSetError"),
            message: "Request to S3/Blob failed.".to_string(),
            code: Some(ErrorCode::Driver(402002)),
            sql_state: None,
            cause: Some(format!("HTTP status {status}")),
            is_fatal: false,
        }
    }

    fn of_cancelled() -> Self {
        Self {
            name: Some("OperationFailedError"),
            message: "SQL execution canceled".to_string(),
            code: Some(ErrorCode::Server(604)),
            sql_state: Some("57014".to_string()),
            cause: None,
            is_fatal: false,
        }
    }

    fn of_query_id_no_data(query_id: &str) -> Self {
        Self {
            name: Some("ClientError"),
            message: format!(
                "Cannot retrieve data. No information returned from server for query {query_id}"
            ),
            code: Some(ErrorCode::Driver(460002)),
            sql_state: None,
            cause: None,
            is_fatal: true,
        }
    }

    fn of_query_id_not_success(query_id: &str, status: &str) -> Self {
        Self {
            name: Some("ClientError"),
            message: format!("Status of query {query_id} is {status}, results are unavailable"),
            code: Some(ErrorCode::Driver(460003)),
            sql_state: None,
            cause: None,
            is_fatal: true,
        }
    }

    fn unnamed(message: String) -> Self {
        Self {
            name: None,
            message,
            code: None,
            sql_state: None,
            cause: None,
            is_fatal: false,
        }
    }

    fn build(&self, env: Env) -> napi::Result<napi::Error> {
        let mut error = new_js_error(&env, self.message.clone())?;
        if let Some(name) = self.name {
            error.set_named_property("name", name)?;
        }
        if let Some(code) = &self.code {
            match code {
                ErrorCode::Server(code) => {
                    error.set_named_property("code", format!("{code:06}"))?;
                }
                ErrorCode::Driver(code) => error.set_named_property("code", code)?,
            }
        }
        if let Some(sql_state) = &self.sql_state {
            error.set_named_property("sqlState", sql_state.as_str())?;
        }
        if let Some(cause) = &self.cause {
            let cause = new_js_error(&env, cause.clone())?;
            error.set_named_property("cause", cause)?;
        }
        if self.is_fatal {
            error.set_named_property("isFatal", true)?;
        }
        Ok(napi::Error::from(error.to_unknown()))
    }
}

// `napi_create_error` takes the napi status as the JS error's `code`, and `from_reason` fixes that
// status at `GenericFailure`, so an error the driver has no code for would arrive carrying one.
// Clearing it here leaves `code` present and undefined, as the old driver had it; a code the driver
// does know is assigned by the caller.
fn new_js_error<'env>(env: &'env Env, message: String) -> napi::Result<Object<'env>> {
    let mut error = env.create_error(napi::Error::from_reason(message))?;
    error.set_named_property("code", Undefined::default())?;
    Ok(error)
}

fn construct_js_error_fail(err: napi::Error) -> napi::Error {
    napi::Error::from_reason(format!("failed to construct JS error: {err}"))
}

enum ChunkDownloadFailure {
    Unreachable,
    ErrorStatus(String),
}

fn chunk_download_failure(error: &ApiError) -> Option<ChunkDownloadFailure> {
    let ApiError::ChunkFetch { source, .. } = error else {
        return None;
    };
    let download_error = match source {
        ChunkError::ChunkRead {
            source: ArrowError::ExternalError(inner),
            ..
        } => inner.downcast_ref::<ChunkError>()?,
        direct => direct,
    };
    match download_error {
        ChunkError::Communication { .. } | ChunkError::ChunkReadTimeout { .. } => {
            Some(ChunkDownloadFailure::Unreachable)
        }
        ChunkError::UnsuccessfulHttpStatusCode { status, .. } => {
            Some(ChunkDownloadFailure::ErrorStatus(status.to_string()))
        }
        _ => None,
    }
}

fn error_name(error: &ApiError) -> Option<&'static str> {
    name_for_kind(error.kind(), error.vendor_code().is_some())
}

fn name_for_kind(kind: ErrorKind, has_server_code: bool) -> Option<&'static str> {
    match kind {
        ErrorKind::QueryFailed => Some("OperationFailedError"),
        ErrorKind::LoginError | ErrorKind::AuthenticationError if has_server_code => {
            Some("OperationFailedError")
        }
        ErrorKind::MissingParameter | ErrorKind::InvalidParameterValue => {
            Some("InvalidParameterError")
        }
        _ => None,
    }
}

// ErrorKind::Io also covers file and stage I/O. A dedicated core network kind would let
// wrappers classify transport failures without inspecting ApiError variants here.
fn is_snowflake_unreachable(error: &ApiError) -> bool {
    match error {
        ApiError::HttpRequest { .. } => true,
        ApiError::Login { source, .. }
            if matches!(source.as_ref(), RestError::OperationTimeout { .. }) =>
        {
            true
        }
        ApiError::Login { source, .. }
        | ApiError::Query { source, .. }
        | ApiError::SessionRefresh { source, .. }
        | ApiError::TokenRequest { source, .. } => matches!(
            source.as_ref(),
            RestError::Communication { .. }
                | RestError::HttpRetry {
                    source: HttpError::Transport { .. },
                    ..
                }
        ),
        _ => false,
    }
}

impl ToJsError for ApiError {
    fn to_js_error(&self, env: Env) -> napi::Error {
        ClientError::of(self)
            .build(env)
            .unwrap_or_else(construct_js_error_fail)
    }
}

impl ToJsError for BridgeError {
    fn to_js_error(&self, env: Env) -> napi::Error {
        ClientError::of_bridge(self)
            .build(env)
            .unwrap_or_else(construct_js_error_fail)
    }
}

impl From<ApiError> for BridgeError {
    fn from(error: ApiError) -> Self {
        BridgeError::Core(Arc::new(error))
    }
}

impl From<RowDecodeError> for BridgeError {
    fn from(error: RowDecodeError) -> Self {
        BridgeError::RowDecode(Arc::new(error))
    }
}

impl<T: ToJsError + ?Sized> ToJsError for Arc<T> {
    fn to_js_error(&self, env: Env) -> napi::Error {
        (**self).to_js_error(env)
    }
}

/// Runs an async operation that can fail and resolves it into a JS async block
/// (a Promise), building any error as a JS `Error` on the main thread.
///
/// napi runs the future on a worker thread, then calls a resolver on the main
/// JS thread. Only that resolver gets an `Env`, which we need to build a proper
/// JS `Error` (see [`ToJsError`]). So the future can't fail directly. Instead it
/// always succeeds with an inner `Result<Value, Failure>`, and we turn the
/// error into a JS `Error` in the main-thread step.
pub fn async_to_js<Value, Failure, Fut>(env: &Env, future: Fut) -> napi::Result<AsyncBlock<Value>>
where
    Value: ToNapiValue + Send + 'static,
    Failure: ToJsError + Send + 'static,
    Fut: Future<Output = std::result::Result<Value, Failure>> + Send + 'static,
{
    AsyncBlockBuilder::build_with_map(env, async move { Ok(future.await) }, |env, outcome| {
        outcome.map_err(|e| e.to_js_error(env))
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn code_of(error: &ClientError) -> Option<String> {
        match error.code {
            Some(ErrorCode::Server(code)) => Some(format!("{code:06}")),
            Some(ErrorCode::Driver(code)) => Some(code.to_string()),
            None => None,
        }
    }

    #[test]
    fn argument_errors_carry_no_server_fields() {
        let error = ClientError::of(&ApiError::invalid_argument("row mode"));

        assert_eq!(error.name, None);
        assert_eq!(error.message, "Invalid argument: row mode");
        assert_eq!(code_of(&error), None);
        assert_eq!(error.sql_state, None);
        assert_eq!(error.cause, None);
        assert!(!error.is_fatal);
    }

    #[test]
    fn a_bridge_message_carries_no_name_or_code() {
        let error = ClientError::of_bridge(&BridgeError::Message(
            "nested multi-statement result is not supported".to_string(),
        ));

        assert_eq!(error.name, None);
        assert_eq!(
            error.message,
            "nested multi-statement result is not supported"
        );
        assert_eq!(code_of(&error), None);
        assert_eq!(error.sql_state, None);
        assert_eq!(error.cause, None);
        assert!(!error.is_fatal);
    }

    #[test]
    fn a_login_rejected_by_snowflake_is_an_operation_failed_error() {
        assert_eq!(
            name_for_kind(ErrorKind::LoginError, true),
            Some("OperationFailedError")
        );
        assert_eq!(
            name_for_kind(ErrorKind::AuthenticationError, true),
            Some("OperationFailedError")
        );
    }

    #[test]
    fn an_authentication_failure_inside_the_driver_has_no_name() {
        assert_eq!(name_for_kind(ErrorKind::LoginError, false), None);
        assert_eq!(name_for_kind(ErrorKind::AuthenticationError, false), None);
    }

    #[test]
    fn missing_and_invalid_settings_are_invalid_parameter_errors() {
        assert_eq!(
            name_for_kind(ErrorKind::MissingParameter, false),
            Some("InvalidParameterError")
        );
        assert_eq!(
            name_for_kind(ErrorKind::InvalidParameterValue, false),
            Some("InvalidParameterError")
        );
    }

    #[test]
    fn a_request_on_a_connection_that_was_never_established_is_not_fatal() {
        let error = ClientError::of_unusable_connection(
            ConnectionOperation::Request,
            UnusableConnection::NeverEstablished,
        );

        assert_eq!(error.name, Some("ClientError"));
        assert_eq!(code_of(&error), Some("407001".to_string()));
        assert_eq!(error.sql_state.as_deref(), Some("08003"));
        assert!(!error.is_fatal);
    }

    #[test]
    fn a_request_on_a_terminated_connection_is_fatal() {
        let error = ClientError::of_unusable_connection(
            ConnectionOperation::Request,
            UnusableConnection::Terminated,
        );

        assert_eq!(error.name, Some("ClientError"));
        assert_eq!(code_of(&error), Some("407002".to_string()));
        assert_eq!(error.sql_state.as_deref(), Some("08003"));
        assert!(error.is_fatal);
    }

    #[test]
    fn destroying_a_connection_that_was_never_established_carries_no_sql_state() {
        let error = ClientError::of_unusable_connection(
            ConnectionOperation::Destroy,
            UnusableConnection::NeverEstablished,
        );

        assert_eq!(error.name, Some("ClientError"));
        assert_eq!(code_of(&error), Some("406501".to_string()));
        assert_eq!(error.message, "Not connected, so nothing to destroy.");
        assert_eq!(error.sql_state, None);
        assert!(!error.is_fatal);
    }

    #[test]
    fn destroying_a_terminated_connection_is_not_fatal() {
        let error = ClientError::of_unusable_connection(
            ConnectionOperation::Destroy,
            UnusableConnection::Terminated,
        );

        assert_eq!(error.name, Some("ClientError"));
        assert_eq!(code_of(&error), Some("406502".to_string()));
        assert_eq!(error.message, "Already disconnected.");
        assert_eq!(error.sql_state, None);
        assert!(!error.is_fatal);
    }

    #[test]
    fn an_invalid_query_id_is_an_invalid_parameter_error() {
        let error = ClientError::of_invalid_id("queryId", "invalidQueryId");

        assert_eq!(error.name, Some("InvalidParameterError"));
        assert_eq!(code_of(&error), Some("460001".to_string()));
        assert_eq!(error.message, "Invalid queryId: invalidQueryId");
        assert_eq!(error.sql_state, None);
        assert!(!error.is_fatal);
    }

    #[test]
    fn an_invalid_request_id_is_an_invalid_parameter_error() {
        let error = ClientError::of_invalid_id("requestId", "foobar");

        assert_eq!(error.name, Some("InvalidParameterError"));
        assert_eq!(code_of(&error), Some("460001".to_string()));
        assert_eq!(error.message, "Invalid requestId: foobar");
        assert_eq!(error.sql_state, None);
        assert!(!error.is_fatal);
    }

    #[test]
    fn a_failed_query_status_is_an_operation_failed_error() {
        let error = ClientError::of_query_status_error(
            "01c715df-0e1a-450a-000c-a913e79f70cb",
            Some(2003),
            &Some("SQL compilation error".to_string()),
        );

        assert_eq!(error.name, Some("OperationFailedError"));
        assert_eq!(code_of(&error), Some("002003".to_string()));
        assert_eq!(error.message, "SQL compilation error");
        assert_eq!(error.sql_state, None);
        assert!(!error.is_fatal);
    }

    #[test]
    fn a_failed_query_status_without_monitoring_fields_uses_the_query_id() {
        let error = ClientError::of_query_status_error("abc", None, &None);

        assert_eq!(error.name, Some("OperationFailedError"));
        assert_eq!(code_of(&error), None);
        assert_eq!(error.message, "Query abc failed");
    }

    #[test]
    fn a_transport_failure_reaching_snowflake_is_a_network_error() {
        let error = ClientError::of_network_unreachable(Some("connection refused".to_string()));

        assert_eq!(error.name, Some("NetworkError"));
        assert_eq!(code_of(&error), Some("401001".to_string()));
        assert_eq!(error.message, "Network error. Could not reach Snowflake.");
        assert_eq!(error.sql_state, None);
        assert_eq!(error.cause.as_deref(), Some("connection refused"));
        assert!(!error.is_fatal);
    }

    #[test]
    fn connecting_an_already_connected_connection_is_not_fatal() {
        let error = ClientError::of_already_connected();

        assert_eq!(error.name, Some("ClientError"));
        assert_eq!(code_of(&error), Some("405502".to_string()));
        assert_eq!(error.message, "Already connected.");
        assert_eq!(error.sql_state.as_deref(), Some("08002"));
        assert!(!error.is_fatal);
    }

    #[test]
    fn connecting_while_a_login_is_in_progress_is_not_fatal() {
        let error = ClientError::of_already_connecting();

        assert_eq!(error.name, Some("ClientError"));
        assert_eq!(code_of(&error), Some("405501".to_string()));
        assert_eq!(error.message, "Connection already in progress.");
        assert_eq!(error.sql_state.as_deref(), Some("08002"));
        assert!(!error.is_fatal);
    }

    #[test]
    fn connecting_a_terminated_connection_is_not_fatal() {
        let error = ClientError::of_connection_terminated();

        assert_eq!(error.name, Some("ClientError"));
        assert_eq!(code_of(&error), Some("405503".to_string()));
        assert_eq!(
            error.message,
            "Connection already terminated. Cannot connect again."
        );
        assert_eq!(error.sql_state.as_deref(), Some("08003"));
        assert!(!error.is_fatal);
    }

    #[test]
    fn no_data_for_a_query_id_is_a_fatal_client_error() {
        let error = ClientError::of_query_id_no_data("qid");

        assert_eq!(error.name, Some("ClientError"));
        assert_eq!(code_of(&error), Some("460002".to_string()));
        assert_eq!(
            error.message,
            "Cannot retrieve data. No information returned from server for query qid"
        );
        assert_eq!(error.sql_state, None);
        assert!(error.is_fatal);
    }

    #[test]
    fn a_non_success_terminal_status_is_a_fatal_client_error() {
        let error = ClientError::of_query_id_not_success("qid", "RESTARTED");

        assert_eq!(error.name, Some("ClientError"));
        assert_eq!(code_of(&error), Some("460003".to_string()));
        assert_eq!(
            error.message,
            "Status of query qid is RESTARTED, results are unavailable"
        );
        assert_eq!(error.sql_state, None);
        assert!(error.is_fatal);
    }
}
