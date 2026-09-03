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
use std::sync::atomic::{AtomicBool, AtomicU8, Ordering};

#[napi]
pub struct Connection {
    /// Shared with every in-flight operation, so the handles outlive this object
    /// for as long as any of them still needs them.
    handles: Arc<Handles>,
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

/// Owns the connection and database handles, releasing them once every holder
/// is finished with them.
///
/// Async methods clone the `Arc` into their futures, so a `Connection` the JS
/// side drops mid-operation cannot pull the handles out from under work that is
/// still running. Releasing early would make `connection_init` fail to find the
/// connection — or, worse, leave a server session established against a handle
/// that is already gone and can therefore never be closed.
///
/// Release is on the last holder rather than on the JS object alone because
/// `sf_core`'s handle managers never reuse ids: a handle nobody releases is
/// consumed for the life of the process, and nothing obliges a JS caller to
/// invoke `destroy()`.
pub(crate) struct Handles {
    pub(crate) connection: Handle,
    database: Handle,
    released: AtomicBool,
}

impl Handles {
    fn new(connection: Handle, database: Handle) -> Arc<Self> {
        Arc::new(Self {
            connection,
            database,
            released: AtomicBool::new(false),
        })
    }

    /// Release without waiting for the last holder, so `destroy()` frees core
    /// resources when the caller asks rather than at some later GC. Idempotent,
    /// so the final drop does not release a second time.
    fn release(&self) {
        if self.released.swap(true, Ordering::AcqRel) {
            return;
        }
        let _ = DRIVER.connection_release(self.connection);
        let _ = DRIVER.database_release(self.database);
    }
}

/// Deliberately does not log out: that is network I/O and cannot be run from a
/// finalizer. Callers who need a graceful logout must still call `destroy()`;
/// this only stops the handles from leaking.
impl Drop for Handles {
    fn drop(&mut self) {
        self.release();
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
            let _ = DRIVER.database_release(database_handle);
            e.to_js_error(*env)
        })?;

        Ok(Self {
            handles: Handles::new(conn_handle, database_handle),
            state: ConnectionState::pristine(),
            session_parameters,
        })
    }

    #[napi]
    pub fn connect(&self, env: &Env) -> Result<AsyncBlock<()>> {
        let state = self.state.clone();
        // Moved into the future so login keeps the handles alive: if the JS
        // object became unreachable once the promise existed, releasing them
        // here would either fail the init or strand a live server session on a
        // handle that no longer exists.
        let handles = self.handles.clone();
        async_to_js(env, async move {
            let result = DRIVER
                .connection_init(None, handles.connection, handles.database)
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
            .statement_new(self.handles.connection)
            .map_err(|e| e.to_js_error(*env))?;
        let operation_ctx = Arc::new(OperationCtx::with_own_token());
        Ok(Statement::from_pending(
            self.handles.clone(),
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
        let conn_handle = self.handles.connection;
        // Shared with the `Statement` handed back, whose `cancel()` triggers it.
        let operation_ctx = Arc::new(OperationCtx::with_own_token());
        Statement::from_pending(
            self.handles.clone(),
            Some(operation_ctx.clone()),
            async move {
                DRIVER
                    .connection_get_query_result(Some(&operation_ctx), conn_handle, query_id)
                    .await
            },
        )
    }

    #[napi]
    pub fn destroy(&self, env: &Env) -> Result<AsyncBlock<()>> {
        let state = self.state.clone();
        // Held for the whole close, so the handles cannot be released — by a
        // `Drop` or anything else — before `connection_close` has used them.
        // Releasing early would leave the session logged in server-side.
        let handles = self.handles.clone();
        async_to_js(env, async move {
            let close = DRIVER.connection_close(handles.connection).await;
            if close.is_ok() {
                // Only once core confirms the session is gone. A failed close is
                // settled back to `Open` there precisely so a later `destroy()`
                // can retry it; releasing regardless would strand a live session
                // on handles nobody can reach again. Releasing eagerly on success
                // still frees core resources when the caller asked rather than at
                // the next GC — and on failure the `Arc` this connection holds
                // keeps them alive for the retry, so nothing leaks either way.
                handles.release();
                state.mark_terminated();
            }
            close
        })
    }
}
