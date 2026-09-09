//! Lets Snowflake's execution platform (XP) provide query and login transport
//! for a sandbox without outbound network access.
//!
//! [`SNOWFLAKE_RUNNING_INSIDE_XP`] is read once when a
//! [`crate::apis::database_driver_v1::DatabaseDriverV1`] is constructed. XP
//! mode fails when no backend is registered instead of falling back to HTTP.
//! The variable's presence enables XP mode, including when its value is empty.
//! After login, heartbeat, session refresh, logout, and result-chunk downloads
//! stay off HTTP. File-transfer payload methods default to unsupported.
//!
//! [`SNOWFLAKE_RUNNING_INSIDE_XP`]: crate::env_vars::SNOWFLAKE_RUNNING_INSIDE_XP

use std::collections::HashMap;

use async_trait::async_trait;

use crate::config::rest_parameters::{LoginParameters, QueryParameters};
use crate::rest::snowflake::{LoginResult, QueryExecutionMode, QueryInput, query_response};

pub(crate) mod registry;
pub(crate) use registry::XpSlot;

/// Driver-originated failures. All negative, so a caller can tell them from a
/// Snowflake server error code, which a host passes through as a positive value.
pub mod error_codes {
    pub const UNSUPPORTED: i32 = -1001;
    pub const NOT_REGISTERED: i32 = -1002;
    pub const ALREADY_REGISTERED: i32 = -1003;
    /// Null pointer, non-UTF-8 text, or a string with an interior NUL.
    pub const INVALID_ARGUMENT: i32 = -1004;
    pub const PANIC: i32 = -1005;
    /// The host returned a payload the driver could not parse.
    pub const PROTOCOL: i32 = -1006;
    pub const ABI_MISMATCH: i32 = -1007;
}

/// `code` is a Snowflake server error code from the host, or one of
/// [`error_codes`] when the driver itself failed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BackendError {
    pub code: i32,
    pub message: String,
}

impl BackendError {
    pub fn new(code: i32, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
        }
    }

    pub fn unsupported(operation: &str) -> Self {
        Self::new(
            error_codes::UNSUPPORTED,
            format!("operation `{operation}` is not supported by the active backend"),
        )
    }

    pub fn not_registered() -> Self {
        Self::new(
            error_codes::NOT_REGISTERED,
            format!(
                "{} is set but no backend has been registered; the host must register \
                 the XP backend before running a query",
                crate::env_vars::SNOWFLAKE_RUNNING_INSIDE_XP
            ),
        )
    }

    pub fn already_registered() -> Self {
        Self::new(
            error_codes::ALREADY_REGISTERED,
            "a backend is already registered and cannot be replaced",
        )
    }

    pub fn invalid_argument(message: impl Into<String>) -> Self {
        Self::new(error_codes::INVALID_ARGUMENT, message)
    }

    pub fn protocol(message: impl Into<String>) -> Self {
        Self::new(error_codes::PROTOCOL, message)
    }

    pub fn panicked(operation: &str) -> Self {
        Self::new(
            error_codes::PANIC,
            format!("panic caught while executing backend operation `{operation}`"),
        )
    }

    pub fn abi_mismatch(expected: u32, got: u32) -> Self {
        Self::new(
            error_codes::ABI_MISMATCH,
            format!("backend ABI version mismatch: driver speaks {expected}, host sent {got}"),
        )
    }
}

impl std::fmt::Display for BackendError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} (code: {})", self.message, self.code)
    }
}

impl std::error::Error for BackendError {}

pub type BackendResult<T> = Result<T, BackendError>;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BackendCancelTarget<'a> {
    QueryId(&'a str),
    RequestId {
        request_id: &'a str,
        sql_text: &'a str,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BackendCancelOutcome {
    Cancelled,
    /// Nothing was running. A normal outcome, not a failure.
    NotRunning,
}

#[derive(Debug, Clone, Copy)]
pub struct BackendQueryOptions {
    pub execution_mode: QueryExecutionMode,
    /// Stable across retries of one logical execution, so it can cancel a query
    /// whose server-side id is not known yet.
    pub request_id: uuid::Uuid,
}

/// A transport the driver can use in place of its own HTTP client. Shared across
/// threads and may be called concurrently.
///
/// The file-transfer methods default to [`BackendError::unsupported`] so a host
/// can be useful without them: SQL works, and `session.file.put` fails naming the
/// missing operation instead of attempting an upload the sandbox would drop.
#[async_trait]
pub trait SnowflakeBackend: Send + Sync {
    /// Reports the session the host is already inside, rather than performing a
    /// credential exchange.
    async fn authenticate(
        &self,
        params: &LoginParameters,
        session_parameters: Option<&HashMap<String, String>>,
    ) -> BackendResult<LoginResult>;

    async fn execute_query(
        &self,
        input: &QueryInput<'_>,
        params: &QueryParameters,
        options: BackendQueryOptions,
    ) -> BackendResult<query_response::Response>;

    /// The monitoring endpoint's body verbatim, so the driver can run the same
    /// extraction it runs on an HTTP response rather than a second one that could
    /// drift from it.
    async fn get_query_status(&self, query_id: &str) -> BackendResult<String>;

    async fn get_query_result(&self, query_id: &str) -> BackendResult<query_response::Response>;

    async fn cancel_query(
        &self,
        target: BackendCancelTarget<'_>,
    ) -> BackendResult<BackendCancelOutcome>;

    async fn get_session_parameters(&self) -> BackendResult<HashMap<String, String>>;

    async fn upload_stream(
        &self,
        _destination: &str,
        _data: &[u8],
        _metadata_json: &str,
    ) -> BackendResult<String> {
        Err(BackendError::unsupported("upload_stream"))
    }

    async fn download_stream(&self, _source: &str, _metadata_json: &str) -> BackendResult<Vec<u8>> {
        Err(BackendError::unsupported("download_stream"))
    }

    async fn list_stage_files(
        &self,
        _stage_location: &str,
        _is_exact_match: bool,
    ) -> BackendResult<String> {
        Err(BackendError::unsupported("list_stage_files"))
    }

    async fn download_query_chunk(
        &self,
        _url: &str,
        _headers: &HashMap<String, String>,
    ) -> BackendResult<Vec<u8>> {
        Err(BackendError::unsupported("download_query_chunk"))
    }
}

#[cfg(test)]
pub(crate) struct TestBackend;

#[cfg(test)]
#[async_trait]
impl SnowflakeBackend for TestBackend {
    async fn authenticate(
        &self,
        _params: &LoginParameters,
        _session_parameters: Option<&HashMap<String, String>>,
    ) -> BackendResult<LoginResult> {
        unimplemented!()
    }

    async fn execute_query(
        &self,
        _input: &QueryInput<'_>,
        _params: &QueryParameters,
        _options: BackendQueryOptions,
    ) -> BackendResult<query_response::Response> {
        unimplemented!()
    }

    async fn get_query_status(&self, _query_id: &str) -> BackendResult<String> {
        unimplemented!()
    }

    async fn get_query_result(&self, _query_id: &str) -> BackendResult<query_response::Response> {
        unimplemented!()
    }

    async fn cancel_query(
        &self,
        _target: BackendCancelTarget<'_>,
    ) -> BackendResult<BackendCancelOutcome> {
        unimplemented!()
    }

    async fn get_session_parameters(&self) -> BackendResult<HashMap<String, String>> {
        unimplemented!()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    #[tokio::test]
    async fn defaulted_file_transfer_methods_name_the_missing_operation() {
        let backend = TestBackend;

        for (err, operation) in [
            (
                backend.upload_stream("@stage/f", b"x", "{}").await,
                "upload_stream",
            ),
            (
                backend
                    .download_stream("@stage/f", "{}")
                    .await
                    .map(|_| String::new()),
                "download_stream",
            ),
            (
                backend.list_stage_files("@stage", true).await,
                "list_stage_files",
            ),
            (
                backend
                    .download_query_chunk("https://chunk.test/c", &HashMap::new())
                    .await
                    .map(|_| String::new()),
                "download_query_chunk",
            ),
        ] {
            let err = err.expect_err("defaulted method should not succeed");
            assert_eq!(err.code, error_codes::UNSUPPORTED);
            assert!(
                err.message.contains(operation),
                "message should name the operation, got: {}",
                err.message
            );
        }
    }

    #[test]
    fn driver_originated_codes_are_unique_and_outside_server_sentinels() {
        let codes = [
            error_codes::UNSUPPORTED,
            error_codes::NOT_REGISTERED,
            error_codes::ALREADY_REGISTERED,
            error_codes::INVALID_ARGUMENT,
            error_codes::PANIC,
            error_codes::PROTOCOL,
            error_codes::ABI_MISMATCH,
        ];
        let unique: HashSet<_> = codes.into_iter().collect();

        assert_eq!(unique.len(), codes.len());
        assert!(codes.into_iter().all(|code| code <= -1000));
        assert!(!unique.contains(&crate::rest::snowflake::GS_CODE_UNAVAILABLE));
    }

    #[test]
    fn not_registered_message_names_the_env_var_and_registration_requirement() {
        let err = BackendError::not_registered();
        assert!(
            err.message
                .contains(crate::env_vars::SNOWFLAKE_RUNNING_INSIDE_XP)
        );
        assert!(err.message.contains("register the XP backend"));
    }
}
