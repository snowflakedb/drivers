use snafu::ResultExt;
use std::{collections::HashMap, sync::Mutex};

use super::Handle;
use super::Setting;
use super::error::*;
use super::global_state::CONN_HANDLE_MANAGER;
use crate::config::rest_parameters::{LoginParameters, QueryParameters};
use reqwest::Client;

pub fn connection_init(conn_handle: Handle, _db_handle: Handle) -> Result<(), ApiError> {
    match CONN_HANDLE_MANAGER.get_obj(conn_handle) {
        Some(conn_ptr) => {
            // Create a blocking runtime for the login process
            let rt = tokio::runtime::Runtime::new().context(RuntimeCreationSnafu)?;

            let settings_guard = conn_ptr
                .lock()
                .map_err(|_| ConnectionLockingSnafu {}.build())?;
            let settings_snapshot = settings_guard.settings.clone();
            let login_parameters = LoginParameters::from_settings(&settings_guard.settings)
                .context(ConfigurationSnafu)?;
            let query_parameters = QueryParameters::from_settings(&settings_guard.settings)
                .context(ConfigurationSnafu)?;
            let http_client = settings_guard.http_client.clone();
            drop(settings_guard);

            let login_result = rt
                .block_on(async {
                    crate::rest::snowflake::snowflake_login_with_client(
                        &http_client,
                        &login_parameters,
                    )
                    .await
                })
                .context(LoginSnafu)?;

            {
                let mut conn_guard = conn_ptr
                    .lock()
                    .map_err(|_| ConnectionLockingSnafu {}.build())?;
                conn_guard.session_token = Some(login_result.session_token.clone());
                conn_guard.session_timezone = login_result.session_timezone.clone();
            }
            let force_json = crate::rest::snowflake::force_json_rowset();
            let session_assignments =
                build_session_parameter_assignments(&settings_snapshot, force_json);
            if let Err(err) = apply_session_parameters(
                &rt,
                &http_client,
                query_parameters,
                login_result.session_token.clone(),
                session_assignments,
            ) {
                tracing::warn!(
                    error = %err,
                    "Unable to apply requested session parameters; continuing with defaults"
                );
            }

            if let Some(warehouse_name) = get_setting_string(&settings_snapshot, "warehouse") {
                let warehouse_identifier = format_identifier(&warehouse_name);
                let statement = format!("USE WAREHOUSE {warehouse_identifier}");
                let warehouse_switch_result = QueryParameters::from_settings(&settings_snapshot)
                    .context(ConfigurationSnafu)
                    .and_then(|params| {
                        rt.block_on(crate::rest::snowflake::snowflake_query_with_client(
                            &http_client,
                            params,
                            login_result.session_token.clone(),
                            statement,
                            None,
                            None,
                            false,
                            login_result.session_timezone.clone(),
                            false,
                        ))
                        .context(QueryExecutionSnafu)
                    });

                match warehouse_switch_result {
                    Ok(_) => {
                        tracing::info!("Successfully switched to warehouse '{warehouse_name}'")
                    }
                    Err(err) => tracing::warn!(
                        error = %err,
                        "Unable to switch to requested warehouse '{warehouse_name}'"
                    ),
                }
            }
            Ok(())
        }
        None => InvalidArgumentSnafu {
            argument: "Connection handle not found".to_string(),
        }
        .fail(),
    }
}

pub fn connection_set_option(handle: Handle, key: String, value: Setting) -> Result<(), ApiError> {
    match CONN_HANDLE_MANAGER.get_obj(handle) {
        Some(conn_ptr) => {
            let mut conn = conn_ptr
                .lock()
                .map_err(|_| ConnectionLockingSnafu {}.build())?;
            tracing::debug!(
                "connection_set_option: key='{}', value='{}'",
                key,
                match &value {
                    Setting::String(v) => v.clone(),
                    Setting::Int(v) => v.to_string(),
                    Setting::Double(v) => v.to_string(),
                    Setting::Bytes(_) => "<bytes>".to_string(),
                }
            );
            conn.settings.insert(key, value);
            Ok(())
        }
        None => InvalidArgumentSnafu {
            argument: "Connection handle not found".to_string(),
        }
        .fail(),
    }
}

pub fn connection_new() -> Handle {
    CONN_HANDLE_MANAGER.add_handle(Mutex::new(Connection::new()))
}

pub fn connection_release(conn_handle: Handle) -> Result<(), ApiError> {
    match CONN_HANDLE_MANAGER.delete_handle(conn_handle) {
        true => Ok(()),
        false => InvalidArgumentSnafu {
            argument: "Failed to release connection handle".to_string(),
        }
        .fail(),
    }
}

pub fn connection_get_timezone(conn_handle: Handle) -> Result<Option<String>, ApiError> {
    match CONN_HANDLE_MANAGER.get_obj(conn_handle) {
        Some(conn_ptr) => {
            let conn = conn_ptr
                .lock()
                .map_err(|_| ConnectionLockingSnafu {}.build())?;
            Ok(conn.session_timezone.clone())
        }
        None => InvalidArgumentSnafu {
            argument: "Connection handle not found".to_string(),
        }
        .fail(),
    }
}

pub struct Connection {
    pub settings: HashMap<String, Setting>,
    pub session_token: Option<String>,
    pub session_timezone: Option<String>,
    pub http_client: Client,
    pub force_json_rowset: bool,
    pub json_rowset_override_applied: bool,
}

impl Default for Connection {
    fn default() -> Self {
        Self::new()
    }
}

impl Connection {
    pub fn new() -> Self {
        Connection {
            settings: HashMap::new(),
            session_token: None,
            session_timezone: None,
            http_client: Client::new(),
            force_json_rowset: false,
            json_rowset_override_applied: false,
        }
    }
}

fn format_identifier(value: &str) -> String {
    let needs_quoting = value.is_empty()
        || value
            .chars()
            .any(|c| !c.is_ascii_alphanumeric() && c != '_' && c != '$');
    if needs_quoting {
        let sanitized = value.replace('"', "\"\"");
        format!("\"{sanitized}\"")
    } else {
        value.to_ascii_uppercase()
    }
}

fn build_session_parameter_assignments(
    settings: &HashMap<String, Setting>,
    force_json: bool,
) -> Vec<String> {
    let mut assignments = Vec::new();

    if force_json {
        assignments.push("GO_QUERY_RESULT_FORMAT = 'JSON'".to_string());
    } else if let Some(value) = get_setting_string(settings, "go_query_result_format") {
        assignments.push(format!(
            "GO_QUERY_RESULT_FORMAT = '{}'",
            value.to_ascii_uppercase()
        ));
    } else {
        assignments.push("GO_QUERY_RESULT_FORMAT = 'ARROW'".to_string());
    }

    if let Some(value) = get_setting_int(settings, "client_prefetch_threads") {
        assignments.push(format!("CLIENT_PREFETCH_THREADS = {value}"));
    }
    if let Some(value) = get_setting_int(settings, "client_result_prefetch_threads") {
        assignments.push(format!("CLIENT_RESULT_PREFETCH_THREADS = {value}"));
    }
    if let Some(value) = get_setting_int(settings, "client_result_prefetch_slots") {
        assignments.push(format!("CLIENT_RESULT_PREFETCH_SLOTS = {value}"));
    }
    if let Some(value) = get_setting_int(settings, "client_result_chunk_size") {
        assignments.push(format!("CLIENT_RESULT_CHUNK_SIZE = {value}"));
    }
    if let Some(value) = get_setting_bool(settings, "client_session_keep_alive") {
        assignments.push(format!(
            "CLIENT_SESSION_KEEP_ALIVE = {}",
            if value { "TRUE" } else { "FALSE" }
        ));
    }
    if let Some(value) = get_setting_int(settings, "client_memory_limit") {
        assignments.push(format!("CLIENT_MEMORY_LIMIT = {value}"));
    }
    if let Some(value) = get_setting_int(settings, "client_stage_array_binding_threshold") {
        assignments.push(format!("CLIENT_STAGE_ARRAY_BINDING_THRESHOLD = {value}"));
    }
    if let Some(value) = get_setting_string(settings, "client_timestamp_type_mapping") {
        assignments.push(format!(
            "CLIENT_TIMESTAMP_TYPE_MAPPING = '{}'",
            value.to_ascii_uppercase()
        ));
    }

    // Enable multi-statement support by default
    // Setting MULTI_STATEMENT_COUNT = 0 allows multiple statements without specifying count per query
    // This can be overridden per-statement using SQL_SF_STMT_ATTR_MULTI_STATEMENT_COUNT
    assignments.push("MULTI_STATEMENT_COUNT = 0".to_string());

    assignments
}

fn apply_session_parameters(
    runtime: &tokio::runtime::Runtime,
    client: &Client,
    query_parameters: QueryParameters,
    session_token: String,
    assignments: Vec<String>,
) -> Result<(), crate::rest::snowflake::RestError> {
    if assignments.is_empty() {
        return Ok(());
    }

    let statement = format!("ALTER SESSION SET {}", assignments.join(", "));
    runtime.block_on(crate::rest::snowflake::snowflake_query_with_client(
        client,
        query_parameters,
        session_token,
        statement,
        None,
        None, // No multi-statement count for session setup
        false,
        None, // No session timezone needed for ALTER SESSION
        false,
    ))?;
    Ok(())
}

fn get_setting_string(settings: &HashMap<String, Setting>, key: &str) -> Option<String> {
    match settings.get(key)? {
        Setting::String(value) => Some(value.clone()),
        _ => None,
    }
}

fn get_setting_int(settings: &HashMap<String, Setting>, key: &str) -> Option<i64> {
    match settings.get(key)? {
        Setting::Int(value) => Some(*value),
        Setting::String(value) => value.parse::<i64>().ok(),
        _ => None,
    }
}

fn get_setting_bool(settings: &HashMap<String, Setting>, key: &str) -> Option<bool> {
    match settings.get(key)? {
        Setting::Int(value) => Some(*value != 0),
        Setting::String(value) => {
            let normalized = value.to_ascii_lowercase();
            match normalized.as_str() {
                "1" | "true" | "on" | "yes" => Some(true),
                "0" | "false" | "off" | "no" => Some(false),
                _ => None,
            }
        }
        _ => None,
    }
}
