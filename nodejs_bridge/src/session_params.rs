use crate::DRIVER;
use napi_derive::napi;
use sf_core::apis::database_driver_v1::{ApiError, Setting};
use sf_core::handle_manager::Handle;
use std::collections::HashMap;
use std::sync::Arc;

#[napi]
pub struct SessionParameter {
    value: Setting,
}

impl From<Setting> for SessionParameter {
    fn from(value: Setting) -> Self {
        Self { value }
    }
}

// TODO: create unified way of fetching session parameters in bridge and in NodeJs
#[napi]
impl SessionParameter {
    #[napi]
    pub fn get_string(&self) -> Option<String> {
        match &self.value {
            Setting::String(value) => Some(value.clone()),
            _ => None,
        }
    }

    #[napi]
    pub fn get_bool(&self) -> Option<bool> {
        match &self.value {
            Setting::Bool(value) => Some(*value),
            _ => None,
        }
    }

    /// Integers reach JavaScript as `number`, so values beyond 2^53 lose
    /// precision. Session parameters rarely carry numbers that large.
    #[napi]
    pub fn get_int(&self) -> Option<i64> {
        match &self.value {
            Setting::Int(value) => Some(*value),
            _ => None,
        }
    }

    #[napi]
    pub fn get_double(&self) -> Option<f64> {
        match &self.value {
            Setting::Double(value) => Some(*value),
            _ => None,
        }
    }
}

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
