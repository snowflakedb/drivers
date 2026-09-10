use crate::DRIVER;
use crate::error::{BridgeError, ConnectionOperation, ToJsError, UnusableConnection, async_to_js};
use crate::session_params::KnownSessionParameters;
use crate::statement::Statement;
use napi::bindgen_prelude::*;
use napi_derive::napi;
use sf_core::apis::database_driver_v1::connection::WrapperIdentity;
use sf_core::apis::database_driver_v1::{ApiError, BindingType, DataPtr};
use sf_core::apis::operation_ctx::OperationCtx;
use sf_core::config::settings::Setting;
use sf_core::handle_manager::Handle;
use std::collections::HashMap;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use tokio::sync::Mutex;

#[napi]
pub struct Connection {
    /// Shared with every in-flight operation, so the handles outlive this object
    /// for as long as any of them still needs them.
    handles: Arc<Handles>,
    /// Serializes the lifecycle transitions so `destroy()` cannot close and
    /// release while `connect()` is still initializing.
    ///
    /// Nothing stops JS from calling both without awaiting the first, and core
    /// reports a *successful* close for a connection that was never
    /// initialized. Unserialized, a `destroy()` could therefore close, release
    /// the handles and return while the concurrent `connection_init` went on to
    /// establish a real session — one now sitting behind handles that no longer
    /// exist, so it could never be reached again, let alone logged out.
    lifecycle: Arc<Mutex<()>>,
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

// TODO: ask core to bundle these three reads into one call.
async fn unusable_connection(handle: Handle) -> Option<UnusableConnection> {
    if DRIVER.connection_is_closed(handle).await.unwrap_or(true) {
        return Some(UnusableConnection::Terminated);
    }
    if DRIVER.connection_is_expired(handle).await.unwrap_or(true) {
        return Some(UnusableConnection::Terminated);
    }
    match DRIVER.connection_is_initialized(handle).await {
        Ok(true) => None,
        Ok(false) => Some(UnusableConnection::NeverEstablished),
        Err(_) => Some(UnusableConnection::Terminated),
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
            if !session_parameters.is_empty() {
                DRIVER
                    .connection_set_session_parameters(conn_handle, session_parameters)
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
            handles: Handles::new(conn_handle, database_handle),
            lifecycle: Arc::new(Mutex::new(())),
        })
    }

    #[napi]
    pub fn connect(&self, env: &Env) -> Result<AsyncBlock<()>> {
        // Moved into the future so login keeps the handles alive: if the JS
        // object became unreachable once the promise existed, releasing them
        // here would either fail the init or strand a live server session on a
        // handle that no longer exists.
        let handles = self.handles.clone();
        let lifecycle = self.lifecycle.clone();
        async_to_js(env, async move {
            // Held across the init so a concurrent `destroy()` cannot release
            // the handles midway and orphan the session this is establishing.
            let _lifecycle = lifecycle.lock().await;
            let result = DRIVER
                .connection_init(None, handles.connection, handles.database)
                .await;
            if result.is_err() {
                // A failed login is neither initialized nor closed; this settles it as terminated.
                let _ = DRIVER.connection_close(handles.connection).await;
            }
            result
        })
    }

    #[napi]
    pub fn is_up(&self) -> bool {
        block_on(is_usable(self.handles.connection))
    }

    #[napi]
    pub fn is_valid_async(&self, env: &Env) -> Result<AsyncBlock<bool>> {
        let handle = self.handles.connection;
        async_to_js(env, async move {
            if !is_usable(handle).await {
                return Ok::<bool, BridgeError>(false);
            }
            Ok(DRIVER.connection_heartbeat(handle).await.unwrap_or(false))
        })
    }

    #[napi]
    pub fn get_session_parameters(&self, env: &Env) -> Result<KnownSessionParameters> {
        block_on(KnownSessionParameters::from_connection(
            self.handles.connection,
        ))
        .map_err(|e| e.to_js_error(*env))
    }

    #[napi]
    pub fn execute(
        &self,
        query: String,
        bindings: Option<QueryBindings>,
        parameters: Option<HashMap<String, String>>,
    ) -> Statement {
        let handles = self.handles.clone();
        let operation_ctx = Arc::new(OperationCtx::with_own_token());
        Statement::from_pending(
            self.handles.clone(),
            Some(operation_ctx.clone()),
            async move {
                refuse_if_unusable(handles.connection).await?;
                let stmt_handle = DRIVER.statement_new(handles.connection)?;
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
                result.map_err(BridgeError::from)
            },
        )
    }

    #[napi]
    pub fn get_query_result(&self, query_id: String) -> Statement {
        let handles = self.handles.clone();
        // Shared with the `Statement` handed back, whose `cancel()` triggers it.
        let operation_ctx = Arc::new(OperationCtx::with_own_token());
        Statement::from_pending(
            self.handles.clone(),
            Some(operation_ctx.clone()),
            async move {
                refuse_if_unusable(handles.connection).await?;
                DRIVER
                    .connection_get_query_result(Some(&operation_ctx), handles.connection, query_id)
                    .await
                    .map_err(BridgeError::from)
            },
        )
    }

    #[napi]
    pub fn destroy(&self, env: &Env) -> Result<AsyncBlock<()>> {
        // Held for the whole close, so the handles cannot be released — by a
        // `Drop` or anything else — before `connection_close` has used them.
        // Releasing early would leave the session logged in server-side.
        let handles = self.handles.clone();
        let lifecycle = self.lifecycle.clone();
        async_to_js(env, async move {
            // Waits out an in-flight `connect()`. Core reports a successful
            // close for a connection that was never initialized, so without
            // this the close could "succeed" and release the handles while the
            // init it raced went on to establish a session nobody could reach.
            // Teardown now always runs against a settled connection: either
            // fully established, or one that never came up.
            let _lifecycle = lifecycle.lock().await;
            // Under the lock, so a destroy that waited out a connect answers for
            // the session that connect left behind.
            if let Some(unusable) = unusable_connection(handles.connection).await {
                return Err(BridgeError::UnusableConnection(
                    ConnectionOperation::Destroy,
                    unusable,
                ));
            }
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
            }
            close.map_err(BridgeError::from)
        })
    }
}

async fn is_usable(handle: Handle) -> bool {
    unusable_connection(handle).await.is_none()
}

async fn refuse_if_unusable(handle: Handle) -> std::result::Result<(), BridgeError> {
    match unusable_connection(handle).await {
        Some(unusable) => Err(BridgeError::UnusableConnection(
            ConnectionOperation::Request,
            unusable,
        )),
        None => Ok(()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn a_connection_nobody_touched_was_never_established() {
        let handle = DRIVER.connection_new();

        assert!(matches!(
            unusable_connection(handle).await,
            Some(UnusableConnection::NeverEstablished)
        ));

        DRIVER.connection_release(handle).unwrap();
    }

    #[tokio::test]
    async fn a_closed_connection_is_terminated() {
        let handle = DRIVER.connection_new();
        DRIVER.connection_close(handle).await.unwrap();

        assert!(matches!(
            unusable_connection(handle).await,
            Some(UnusableConnection::Terminated)
        ));

        DRIVER.connection_release(handle).unwrap();
    }

    #[tokio::test]
    async fn a_released_handle_is_terminated() {
        let handle = DRIVER.connection_new();
        DRIVER.connection_release(handle).unwrap();

        assert!(matches!(
            unusable_connection(handle).await,
            Some(UnusableConnection::Terminated)
        ));
    }
}
