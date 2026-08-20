use crate::DRIVER;
use sf_core::apis::database_driver_v1::ApiError;
use sf_core::handle_manager::Handle;
use snafu::location;
use std::collections::HashMap;
use std::sync::Arc;

#[derive(Debug, Clone)]
pub(crate) struct SessionParams {
    #[allow(dead_code)]
    // consumed by the TIME decoder, not yet implemented at this point in the stack
    pub(crate) time_format: Arc<str>,
}

impl SessionParams {
    pub(crate) async fn from_connection(conn_handle: Handle) -> Result<Self, ApiError> {
        let params = DRIVER.connection_get_all_parameters(conn_handle).await?;
        Ok(Self {
            time_format: get_uppercase(&params, "TIME_OUTPUT_FORMAT")?,
        })
    }
}

fn get(params: &HashMap<String, String>, key: &str) -> Result<Arc<str>, ApiError> {
    match params.get(key) {
        Some(value) => Ok(Arc::from(value.as_str())),
        None => Err(ApiError::InvalidArgument {
            argument: format!(
                "session parameter {key} missing from connection_get_all_parameters response"
            ),
            location: location!(),
        }),
    }
}

/// Upper-cased because the format renderers match tokens like `"HH24"`
/// case-sensitively, and nothing upstream normalizes value case.
fn get_uppercase(params: &HashMap<String, String>, key: &str) -> Result<Arc<str>, ApiError> {
    Ok(Arc::from(get(params, key)?.to_uppercase()))
}
