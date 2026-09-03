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
    database_handle: Handle,
    state: ConnectionState,
    /// Decides which of `destroy()` and [`Drop`] releases the two handles.
    cleanup: Cleanup,
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

/// Decides which of `destroy()` and [`Drop`] gets to release the connection and
/// database handles, so exactly one of them does.
///
/// A plain "already released" flag is not enough. `destroy()` closes the session
/// asynchronously, and the async block captures only the raw handles — nothing
/// keeps the JS object alive meanwhile. If `Drop` ran first and released, the
/// pending `connection_close` would find no connection and skip the logout,
/// stranding the session server-side. So `destroy()` claims ownership
/// *synchronously*, and `Drop` releases only what nobody claimed.
#[derive(Clone)]
struct Cleanup(Arc<AtomicU8>);

impl Cleanup {
    /// Nobody has taken responsibility for the handles yet.
    const IDLE: u8 = 0;
    /// A `destroy()` call owns the close and will release once it finishes.
    const OWNED: u8 = 1;
    /// The handles have been released.
    const RELEASED: u8 = 2;

    fn idle() -> Self {
        Self(Arc::new(AtomicU8::new(Self::IDLE)))
    }

    /// Take responsibility for closing and then releasing. Only the first
    /// `destroy()` succeeds; a later one still closes, but must not release
    /// handles the first call is already accounting for.
    fn claim_for_close(&self) -> bool {
        self.claim(Self::OWNED)
    }

    /// Take responsibility for handles no `destroy()` ever claimed.
    fn claim_abandoned(&self) -> bool {
        self.claim(Self::RELEASED)
    }

    fn claim(&self, to: u8) -> bool {
        self.0
            .compare_exchange(Self::IDLE, to, Ordering::AcqRel, Ordering::Acquire)
            .is_ok()
    }

    fn mark_released(&self) {
        self.0.store(Self::RELEASED, Ordering::Release);
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
            handle: conn_handle,
            database_handle,
            state: ConnectionState::pristine(),
            cleanup: Cleanup::idle(),
            session_parameters,
        })
    }

    #[napi]
    pub fn connect(&self, env: &Env) -> Result<AsyncBlock<()>> {
        let handle = self.handle;
        let database_handle = self.database_handle;
        let state = self.state.clone();
        async_to_js(env, async move {
            let result = DRIVER.connection_init(None, handle, database_handle).await;
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
        let database_handle = self.database_handle;
        let state = self.state.clone();
        let cleanup = self.cleanup.clone();
        // Claimed here, synchronously, rather than inside the async block: a
        // `Drop` landing between this call and the close below must not release
        // the handles, or `connection_close` would find no connection and skip
        // the logout. Safe to claim before the work runs because the async block
        // is scheduled on the runtime by `async_to_js`, so it completes whether
        // or not the caller awaits the promise.
        let owns_release = cleanup.claim_for_close();
        async_to_js(env, async move {
            let close = DRIVER.connection_close(handle).await;
            if owns_release {
                release_handles(handle, database_handle);
                cleanup.mark_released();
            }
            state.mark_terminated();
            close
        })
    }
}

fn release_handles(handle: Handle, database_handle: Handle) {
    let _ = DRIVER.connection_release(handle);
    let _ = DRIVER.database_release(database_handle);
}

/// Reclaim the core handles when the JS object is garbage collected.
///
/// `destroy()` is the documented cleanup path, but nothing obliges a JS caller
/// to invoke it, and `sf_core`'s handle managers never reuse ids — so a
/// `Connection` that is simply dropped would strand both its connection and
/// database handle for the life of the process.
///
/// Only handles no `destroy()` ever claimed are released here. Yanking them out
/// from under an in-flight close would leave the server-side session logged in,
/// which is a worse leak than the one this exists to prevent.
///
/// This deliberately does not log out: that is network I/O and cannot be run
/// from a finalizer. Callers who need a graceful logout must still call
/// `destroy()`; this only stops the handles from leaking.
impl Drop for Connection {
    fn drop(&mut self) {
        if self.cleanup.claim_abandoned() {
            release_handles(self.handle, self.database_handle);
        }
    }
}
