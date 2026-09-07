use crate::DRIVER;
use crate::error::BridgeError;
use napi_derive::napi;
use sf_core::apis::database_driver_v1::Setting;
use sf_core::handle_manager::Handle;
use std::collections::HashMap;

/// The session parameters the Node.js driver reads and knows the type of. Every
/// field is resolved from the server-provided parameter set, so the parameters
/// are always present; a missing key surfaces as an error rather than a default.
#[napi(object)]
pub struct KnownSessionParameters {
    pub time_output_format: String,
    pub js_treat_integer_as_big_int: bool,
    pub client_stage_array_binding_threshold: i64,
}

impl KnownSessionParameters {
    pub(crate) async fn from_connection(conn_handle: Handle) -> Result<Self, BridgeError> {
        let params = DRIVER.connection_get_all_parameters(conn_handle).await?;
        Ok(Self {
            time_output_format: required_string(&params, "TIME_OUTPUT_FORMAT")?,
            js_treat_integer_as_big_int: required_bool(&params, "JS_TREAT_INTEGER_AS_BIGINT")?,
            client_stage_array_binding_threshold: required_int(
                &params,
                "CLIENT_STAGE_ARRAY_BINDING_THRESHOLD",
            )?,
        })
    }
}

#[cfg(test)]
impl KnownSessionParameters {
    /// Builds a value carrying Snowflake's documented defaults, for tests that
    /// need a `KnownSessionParameters` to exercise a code path that does not
    /// depend on the parameter values themselves.
    pub(crate) fn test_defaults() -> Self {
        Self {
            time_output_format: "HH24:MI:SS".to_string(),
            js_treat_integer_as_big_int: false,
            client_stage_array_binding_threshold: 0,
        }
    }
}

fn missing(key: &str) -> BridgeError {
    BridgeError::Message(format!(
        "session parameter {key} is missing or has an unexpected type"
    ))
}

fn required_string(params: &HashMap<String, Setting>, key: &str) -> Result<String, BridgeError> {
    match params.get(key) {
        Some(Setting::String(value)) => Ok(value.clone()),
        _ => Err(missing(key)),
    }
}

fn required_bool(params: &HashMap<String, Setting>, key: &str) -> Result<bool, BridgeError> {
    match params.get(key) {
        Some(Setting::Bool(value)) => Ok(*value),
        Some(Setting::String(value)) => match value.to_lowercase().as_str() {
            "true" => Ok(true),
            "false" => Ok(false),
            _ => Err(missing(key)),
        },
        _ => Err(missing(key)),
    }
}

fn required_int(params: &HashMap<String, Setting>, key: &str) -> Result<i64, BridgeError> {
    match params.get(key) {
        Some(Setting::Int(value)) => Ok(*value),
        Some(Setting::String(value)) => value.parse().map_err(|_| missing(key)),
        _ => Err(missing(key)),
    }
}
