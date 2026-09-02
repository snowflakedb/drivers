use crate::DRIVER;
use sf_core::apis::database_driver_v1::{ApiError, Setting};
use sf_core::handle_manager::Handle;
use std::collections::HashMap;
use std::sync::Arc;

/// Snowflake's documented default for `TIME_OUTPUT_FORMAT` when the session
/// parameter is unset.
const DEFAULT_TIME_OUTPUT_FORMAT: &str = "HH24:MI:SS";

#[derive(Debug, Clone)]
pub(crate) struct SessionParams {
    pub(crate) time_format: Arc<str>,
}

impl SessionParams {
    pub(crate) async fn from_connection(conn_handle: Handle) -> Result<Self, ApiError> {
        let params = DRIVER.connection_get_all_parameters(conn_handle).await?;
        Ok(Self {
            time_format: get_uppercase_or_default(
                &params,
                "TIME_OUTPUT_FORMAT",
                DEFAULT_TIME_OUTPUT_FORMAT,
            ),
        })
    }
}

/// Upper-cased because the format renderers match tokens like `"HH24"`
/// case-sensitively, and nothing upstream normalizes value case.
fn get_uppercase_or_default(
    params: &HashMap<String, Setting>,
    key: &str,
    default: &str,
) -> Arc<str> {
    Arc::from(
        params
            .get(key)
            .and_then(|setting| match setting {
                Setting::String(value) => Some(value.as_str()),
                _ => None,
            })
            .unwrap_or(default)
            .to_uppercase(),
    )
}
