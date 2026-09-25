use crate::DRIVER;
use crate::error::{BridgeError, ToJsError, async_to_js};
use crate::query::{QueryStatus, require_valid_query_id};
use crate::session::Session;
use crate::session_params::KnownSessionParameters;
use crate::statement::Statement;
use napi::bindgen_prelude::*;
use napi::threadsafe_function::ThreadsafeFunctionCallMode;
use napi_derive::napi;
use sf_core::apis::database_driver_v1::connection::WrapperIdentity;
use sf_core::apis::database_driver_v1::{ApiError, BindingType, DataPtr};
use sf_core::apis::operation_ctx::OperationCtx;
use sf_core::config::param_names;
use sf_core::config::rest_parameters::BrowserOpenFn;
use sf_core::config::settings::Setting;
use sf_core::rest::snowflake::QueryStatusResult;
use std::collections::HashMap;
use std::sync::Arc;

const NODE_TLS_REJECT_UNAUTHORIZED: &str = "NODE_TLS_REJECT_UNAUTHORIZED";
const NODE_EXTRA_CA_CERTS: &str = "NODE_EXTRA_CA_CERTS";

#[napi]
pub struct Connection {
    session: Session,
}

#[napi]
pub enum QueryBindingFormat {
    Json,
    Csv,
}

/// Bind parameters handed down from the wrapper. `data` is either a JSON object
/// of `{ "1": { type, value }, ... }` or CSV text, selected by `format`.
#[napi(object)]
pub struct QueryBindings {
    pub format: QueryBindingFormat,
    pub data: String,
}

#[napi]
impl Connection {
    #[napi(constructor)]
    pub fn new(
        options: HashMap<String, String>,
        env: &Env,
        session_parameters: HashMap<String, String>,
        open_external_browser_callback: Option<Function<String, ()>>,
    ) -> Result<Self> {
        let database_handle = DRIVER.database_new();
        DRIVER.database_init(database_handle).map_err(|e| {
            let _ = DRIVER.database_release(database_handle);
            e.to_js_error(*env)
        })?;

        let conn_handle = DRIVER.connection_new();

        // TODO: temporary conversion, proper options mapping will be done later
        let mut converted_options: HashMap<String, Setting> = options
            .iter()
            .map(|(k, v)| (k.clone(), Setting::String(v.clone())))
            .collect();
        if std::env::var(NODE_TLS_REJECT_UNAUTHORIZED).as_deref() == Ok("0") {
            converted_options
                .entry(param_names::TLS_SKIP_VERIFY.as_str().to_string())
                .or_insert_with(|| Setting::String("true".to_string()));
        }
        if let Ok(ca_path) = std::env::var(NODE_EXTRA_CA_CERTS)
            && !ca_path.is_empty()
        {
            converted_options
                .entry(param_names::EXTRA_ROOT_STORE_PATH.as_str().to_string())
                .or_insert_with(|| Setting::String(ca_path));
        }
        let browser_opener = open_external_browser_callback
            .map(browser_opener_from_js)
            .transpose()?;

        block_on(async {
            DRIVER
                .connection_set_options(conn_handle, converted_options, false, None)
                .await?;
            if !session_parameters.is_empty() {
                DRIVER
                    .connection_set_session_parameters(conn_handle, session_parameters)
                    .await?;
            }
            if let Some(opener) = browser_opener {
                DRIVER
                    .connection_set_browser_opener(conn_handle, opener)
                    .await?;
            }
            DRIVER
                .set_wrapper_identity(
                    conn_handle,
                    WrapperIdentity {
                        // TODO: pass this from nodejs (function arguments)
                        driver_name: "JavaScript".to_string(),
                        driver_version: "4.0.0-beta.0".to_string(),
                        language_runtime: "nodejs".to_string(),
                        language_version: "24.0.0".to_string(),
                        language_compiler: None,
                        release_type: None,
                    },
                )
                .await?;
            Ok::<_, ApiError>(())
        })
        .map_err(|e| {
            let _ = DRIVER.connection_release(conn_handle);
            let _ = DRIVER.database_release(database_handle);
            e.to_js_error(*env)
        })?;

        Ok(Self {
            session: Session::new(conn_handle, database_handle),
        })
    }

    #[napi]
    pub fn connect(&self, env: &Env) -> Result<AsyncBlock<()>> {
        let session = self.session.clone();
        async_to_js(env, async move { session.connect().await })
    }

    #[napi]
    pub fn is_up(&self) -> bool {
        block_on(self.session.is_up())
    }

    #[napi]
    pub fn is_valid_async(&self, env: &Env) -> Result<AsyncBlock<bool>> {
        let session = self.session.clone();
        async_to_js(env, async move {
            Ok::<bool, BridgeError>(session.is_valid().await)
        })
    }

    #[napi]
    pub fn get_session_parameters(&self, env: &Env) -> Result<KnownSessionParameters> {
        block_on(self.session.known_session_parameters()).map_err(|e| e.to_js_error(*env))
    }

    #[napi]
    pub fn execute(
        &self,
        query: String,
        bindings: Option<QueryBindings>,
        parameters: Option<HashMap<String, String>>,
    ) -> Statement {
        let session = self.session.clone();
        let operation_ctx = Arc::new(OperationCtx::with_own_token());
        Statement::from_pending(Some(operation_ctx.clone()), async move {
            let ready = session.ready().await?;
            let stmt_handle = DRIVER.statement_new(ready.connection())?;
            let binding_bytes = bindings.map(|b| (b.format, b.data.into_bytes()));
            let result = async {
                DRIVER.statement_set_sql_query(stmt_handle, query).await?;
                if let Some(parameters) = parameters {
                    let options = parameters
                        .into_iter()
                        .map(|(k, v)| (k, Setting::String(v)))
                        .collect();
                    DRIVER.statement_set_options(stmt_handle, options).await?;
                }
                let bindings = binding_bytes.as_ref().map(|(format, bytes)| {
                    let ptr = DataPtr::new(bytes.as_ptr(), bytes.len() as i64);
                    match format {
                        QueryBindingFormat::Csv => BindingType::Csv(ptr),
                        QueryBindingFormat::Json => BindingType::Json(ptr),
                    }
                });
                DRIVER
                    .statement_execute_query(Some(&operation_ctx), stmt_handle, bindings, None)
                    .await
            }
            .await;
            let _ = DRIVER.statement_release(stmt_handle);
            result
                .map(|result| (ready, result))
                .map_err(BridgeError::from)
        })
    }

    #[napi]
    pub fn get_query_status(&self, env: &Env, query_id: String) -> Result<AsyncBlock<QueryStatus>> {
        let session = self.session.clone();
        async_to_js(env, async move {
            let result = get_query_status_result(&session, &query_id).await?;
            Ok::<_, BridgeError>(QueryStatus::parse(&result.status_name))
        })
    }

    #[napi]
    pub fn get_query_status_throw_if_error(
        &self,
        env: &Env,
        query_id: String,
    ) -> Result<AsyncBlock<QueryStatus>> {
        let session = self.session.clone();
        async_to_js(env, async move {
            let result = get_query_status_result(&session, &query_id).await?;
            let status = QueryStatus::parse(&result.status_name);
            if status.is_an_error() {
                return Err(BridgeError::QueryStatusFailed {
                    query_id,
                    error_code: result.error_code,
                    error_message: result.error_message,
                });
            }
            Ok(status)
        })
    }

    #[napi]
    pub fn get_query_result(&self, query_id: String) -> Statement {
        let session = self.session.clone();
        // Shared with the `Statement` handed back, whose `cancel()` triggers it.
        let operation_ctx = Arc::new(OperationCtx::with_own_token());
        Statement::from_pending(Some(operation_ctx.clone()), async move {
            require_valid_query_id(&query_id)?;
            let ready = session.ready().await?;
            DRIVER
                .connection_get_query_result(Some(&operation_ctx), ready.connection(), query_id)
                .await
                .map(|result| (ready, result))
                .map_err(BridgeError::from)
        })
    }

    // TODO: destroy does not release core handles when the connection was never
    // established.
    #[napi]
    pub fn destroy(&self, env: &Env) -> Result<AsyncBlock<()>> {
        let session = self.session.clone();
        async_to_js(env, async move { session.close().await })
    }
}

/// Adapts `openExternalBrowserCallback` to the synchronous opener core invokes.
fn browser_opener_from_js(callback: Function<String, ()>) -> Result<BrowserOpenFn> {
    let open_in_js = callback.build_threadsafe_function::<String>().build()?;
    Ok(Arc::new(move |url: &str| {
        let (outcome_tx, outcome_rx) = std::sync::mpsc::sync_channel(1);
        let queued = open_in_js.call_with_return_value(
            url.to_string(),
            ThreadsafeFunctionCallMode::NonBlocking,
            move |returned, _| {
                let _ = outcome_tx.send(returned.map_err(|thrown| thrown.reason));
                Ok(())
            },
        );
        if queued != Status::Ok {
            return Err(format!("openExternalBrowserCallback failed: {queued}"));
        }
        outcome_rx
            .recv()
            .unwrap_or_else(|_| Err("openExternalBrowserCallback did not return".into()))
    }))
}

async fn get_query_status_result(
    session: &Session,
    query_id: &str,
) -> std::result::Result<QueryStatusResult, BridgeError> {
    require_valid_query_id(query_id)?;
    let ready = session.ready().await?;
    let operation_ctx = OperationCtx::with_own_token();
    DRIVER
        .connection_get_query_status(Some(&operation_ctx), ready.connection(), query_id)
        .await
        .map_err(BridgeError::from)
}
