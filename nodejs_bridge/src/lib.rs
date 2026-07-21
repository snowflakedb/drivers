mod error;

use crate::error::*;
use napi::bindgen_prelude::*;
use napi_derive::napi;
use sf_core::apis::database_driver_v1::connection::WrapperIdentity;
use sf_core::apis::database_driver_v1::{ApiError, DatabaseDriverV1};
use sf_core::config::settings::Setting;
use sf_core::handle_manager::Handle;
use std::collections::HashMap;
use std::sync::LazyLock;

static DRIVER: LazyLock<DatabaseDriverV1> = LazyLock::new(|| {
    // TODO:
    // Implement proper bidirectional logger with configurable level,  as is done by other driver wrappers.
    let _ = tracing_subscriber::fmt()
        .with_writer(std::io::stderr)
        .with_max_level(tracing::Level::DEBUG)
        .try_init();
    DatabaseDriverV1::new()
});
static RUNTIME: LazyLock<tokio::runtime::Runtime> =
    LazyLock::new(|| tokio::runtime::Runtime::new().expect("failed to build tokio runtime"));

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

        RUNTIME
            .block_on(async {
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
}
