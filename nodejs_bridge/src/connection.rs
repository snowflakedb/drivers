use crate::DRIVER;
use crate::error::to_napi_err;
use crate::statement::Statement;
use napi::bindgen_prelude::*;
use napi_derive::napi;
use sf_core::apis::database_driver_v1::ApiError;
use sf_core::apis::database_driver_v1::connection::WrapperIdentity;
use sf_core::apis::operation_ctx::OperationCtx;
use sf_core::config::settings::Setting;
use sf_core::handle_manager::Handle;
use std::collections::HashMap;

#[napi]
pub struct Connection {
    handle: Handle,
    /// Cancellation context for the in-flight `connect()`. Node owns its own
    /// token rather than going through the transport's handle registry, since it
    /// calls the driver API directly and never crosses the protobuf layer.
    connect_ctx: OperationCtx,
}

#[napi]
impl Connection {
    #[napi(constructor)]
    pub fn new(options: HashMap<String, String>) -> Result<Self> {
        let conn_handle = DRIVER.connection_new();

        // TODO: temporary conversion, proper options mapping will be done later
        let converted_options = options
            .iter()
            .map(|(k, v)| (k.clone(), Setting::String(v.clone())))
            .collect();

        block_on(async {
            DRIVER
                .connection_set_options(conn_handle, converted_options, false)
                .await?;
            DRIVER
                .set_wrapper_identity(
                    conn_handle,
                    WrapperIdentity {
                        // TODO: pass this from nodejs (function arguments)
                        driver_name: "nodejs".to_string(),
                        driver_version: "0.1.0".to_string(),
                        language_runtime: "nodejs".to_string(),
                        language_version: "24.0.0".to_string(),
                        language_compiler: None,
                        release_type: None,
                    },
                )
                .await?;
            Ok::<_, ApiError>(())
        })
        // TODO: must release connection on any error
        .map_err(to_napi_err)?;

        Ok(Self {
            handle: conn_handle,
            connect_ctx: OperationCtx::with_own_token(),
        })
    }

    /// Cancel an in-flight [`Self::connect`] from another JS tick or thread.
    /// A no-op once connect has finished.
    #[napi]
    pub fn cancel_connect(&self) {
        self.connect_ctx.cancel();
    }

    #[napi]
    pub async fn connect(&self) -> Result<()> {
        DRIVER
            // TODO:
            // The _db_handle parameter is currently unused but required; passing a dummy value for now.
            // This argument is planned for removal from connection_init in a future update.
            .connection_init(
                Some(&self.connect_ctx),
                self.handle,
                Handle { id: 0, magic: 0 },
            )
            .await
            .map_err(to_napi_err)
    }

    #[napi]
    pub fn execute(&self, query: String) -> Result<Statement> {
        // TODO:
        // - too much map_err calls :/
        // - must release statement or result set on errors?
        let stmt_handle = DRIVER.statement_new(self.handle).map_err(to_napi_err)?;
        Ok(Statement::from_pending(Some(stmt_handle), async move {
            DRIVER.statement_set_sql_query(stmt_handle, query).await?;
            let outcome = DRIVER
                .statement_execute_query(stmt_handle, None, None)
                .await?;
            let _ = DRIVER.statement_release(stmt_handle);
            // Node.js does not currently surface request_id to its callers (unlike
            // Python's cursor._request_id property) — no consumer needs it yet, so
            // it's intentionally discarded here rather than an oversight.
            Ok(outcome.result)
        }))
    }

    #[napi]
    pub fn get_query_result(&self, query_id: String) -> Statement {
        let conn_handle = self.handle;
        Statement::from_pending(None, async move {
            DRIVER
                .connection_get_query_result(conn_handle, query_id)
                .await
        })
    }

    #[napi]
    pub async fn destroy(&self) -> Result<()> {
        DRIVER
            .connection_close(self.handle)
            .await
            .map_err(to_napi_err)?;
        DRIVER
            .connection_release(self.handle)
            .map_err(to_napi_err)?;
        Ok(())
    }
}
