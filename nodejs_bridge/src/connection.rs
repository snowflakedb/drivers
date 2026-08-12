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
    database_handle: Handle,
    /// Cancellation context for the in-flight `connect()`. Node owns its own
    /// token rather than going through the transport's handle registry, since it
    /// calls the driver API directly and never crosses the protobuf layer.
    connect_ctx: OperationCtx,
}

#[napi]
impl Connection {
    #[napi(constructor)]
    pub fn new(options: HashMap<String, String>) -> Result<Self> {
        let database_handle = DRIVER.database_new();
        if let Err(error) = DRIVER.database_init(database_handle) {
            let _ = DRIVER.database_release(database_handle);
            return Err(to_napi_err(error));
        }

        let conn_handle = DRIVER.connection_new();

        // TODO: temporary conversion, proper options mapping will be done later
        let converted_options = options
            .iter()
            .map(|(k, v)| (k.clone(), Setting::String(v.clone())))
            .collect();

        if let Err(error) = block_on(async {
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
        }) {
            let _ = DRIVER.connection_release(conn_handle);
            let _ = DRIVER.database_release(database_handle);
            return Err(to_napi_err(error));
        }

        Ok(Self {
            handle: conn_handle,
            database_handle,
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
            .connection_init(Some(&self.connect_ctx), self.handle, self.database_handle)
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
            let result = DRIVER
                .statement_execute_query(stmt_handle, None, None)
                .await?;
            let _ = DRIVER.statement_release(stmt_handle);
            Ok(result)
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
        let close_result = DRIVER.connection_close(self.handle).await;
        let connection_release_result = DRIVER.connection_release(self.handle);
        let database_release_result = DRIVER.database_release(self.database_handle);

        close_result.map_err(to_napi_err)?;
        connection_release_result.map_err(to_napi_err)?;
        database_release_result.map_err(to_napi_err)
    }
}
