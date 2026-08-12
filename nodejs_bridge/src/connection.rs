use crate::DRIVER;
use crate::error::{ToJsError, async_to_js};
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

#[napi]
pub struct Connection {
    handle: Handle,
    database_handle: Handle,
    /// TEMPORARY: Copied onto every [`Statement`] this connection creates.\
    session_parameters: HashMap<String, String>,
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
            session_parameters,
        })
    }

    #[napi]
    pub fn connect(&self, env: &Env) -> Result<AsyncBlock<()>> {
        let handle = self.handle;
        let database_handle = self.database_handle;
        async_to_js(env, async move {
            DRIVER.connection_init(None, handle, database_handle).await
        })
    }

    #[napi]
    pub fn get_session_parameter(&self, name: String) -> Option<String> {
        self.session_parameters.get(&name).cloned()
    }

    #[napi]
    pub fn execute(&self, env: &Env, query: String) -> Result<Statement> {
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
        async_to_js(env, async move {
            let close = DRIVER.connection_close(handle).await;
            let _ = DRIVER.connection_release(handle);
            let _ = DRIVER.database_release(database_handle);
            close
        })
    }
}
