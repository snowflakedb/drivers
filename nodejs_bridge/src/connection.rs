use crate::BRIDGE;
use crate::error::{BridgeError, ToJsError, async_to_js};
use crate::query;
use crate::query_status::QueryStatus;
use crate::session::Session;
use crate::session_params::KnownSessionParameters;
use crate::statement::Statement;
use crate::validation_utils::{require_valid_query_id, require_valid_request_id};
use napi::bindgen_prelude::*;
use napi::threadsafe_function::ThreadsafeFunctionCallMode;
use napi_derive::napi;
use sf_core::apis::database_driver_v1::connection::WrapperIdentity;
use sf_core::apis::database_driver_v1::{ApiError, BindingType, DataPtr};
use sf_core::apis::operation_ctx::OperationCtx;
use sf_core::config::param_names;
use sf_core::config::rest_parameters::BrowserOpenFn;
use sf_core::config::settings::Setting;
use sf_core::handle_manager::Handle;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

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

#[napi(object)]
pub struct ExecuteParams {
    pub query: String,
    pub bindings: Option<QueryBindings>,
    pub parameters: Option<HashMap<String, String>>,
    pub async_exec: Option<bool>,
    pub request_id: Option<String>,
    pub describe_only: Option<bool>,
}

#[napi(object)]
pub struct ConnectionTokenInfo {
    pub session_token: String,
    pub master_token: String,
    pub session_token_expires_at_ms: Option<i64>,
    pub master_token_expires_at_ms: Option<i64>,
}

#[napi(object, object_to_js = false)]
pub struct ConnectionParams<'a> {
    pub options: HashMap<String, String>,
    pub session_parameters: HashMap<String, String>,
    pub open_external_browser_callback: Option<Function<'a, String, ()>>,
    pub token_info: Option<ConnectionTokenInfo>,
}

#[napi]
impl Connection {
    #[napi(constructor)]
    pub fn new(env: &Env, params: ConnectionParams) -> Result<Self> {
        let ConnectionParams {
            options,
            session_parameters,
            open_external_browser_callback,
            token_info,
        } = params;
        let database_handle = BRIDGE.driver.database_new();
        BRIDGE.driver.database_init(database_handle).map_err(|e| {
            let _ = BRIDGE.driver.database_release(database_handle);
            e.to_js_error(*env)
        })?;

        let conn_handle = BRIDGE.driver.connection_new();

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
        if let Some(token_info) = &token_info {
            converted_options.insert(
                param_names::SESSION_TOKEN.as_str().to_string(),
                Setting::String(token_info.session_token.clone()),
            );
            converted_options.insert(
                param_names::MASTER_TOKEN.as_str().to_string(),
                Setting::String(token_info.master_token.clone()),
            );
        }
        let browser_opener = open_external_browser_callback
            .map(browser_opener_from_js)
            .transpose()?;

        block_on(async {
            BRIDGE
                .driver
                .connection_set_options(conn_handle, converted_options, false, None)
                .await?;
            if !session_parameters.is_empty() {
                BRIDGE
                    .driver
                    .connection_set_session_parameters(conn_handle, session_parameters)
                    .await?;
            }
            if let Some(opener) = browser_opener {
                BRIDGE
                    .driver
                    .connection_set_browser_opener(conn_handle, opener)
                    .await?;
            }
            BRIDGE
                .driver
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
                        application_path: None,
                    },
                )
                .await?;

            // A connection built from session and master tokens never goes through `connect()`,
            // so it is initialized here. This is not cheap yet: it runs the full core connection
            // setup on the JS thread, and on macOS loading the system root certificates alone takes
            // about 100 ms. SNOW-4249447 and SNOW-4249371 track making it cheap.
            if token_info.is_some() {
                BRIDGE
                    .driver
                    .connection_init(None, conn_handle, database_handle)
                    .await?;
            }

            Ok::<_, ApiError>(())
        })
        .map_err(|e| {
            let _ = BRIDGE.driver.connection_release(conn_handle);
            let _ = BRIDGE.driver.database_release(database_handle);
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
    pub fn get_token_info(&self, env: &Env) -> Result<Option<ConnectionTokenInfo>> {
        let Some(info) = block_on(self.session.token_info()).map_err(|e| e.to_js_error(*env))?
        else {
            return Ok(None);
        };
        let (Some(session_token), Some(master_token)) = (info.session_token, info.master_token)
        else {
            return Ok(None);
        };
        Ok(Some(ConnectionTokenInfo {
            session_token: session_token.reveal().to_string(),
            master_token: master_token.reveal().to_string(),
            session_token_expires_at_ms: info.session_token_expires_at_ms,
            master_token_expires_at_ms: info.master_token_expires_at_ms,
        }))
    }

    #[napi]
    pub fn execute(&self, env: &Env, params: ExecuteParams) -> Result<Statement> {
        let session = self.session.clone();
        let operation_ctx = Arc::new(OperationCtx::with_own_token());
        let ExecuteParams {
            query,
            bindings,
            parameters,
            async_exec,
            request_id,
            describe_only,
        } = params;
        let request_id = resolve_execute_request_id(request_id, uuid::Uuid::new_v4)
            .map_err(|error| error.to_js_error(*env))?;
        if async_exec.unwrap_or(false) {
            Ok(Statement::from_async_exec(
                Some(operation_ctx.clone()),
                Some(request_id),
                async move {
                    let ready = session.ready().await?;
                    run_new_statement(
                        ready.connection(),
                        query,
                        bindings,
                        parameters,
                        async move |stmt, binds| {
                            BRIDGE
                                .driver
                                .statement_execute_async(
                                    Some(&operation_ctx),
                                    stmt,
                                    binds,
                                    Some(request_id),
                                    describe_only,
                                )
                                .await
                                .map_err(BridgeError::from)
                        },
                    )
                    .await
                },
            ))
        } else {
            Ok(Statement::from_query_result(
                Some(operation_ctx.clone()),
                Some(request_id),
                async move {
                    let ready = session.ready().await?;
                    run_new_statement(
                        ready.connection(),
                        query,
                        bindings,
                        parameters,
                        async move |stmt, binds| {
                            BRIDGE
                                .driver
                                .statement_execute_query(
                                    Some(&operation_ctx),
                                    stmt,
                                    binds,
                                    None,
                                    Some(request_id),
                                    describe_only,
                                )
                                .await
                                .map_err(BridgeError::from)
                        },
                    )
                    .await
                    .map(|result| (ready, result))
                },
            ))
        }
    }

    #[napi]
    pub fn get_query_status(&self, env: &Env, query_id: String) -> Result<AsyncBlock<QueryStatus>> {
        let session = self.session.clone();
        async_to_js(
            env,
            async move { query::get_status(&session, &query_id).await },
        )
    }

    #[napi]
    pub fn get_query_status_throw_if_error(
        &self,
        env: &Env,
        query_id: String,
    ) -> Result<AsyncBlock<QueryStatus>> {
        let session = self.session.clone();
        async_to_js(env, async move {
            query::get_status_throw_if_error(&session, &query_id).await
        })
    }

    #[napi]
    pub fn wait_for_query_result(
        &self,
        env: &Env,
        query_id: String,
        retry_interval_ms: Option<u32>,
    ) -> Result<AsyncBlock<()>> {
        let session = self.session.clone();
        let retry_interval = retry_interval_ms.map(|ms| Duration::from_millis(ms as u64));
        async_to_js(env, async move {
            query::wait_for_result(&session, query_id, retry_interval).await
        })
    }

    #[napi]
    pub fn get_query_result(&self, query_id: String) -> Statement {
        let session = self.session.clone();
        // Shared with the `Statement` handed back, whose `cancel()` triggers it.
        let operation_ctx = Arc::new(OperationCtx::with_own_token());
        Statement::from_query_result(Some(operation_ctx.clone()), None, async move {
            require_valid_query_id(&query_id)?;
            let ready = session.ready().await?;
            BRIDGE
                .driver
                .connection_get_query_result(Some(&operation_ctx), ready.connection(), query_id)
                .await
                .map(|result| (ready, result))
                .map_err(BridgeError::from)
        })
    }

    #[napi]
    pub fn is_an_error(&self, status: QueryStatus) -> bool {
        status.is_an_error()
    }

    #[napi]
    pub fn is_still_running(&self, status: QueryStatus) -> bool {
        status.is_still_running()
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

fn resolve_execute_request_id(
    request_id: Option<String>,
    uuid_supplier: impl FnOnce() -> uuid::Uuid,
) -> std::result::Result<uuid::Uuid, BridgeError> {
    match request_id.filter(|id| !id.is_empty()) {
        None => Ok(uuid_supplier()),
        // Uuid::parse_str also accepts 32-char, braced, and urn forms.
        // require_valid_request_id keeps requestId on the same hyphenated check as queryId.
        Some(id) => require_valid_request_id(&id).and_then(|()| {
            uuid::Uuid::parse_str(&id).map_err(|_| BridgeError::InvalidRequestId(id))
        }),
    }
}

async fn run_new_statement<T>(
    connection: Handle,
    query: String,
    bindings: Option<QueryBindings>,
    parameters: Option<HashMap<String, String>>,
    run: impl for<'a> AsyncFnOnce(
        Handle,
        Option<BindingType<'a>>,
    ) -> std::result::Result<T, BridgeError>,
) -> std::result::Result<T, BridgeError> {
    let stmt_handle = BRIDGE.driver.statement_new(connection)?;
    let binding_bytes = bindings.map(|b| (b.format, b.data.into_bytes()));
    let result = async {
        BRIDGE
            .driver
            .statement_set_sql_query(stmt_handle, query)
            .await?;
        if let Some(parameters) = parameters {
            let options = parameters
                .into_iter()
                .map(|(k, v)| (k, Setting::String(v)))
                .collect();
            BRIDGE
                .driver
                .statement_set_options(stmt_handle, options)
                .await?;
        }
        let bindings = binding_bytes.as_ref().map(|(format, bytes)| {
            let ptr = DataPtr::new(bytes.as_ptr(), bytes.len() as i64);
            match format {
                QueryBindingFormat::Csv => BindingType::Csv(ptr),
                QueryBindingFormat::Json => BindingType::Json(ptr),
            }
        });
        run(stmt_handle, bindings).await
    }
    .await;
    let _ = BRIDGE.driver.statement_release(stmt_handle);
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolve_execute_request_id_creates_uuid_when_omitted() {
        let supplied = uuid::Uuid::from_u128(0x1234_5678_1234_4123_a123_1234_5678_9012);
        let Ok(parsed) = resolve_execute_request_id(None, || supplied) else {
            panic!("minted request id parses");
        };
        assert_eq!(parsed, supplied);
    }

    #[test]
    fn resolve_execute_request_id_creates_uuid_when_empty() {
        let supplied = uuid::Uuid::from_u128(0x1234_5678_1234_4123_a123_1234_5678_9012);
        let Ok(parsed) = resolve_execute_request_id(Some(String::new()), || supplied) else {
            panic!("empty request id mints");
        };
        assert_eq!(parsed, supplied);
    }

    #[test]
    fn resolve_execute_request_id_keeps_a_valid_caller_id() {
        let request_id = "12345678-1234-4123-A123-123456789012";
        let Ok(parsed) =
            resolve_execute_request_id(Some(request_id.to_string()), uuid::Uuid::new_v4)
        else {
            panic!("valid request id parses");
        };
        assert_eq!(
            parsed.to_string().to_ascii_lowercase(),
            request_id.to_ascii_lowercase()
        );
    }

    #[test]
    fn resolve_execute_request_id_rejects_malformed_values() {
        let request_id = "foobar";
        assert!(matches!(
            resolve_execute_request_id(Some(request_id.to_string()), uuid::Uuid::new_v4),
            Err(BridgeError::InvalidRequestId(id)) if id == request_id
        ));
    }
}
