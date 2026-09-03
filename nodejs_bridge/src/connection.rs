use crate::DRIVER;
use crate::error::{ToJsError, UnusableConnection, async_to_js};
use crate::statement::Statement;
use napi::bindgen_prelude::*;
use napi_derive::napi;
use sf_core::apis::database_driver_v1::ApiError;
use sf_core::apis::database_driver_v1::connection::WrapperIdentity;
use sf_core::apis::operation_ctx::OperationCtx;
use sf_core::config::settings::Setting;
use sf_core::handle_manager::Handle;
use std::collections::HashMap;
use std::sync::Arc;
use std::sync::atomic::{AtomicU8, Ordering};

#[napi]
pub struct Connection {
    handle: Handle,
    state: ConnectionState,
    /// TEMPORARY: Copied onto every [`Statement`] this connection creates.\
    session_parameters: HashMap<String, String>,
}

/// Tracked here because the core cannot answer for it: `destroy` releases the
/// handle, and a failed login leaves the core reporting the same error as for a
/// connection nobody touched.
#[derive(Clone)]
struct ConnectionState(Arc<AtomicU8>);

impl ConnectionState {
    const PRISTINE: u8 = 0;
    const CONNECTED: u8 = 1;
    const TERMINATED: u8 = 2;

    fn pristine() -> Self {
        Self(Arc::new(AtomicU8::new(Self::PRISTINE)))
    }

    fn mark_connected(&self) {
        self.0.store(Self::CONNECTED, Ordering::Relaxed);
    }

    fn mark_terminated(&self) {
        self.0.store(Self::TERMINATED, Ordering::Relaxed);
    }

    fn unusable(&self) -> Option<UnusableConnection> {
        match self.0.load(Ordering::Relaxed) {
            Self::CONNECTED => None,
            Self::TERMINATED => Some(UnusableConnection::Terminated),
            _ => Some(UnusableConnection::NeverEstablished),
        }
    }
}

#[napi]
impl Connection {
    #[napi(constructor)]
    pub fn new(
        options: HashMap<String, String>,
        env: &Env,
        session_parameters: HashMap<String, String>,
    ) -> Result<Self> {
        let conn_handle = DRIVER.connection_new();

        // TODO: temporary conversion, proper options mapping will be done later
        let mut converted_options: HashMap<String, Setting> = options
            .iter()
            .map(|(k, v)| (k.clone(), Setting::String(v.clone())))
            .collect();
        if std::env::var("NODE_TLS_REJECT_UNAUTHORIZED").as_deref() == Ok("0") {
            converted_options
                .entry("tls_skip_verify".to_string())
                .or_insert_with(|| Setting::String("true".to_string()));
        }
        if let Ok(ca_path) = std::env::var("NODE_EXTRA_CA_CERTS")
            && !ca_path.is_empty()
        {
            converted_options
                .entry("custom_root_store_path".to_string())
                .or_insert_with(|| Setting::String(ca_path));
        }

        block_on(async {
            DRIVER
                .connection_set_options(conn_handle, converted_options, false)
                .await?;
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
            e.to_js_error(*env)
        })?;

        Ok(Self {
            handle: conn_handle,
            state: ConnectionState::pristine(),
            session_parameters,
        })
    }

    #[napi]
    pub fn connect(&self, env: &Env) -> Result<AsyncBlock<()>> {
        let handle = self.handle;
        let state = self.state.clone();
        async_to_js(env, async move {
            // TODO:
            // The _db_handle parameter is currently unused but required; passing a dummy value for now.
            // This argument is planned for removal from connection_init in a future update.
            let result = DRIVER
                .connection_init(None, handle, Handle { id: 0, magic: 0 })
                .await;
            match result {
                Ok(()) => state.mark_connected(),
                Err(_) => state.mark_terminated(),
            }
            result
        })
    }

    #[napi]
    pub fn get_session_parameter(&self, name: String) -> Option<String> {
        self.session_parameters.get(&name).cloned()
    }

    #[napi]
    pub fn execute(&self, env: &Env, query: String) -> Result<Statement> {
        if let Some(unusable) = self.state.unusable() {
            return Ok(Statement::refused(unusable));
        }
        let stmt_handle = DRIVER
            .statement_new(self.handle)
            .map_err(|e| e.to_js_error(*env))?;
        let operation_ctx = Arc::new(OperationCtx::with_own_token());
        Ok(Statement::from_pending(
            self.handle,
            Some(operation_ctx.clone()),
            async move {
                let result = async {
                    DRIVER.statement_set_sql_query(stmt_handle, query).await?;
                    DRIVER
                        .statement_execute_query(Some(&operation_ctx), stmt_handle, None, None)
                        .await
                }
                .await;
                let _ = DRIVER.statement_release(stmt_handle);
                result
            },
        ))
    }

    #[napi]
    pub fn get_query_result(&self, query_id: String) -> Statement {
        if let Some(unusable) = self.state.unusable() {
            return Statement::refused(unusable);
        }
        let conn_handle = self.handle;
        // Shared with the `Statement` handed back, whose `cancel()` triggers it.
        let operation_ctx = Arc::new(OperationCtx::with_own_token());
        Statement::from_pending(conn_handle, Some(operation_ctx.clone()), async move {
            DRIVER
                .connection_get_query_result(Some(&operation_ctx), conn_handle, query_id)
                .await
        })
    }

    #[napi]
    pub fn destroy(&self, env: &Env) -> Result<AsyncBlock<()>> {
        let handle = self.handle;
        let state = self.state.clone();
        async_to_js(env, async move {
            let close = DRIVER.connection_close(handle).await;
            let _ = DRIVER.connection_release(handle);
            state.mark_terminated();
            close
        })
    }
}
