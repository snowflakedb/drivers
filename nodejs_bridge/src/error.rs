use napi::bindgen_prelude::*;
use sf_core::apis::database_driver_v1::ApiError;
use std::future::Future;
use std::sync::Arc;

pub trait ToJsError {
    fn to_js_error(&self, env: Env) -> napi::Error;
}

impl ToJsError for ApiError {
    fn to_js_error(&self, env: Env) -> napi::Error {
        let err_builder = || -> napi::Result<napi::Error> {
            let mut err = env.create_error(napi::Error::from_reason(self.to_string()))?;
            err.set_named_property("name", "[TODO]")?;
            Ok(napi::Error::from(err.to_unknown()))
        };
        err_builder().unwrap_or_else(|err| {
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
