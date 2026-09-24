use crate::DRIVER;
use crate::error::BridgeError;
use napi_derive::napi;
use sf_core::apis::database_driver_v1::Setting;
use sf_core::handle_manager::Handle;
use std::collections::HashMap;

/// Session parameters that both the Node.js layer and this bridge read for a
/// connection.
///
/// Each field holds the value the server set for the session. When that value
/// is not available yet, the field falls back to a safe default instead of
/// failing.
///
/// The value is not available in three cases:
/// - The connection was built from an existing session token and master token.
///   Core then has no session parameters until the first query succeeds and the
///   server returns them.
/// - The connection is not connected yet. Ideally we would never read session
///   parameters before connecting, and the function that needs them would fail
///   early instead. Building that guard is extra complexity we skip for now, so
///   this case falls back to a default too.
/// - The connection is terminated, so there is nothing left to read.
///   `Connection::get_session_parameters` answers with the defaults for any
///   connection core reports as unusable, leaving the operation the caller is
///   about to attempt to report the terminated connection itself.
///
/// A read that fails while the connection is usable stays an error: a missing
/// key is expected, an unreadable map is not. `result_data_from` keeps that
/// strictness too — it snapshots these values to decode a result set that
/// already arrived, where a default would mean handing back rows formatted
/// against parameters the session never had.
///
/// Ideally these defaults should not exist. Given the constraints above, we keep
/// them for now.
#[napi(object)]
#[derive(Clone)]
pub struct KnownSessionParameters {
    pub time_output_format: String,
    pub js_treat_integer_as_big_int: bool,
    pub client_stage_array_binding_threshold: i64,
}

impl KnownSessionParameters {
    pub(crate) async fn from_connection(conn_handle: Handle) -> Result<Self, BridgeError> {
        let params = DRIVER.connection_get_all_parameters(conn_handle).await?;
        Ok(Self::from_parameters(&params))
    }

    pub(crate) fn defaults() -> Self {
        Self::from_parameters(&HashMap::new())
    }

    fn from_parameters(params: &HashMap<String, Setting>) -> Self {
        Self {
            time_output_format: string_or_default(params, "TIME_OUTPUT_FORMAT", "HH24:MI:SS"),
            js_treat_integer_as_big_int: bool_or_default(
                params,
                "JS_TREAT_INTEGER_AS_BIGINT",
                false,
            ),
            client_stage_array_binding_threshold: int_or_default(
                params,
                "CLIENT_STAGE_ARRAY_BINDING_THRESHOLD",
                100_000,
            ),
        }
    }
}

fn string_or_default(params: &HashMap<String, Setting>, key: &str, default: &str) -> String {
    match params.get(key) {
        Some(Setting::String(value)) if !value.is_empty() => value.clone(),
        _ => default.to_string(),
    }
}

fn bool_or_default(params: &HashMap<String, Setting>, key: &str, default: bool) -> bool {
    match params.get(key) {
        Some(Setting::Bool(value)) => *value,
        Some(Setting::String(value)) => match value.to_lowercase().as_str() {
            "true" => true,
            "false" => false,
            _ => default,
        },
        _ => default,
    }
}

fn int_or_default(params: &HashMap<String, Setting>, key: &str, default: i64) -> i64 {
    match params.get(key) {
        Some(Setting::Int(value)) => *value,
        Some(Setting::String(value)) => value.parse().unwrap_or(default),
        _ => default,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn a_connection_without_session_parameters_answers_with_the_client_defaults() {
        let handle = DRIVER.connection_new();

        let Ok(params) = KnownSessionParameters::from_connection(handle).await else {
            panic!("fresh connection has a parameter map");
        };

        assert_eq!(params.time_output_format, "HH24:MI:SS");
        assert!(!params.js_treat_integer_as_big_int);
        assert_eq!(params.client_stage_array_binding_threshold, 100_000);

        DRIVER.connection_release(handle).unwrap();
    }

    #[tokio::test]
    async fn a_value_core_knows_wins_over_the_client_default() {
        let handle = DRIVER.connection_new();
        DRIVER
            .connection_set_options(
                handle,
                HashMap::from([
                    (
                        "TIME_OUTPUT_FORMAT".to_string(),
                        Setting::String("HH24:MI:SS.FF3".to_string()),
                    ),
                    (
                        "JS_TREAT_INTEGER_AS_BIGINT".to_string(),
                        Setting::String("true".to_string()),
                    ),
                    (
                        "CLIENT_STAGE_ARRAY_BINDING_THRESHOLD".to_string(),
                        Setting::String("64".to_string()),
                    ),
                ]),
                false,
                None,
            )
            .await
            .unwrap();

        let Ok(params) = KnownSessionParameters::from_connection(handle).await else {
            panic!("options are visible on the parameter map");
        };

        assert_eq!(params.time_output_format, "HH24:MI:SS.FF3");
        assert!(params.js_treat_integer_as_big_int);
        assert_eq!(params.client_stage_array_binding_threshold, 64);

        DRIVER.connection_release(handle).unwrap();
    }

    #[tokio::test]
    async fn a_read_for_a_handle_core_no_longer_knows_fails() {
        let handle = DRIVER.connection_new();
        DRIVER.connection_release(handle).unwrap();

        assert!(
            KnownSessionParameters::from_connection(handle)
                .await
                .is_err()
        );
    }

    #[test]
    fn the_defaults_are_the_client_values() {
        let params = KnownSessionParameters::defaults();

        assert_eq!(params.time_output_format, "HH24:MI:SS");
        assert!(!params.js_treat_integer_as_big_int);
        assert_eq!(params.client_stage_array_binding_threshold, 100_000);
    }
}
