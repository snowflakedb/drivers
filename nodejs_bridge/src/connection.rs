use crate::error::{BridgeError, ToJsError, async_to_js};
use crate::query;
use crate::query_status::QueryStatus;
use crate::session::{Session, SessionConfig};
use crate::session_params::KnownSessionParameters;
use crate::statement::{ExecuteParams, Statement};
use crate::telemetry::Telemetry;
use napi::bindgen_prelude::*;
use napi::threadsafe_function::ThreadsafeFunctionCallMode;
use napi_derive::napi;
use sf_core::apis::database_driver_v1::connection::WrapperIdentity as CoreWrapperIdentity;
use sf_core::config::param_names;
use sf_core::config::rest_parameters::{BrowserOpenFn, BrowserResponseRenderFn};
use sf_core::config::settings::Setting;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

const NODE_TLS_REJECT_UNAUTHORIZED: &str = "NODE_TLS_REJECT_UNAUTHORIZED";
const NODE_EXTRA_CA_CERTS: &str = "NODE_EXTRA_CA_CERTS";

#[napi(object)]
pub struct WrapperIdentity {
    pub driver_name: String,
    pub driver_version: String,
    pub language_runtime: String,
    pub language_version: String,
    pub language_compiler: Option<String>,
    pub release_type: Option<String>,
    pub application_path: Option<String>,
}

impl From<WrapperIdentity> for CoreWrapperIdentity {
    fn from(identity: WrapperIdentity) -> Self {
        Self {
            driver_name: identity.driver_name,
            driver_version: identity.driver_version,
            language_runtime: identity.language_runtime,
            language_version: identity.language_version,
            language_compiler: identity.language_compiler,
            release_type: identity.release_type,
            application_path: identity.application_path,
        }
    }
}

#[napi(object)]
pub struct ConnectionTokenInfo {
    pub session_token: String,
    pub master_token: String,
    pub session_token_expires_at_ms: Option<i64>,
    pub master_token_expires_at_ms: Option<i64>,
}

#[napi(object, use_nullable = true)]
pub struct BrowserResponseResult {
    pub error: Option<String>,
}

#[napi(object, object_to_js = false)]
pub struct ConnectionParams<'a> {
    // TODO(SNOW-4452005): check whether this can take typed values (bool, number) and map them to
    // the matching `Setting` variants, instead of sending every value as `Setting::String`.
    pub options: HashMap<String, String>,
    pub session_parameters: HashMap<String, String>,
    pub wrapper_identity: WrapperIdentity,
    pub open_external_browser_callback: Option<Function<'a, String, ()>>,
    pub browser_response_renderer: Option<Function<'a, BrowserResponseResult, String>>,
    pub token_info: Option<ConnectionTokenInfo>,
    pub no_connection_details: Option<bool>,
}

impl ConnectionParams<'_> {
    fn build_connection_options(&self) -> HashMap<String, Setting> {
        let mut core_options: HashMap<String, Setting> = self
            .options
            .iter()
            .map(|(k, v)| (k.clone(), Setting::String(v.clone())))
            .collect();

        if std::env::var(NODE_TLS_REJECT_UNAUTHORIZED).as_deref() == Ok("0") {
            core_options.insert(
                param_names::TLS_SKIP_VERIFY.as_str().to_string(),
                Setting::String("true".to_string()),
            );
        }
        if let Ok(ca_path) = std::env::var(NODE_EXTRA_CA_CERTS)
            && !ca_path.is_empty()
        {
            core_options.insert(
                param_names::EXTRA_ROOT_STORE_PATH.as_str().to_string(),
                Setting::String(ca_path),
            );
        }

        if let Some(token_info) = &self.token_info {
            core_options.insert(
                param_names::SESSION_TOKEN.as_str().to_string(),
                Setting::String(token_info.session_token.clone()),
            );
            core_options.insert(
                param_names::MASTER_TOKEN.as_str().to_string(),
                Setting::String(token_info.master_token.clone()),
            );
        }

        core_options
    }
}

#[napi]
pub struct Connection {
    session: Session,
    telemetry: Arc<Telemetry>,
}

#[napi]
impl Connection {
    #[napi(constructor)]
    pub fn new(env: &Env, params: ConnectionParams) -> Result<Self> {
        let options = params.build_connection_options();
        let browser_opener = params
            .open_external_browser_callback
            .map(browser_opener_from_js)
            .transpose()?;
        let browser_response_renderer = params
            .browser_response_renderer
            .map(browser_response_renderer_from_js)
            .transpose()?;
        let session = block_on(Session::open(SessionConfig {
            options,
            no_connection_details: params.no_connection_details.unwrap_or(false),
            session_parameters: params.session_parameters,
            browser_opener,
            browser_response_renderer,
            wrapper_identity: params.wrapper_identity.into(),
        }))
        .map_err(|e| e.to_js_error(*env, None))?;

        Ok(Self {
            session,
            telemetry: Arc::default(),
        })
    }

    #[napi]
    pub fn connect(&self, env: &Env) -> Result<AsyncBlock<()>> {
        let session = self.session.clone();
        async_to_js(env, Some(session.clone()), async move {
            session.connect().await
        })
    }

    #[napi]
    pub fn is_up(&self) -> bool {
        block_on(self.session.is_up())
    }

    #[napi]
    pub fn is_valid_async(&self, env: &Env) -> Result<AsyncBlock<bool>> {
        let session = self.session.clone();
        async_to_js(env, Some(session.clone()), async move {
            Ok::<bool, BridgeError>(session.is_valid().await)
        })
    }

    #[napi]
    pub fn get_session_parameters(&self, env: &Env) -> Result<KnownSessionParameters> {
        block_on(self.session.known_session_parameters())
            .map_err(|e| e.to_js_error(*env, Some(&self.session)))
    }

    #[napi]
    pub fn get_token_info(&self, env: &Env) -> Result<Option<ConnectionTokenInfo>> {
        let Some(info) = block_on(self.session.token_info())
            .map_err(|e| e.to_js_error(*env, Some(&self.session)))?
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
    pub fn execute(
        &self,
        env: &Env,
        params: ExecuteParams,
        build_fallback_inline_bindings: Option<Function<(), String>>,
    ) -> Result<Statement> {
        let build_fallback_inline_bindings = build_fallback_inline_bindings
            .map(|build| {
                build
                    .build_threadsafe_function::<()>()
                    .weak::<true>()
                    .build()
            })
            .transpose()?;
        Statement::execute(self.session.clone(), params, build_fallback_inline_bindings)
            .map_err(|e| e.to_js_error(*env, Some(&self.session)))
    }

    #[napi]
    pub fn get_query_status(&self, env: &Env, query_id: String) -> Result<AsyncBlock<QueryStatus>> {
        let session = self.session.clone();
        async_to_js(env, Some(session.clone()), async move {
            query::get_status(&session, &query_id).await
        })
    }

    #[napi]
    pub fn get_query_status_throw_if_error(
        &self,
        env: &Env,
        query_id: String,
    ) -> Result<AsyncBlock<QueryStatus>> {
        let session = self.session.clone();
        async_to_js(env, Some(session.clone()), async move {
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
        async_to_js(env, Some(session.clone()), async move {
            query::wait_for_result(&session, query_id, retry_interval).await
        })
    }

    #[napi]
    pub fn get_query_result(&self, query_id: String) -> Statement {
        Statement::from_query_id(self.session.clone(), query_id)
    }

    #[napi]
    pub fn is_an_error(&self, status: QueryStatus) -> bool {
        status.is_an_error()
    }

    #[napi]
    pub fn is_still_running(&self, status: QueryStatus) -> bool {
        status.is_still_running()
    }

    #[napi]
    pub fn send_telemetry_event(
        &self,
        env: &Env,
        event_type: String,
        query_id: Option<String>,
    ) -> Result<AsyncBlock<()>> {
        let connection_handle = self.session.connection_handle();
        let telemetry = Arc::clone(&self.telemetry);
        async_to_js(env, None, async move {
            telemetry
                .send_event(connection_handle, event_type, query_id)
                .await
        })
    }

    #[napi]
    pub fn send_error_telemetry_event(&self, exception_type: String, error_source: String) {
        self.session
            .record_wrapper_error(&exception_type, &error_source);
    }

    #[napi]
    pub fn destroy(&self, env: &Env) -> Result<AsyncBlock<()>> {
        let session = self.session.clone();
        async_to_js(
            env,
            Some(session.clone()),
            async move { session.close().await },
        )
    }
}

/// Adapts `openExternalBrowserCallback` to the synchronous opener core invokes.
fn browser_opener_from_js(callback: Function<String, ()>) -> Result<BrowserOpenFn> {
    let open = blocking_js_call("openExternalBrowserCallback", callback)?;
    Ok(Arc::new(move |url: &str| open(url.to_string())))
}

fn browser_response_renderer_from_js(
    callback: Function<BrowserResponseResult, String>,
) -> Result<BrowserResponseRenderFn> {
    let render = blocking_js_call("browserResponseRenderer", callback)?;
    Ok(Arc::new(move |error: Option<&str>| {
        render(BrowserResponseResult {
            error: error.map(str::to_string),
        })
    }))
}

/// Wraps a JS callback as a blocking Rust call: queues it on the JS thread,
/// waits for the return value, and turns a throw into `Err`.
fn blocking_js_call<Arg, Ret>(
    name: &'static str,
    callback: Function<Arg, Ret>,
) -> Result<impl Fn(Arg) -> std::result::Result<Ret, String> + Send + Sync + 'static>
where
    Arg: JsValuesTupleIntoVec + 'static,
    Ret: FromNapiValue + 'static,
{
    let js = callback
        .build_threadsafe_function::<Arg>()
        .weak::<true>()
        .build()?;
    Ok(move |arg: Arg| {
        let (outcome_tx, outcome_rx) = std::sync::mpsc::sync_channel(1);
        let queued = js.call_with_return_value(
            arg,
            ThreadsafeFunctionCallMode::NonBlocking,
            move |returned, _| {
                let _ = outcome_tx.send(returned.map_err(|thrown| thrown.reason));
                Ok(())
            },
        );
        if queued != Status::Ok {
            return Err(format!("{name} failed: {queued}"));
        }
        outcome_rx
            .recv()
            .unwrap_or_else(|_| Err(format!("{name} did not return")))
    })
}
