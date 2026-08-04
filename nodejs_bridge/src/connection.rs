use crate::DRIVER;
use crate::error::to_napi_err;
use crate::statement::Statement;
use napi::bindgen_prelude::*;
use napi_derive::napi;
use sf_core::apis::database_driver_v1::connection::WrapperIdentity;
use sf_core::apis::database_driver_v1::{ApiError, ExecuteQueryResult};
use sf_core::config::settings::Setting;
use sf_core::handle_manager::Handle;
use std::collections::HashMap;

#[napi]
pub struct Connection {
    handle: Handle,
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
                    },
                )
                .await?;
            Ok::<_, ApiError>(())
        })
        // TODO: must release connection on any error
        .map_err(to_napi_err)?;

        Ok(Self {
            handle: conn_handle,
        })
    }

    #[napi]
    pub async fn connect(&self) -> Result<()> {
        DRIVER
            // TODO:
            // The _db_handle parameter is currently unused but required; passing a dummy value for now.
            // This argument is planned for removal from connection_init in a future update.
            .connection_init(self.handle, Handle { id: 0, magic: 0 })
            .await
            .map_err(to_napi_err)
    }

    async fn statement_from_result(&self, result: ExecuteQueryResult) -> Result<Statement> {
        let (result_set_handle, descriptor) = match result {
            ExecuteQueryResult::Single(rs) => (rs.handle, rs.descriptor),
            ExecuteQueryResult::Multi { .. } => {
                return Err(to_napi_err("multi-statement results are not supported yet"));
            }
        };
        let batch_reader = DRIVER
            .result_set_get_stream(result_set_handle)
            .await
            .map_err(to_napi_err)?;
        Ok(Statement::new(result_set_handle, descriptor, batch_reader))
    }

    #[napi]
    pub async fn execute(&self, query: String) -> Result<Statement> {
        // TODO:
        // - too much map_err calls :/
        // - must release statement or result set on errors?
        let stmt_handle = DRIVER.statement_new(self.handle).map_err(to_napi_err)?;

        DRIVER
            .statement_set_sql_query(stmt_handle, query)
            .await
            .map_err(to_napi_err)?;

        let result = DRIVER
            .statement_execute_query(stmt_handle, None, None)
            .await
            .map_err(to_napi_err)?;

        let _ = DRIVER.statement_release(stmt_handle);

        self.statement_from_result(result).await
    }

    #[napi]
    pub async fn get_query_result(&self, query_id: String) -> Result<Statement> {
        let result = DRIVER
            .connection_get_query_result(self.handle, query_id)
            .await
            .map_err(to_napi_err)?;
        self.statement_from_result(result).await
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
