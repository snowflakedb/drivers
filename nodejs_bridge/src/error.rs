use napi::bindgen_prelude::*;
use sf_core::apis::database_driver_v1::{ApiError, ErrorKind};
use std::future::Future;
use std::sync::Arc;

pub trait ToJsError {
    fn to_js_error(&self, env: Env) -> napi::Error;
}

#[derive(Clone)]
pub(crate) enum BridgeError {
    Core(Arc<ApiError>),
    UnusableConnection(UnusableConnection),
    Message(String),
}

#[derive(Clone, Copy)]
pub(crate) enum UnusableConnection {
    NeverEstablished,
    Terminated,
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
        Self {
            name: error_name(error.kind()),
            message: error.to_string(),
            code: error.vendor_code().map(ErrorCode::Server),
            sql_state: error.sql_state(),
            cause: error.root_cause(),
            is_fatal: false,
        }
    }

    fn of_unusable_connection(connection: UnusableConnection) -> Self {
        let (code, message, is_fatal) = match connection {
            UnusableConnection::NeverEstablished => (
                407001,
                "Unable to perform operation because a connection was never established.",
                false,
            ),
            UnusableConnection::Terminated => (
                407002,
                "Unable to perform operation using terminated connection.",
                true,
            ),
        };
        Self {
            name: Some("ClientError"),
            message: message.to_string(),
            code: Some(ErrorCode::Driver(code)),
            sql_state: Some("08003".to_string()),
            cause: None,
            is_fatal,
        }
    }

    fn build(&self, env: Env) -> napi::Result<napi::Error> {
        let mut error = env.create_error(napi::Error::from_reason(self.message.clone()))?;
        if let Some(name) = self.name {
            error.set_named_property("name", name)?;
        }
        match &self.code {
            Some(ErrorCode::Server(code)) => {
                error.set_named_property("code", format!("{code:06}"))?;
            }
            Some(ErrorCode::Driver(code)) => error.set_named_property("code", code)?,
            None => {}
        }
        if let Some(sql_state) = &self.sql_state {
            error.set_named_property("sqlState", sql_state.as_str())?;
        }
        if let Some(cause) = &self.cause {
            let cause = env.create_error(napi::Error::from_reason(cause.clone()))?;
            error.set_named_property("cause", cause)?;
        }
        if self.is_fatal {
            error.set_named_property("isFatal", true)?;
        }
        Ok(napi::Error::from(error.to_unknown()))
    }
}

fn error_name(kind: ErrorKind) -> Option<&'static str> {
    match kind {
        ErrorKind::QueryFailed => Some("OperationFailedError"),
        _ => None,
    }
}

impl ToJsError for ApiError {
    fn to_js_error(&self, env: Env) -> napi::Error {
        ClientError::of(self).build(env).unwrap_or_else(|err| {
            napi::Error::from_reason(format!("failed to construct JS error for {self}: {err}"))
        })
    }
}

impl ToJsError for BridgeError {
    fn to_js_error(&self, env: Env) -> napi::Error {
        match self {
            BridgeError::Core(error) => error.to_js_error(env),
            BridgeError::UnusableConnection(connection) => {
                ClientError::of_unusable_connection(*connection)
                    .build(env)
                    .unwrap_or_else(|err| {
                        napi::Error::from_reason(format!("failed to construct JS error: {err}"))
                    })
            }
            BridgeError::Message(message) => napi::Error::from_reason(message.clone()),
        }
    }
}

impl From<ApiError> for BridgeError {
    fn from(error: ApiError) -> Self {
        BridgeError::Core(Arc::new(error))
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
    fn a_connection_that_was_never_established_is_not_fatal() {
        let error = ClientError::of_unusable_connection(UnusableConnection::NeverEstablished);

        assert_eq!(error.name, Some("ClientError"));
        assert_eq!(code_of(&error), Some("407001".to_string()));
        assert_eq!(error.sql_state.as_deref(), Some("08003"));
        assert!(!error.is_fatal);
    }

    #[test]
    fn a_terminated_connection_is_fatal() {
        let error = ClientError::of_unusable_connection(UnusableConnection::Terminated);

        assert_eq!(error.name, Some("ClientError"));
        assert_eq!(code_of(&error), Some("407002".to_string()));
        assert_eq!(error.sql_state.as_deref(), Some("08003"));
        assert!(error.is_fatal);
    }
}
