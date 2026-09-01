use napi::bindgen_prelude::*;
use sf_core::apis::database_driver_v1::{ApiError, ErrorKind};
use std::future::Future;
use std::sync::Arc;

pub trait ToJsError {
    fn to_js_error(&self, env: Env) -> napi::Error;
}

struct ClientError {
    name: Option<&'static str>,
    message: String,
    code: Option<String>,
    sql_state: Option<String>,
    cause: Option<String>,
}

impl ClientError {
    fn of(error: &ApiError) -> Self {
        Self {
            name: error_name(error.kind()),
            message: error.to_string(),
            code: error.vendor_code().map(server_code),
            sql_state: error.sql_state(),
            cause: error.root_cause(),
        }
    }

    fn build(&self, env: Env) -> napi::Result<napi::Error> {
        let mut error = env.create_error(napi::Error::from_reason(self.message.clone()))?;
        if let Some(name) = self.name {
            error.set_named_property("name", name)?;
        }
        if let Some(code) = &self.code {
            error.set_named_property("code", code.as_str())?;
        }
        if let Some(sql_state) = &self.sql_state {
            error.set_named_property("sqlState", sql_state.as_str())?;
        }
        if let Some(cause) = &self.cause {
            let cause = env.create_error(napi::Error::from_reason(cause.clone()))?;
            error.set_named_property("cause", cause)?;
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

/// Snowflake error codes are six digits wide; the core parses them to integers.
fn server_code(vendor_code: i32) -> String {
    format!("{vendor_code:06}")
}

impl ToJsError for ApiError {
    fn to_js_error(&self, env: Env) -> napi::Error {
        ClientError::of(self).build(env).unwrap_or_else(|err| {
            napi::Error::from_reason(format!("failed to construct JS error for {self}: {err}"))
        })
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

    #[test]
    fn server_codes_keep_their_six_digit_form() {
        assert_eq!(server_code(605), "000605");
        assert_eq!(server_code(390100), "390100");
    }

    #[test]
    fn argument_errors_carry_no_server_fields() {
        let error = ClientError::of(&ApiError::invalid_argument("row mode"));

        assert_eq!(error.name, None);
        assert_eq!(error.message, "Invalid argument: row mode");
        assert_eq!(error.code, None);
        assert_eq!(error.sql_state, None);
        assert_eq!(error.cause, None);
    }
}
