use super::Handle;
use super::connection::with_valid_session;
use super::error::*;
use super::global_state::CONN_HANDLE_MANAGER;
use crate::config::rest_parameters::QueryParameters;
use crate::rest::snowflake::{QueryExecutionMode, snowflake_query_with_client};
use snafu::{OptionExt, ResultExt};

/// Get a session parameter value from the cache, with SQL fallback
pub fn connection_get_parameter(
    conn_handle: Handle,
    key: String,
) -> Result<Option<String>, ApiError> {
    match CONN_HANDLE_MANAGER.get_obj(conn_handle) {
        Some(conn_ptr) => {
            // First, check cache
            let cached_value = {
                let conn = conn_ptr
                    .lock()
                    .map_err(|_| ConnectionLockingSnafu {}.build())?;

                let cache = conn
                    .session_parameters
                    .read()
                    .map_err(|_| ConnectionLockingSnafu {}.build())?;

                // Normalize key to uppercase for case-insensitive lookup
                let normalized_key = key.to_uppercase();
                cache.get(&normalized_key).cloned()
            };

            // If found in cache, return it
            if let Some(value) = cached_value {
                return Ok(Some(value));
            }

            // Not in cache - execute SHOW PARAMETERS query
            let rt = tokio::runtime::Runtime::new().context(RuntimeCreationSnafu)?;

            let (query_parameters, http_client, retry_policy) = {
                let conn = conn_ptr
                    .lock()
                    .map_err(|_| ConnectionLockingSnafu {}.build())?;

                (
                    QueryParameters::from_settings(&conn.settings).context(ConfigurationSnafu)?,
                    conn.http_client
                        .clone()
                        .context(ConnectionNotInitializedSnafu)?,
                    conn.retry_policy.clone(),
                )
            };

            // Execute SHOW PARAMETERS LIKE query
            let sql = format!("SHOW PARAMETERS LIKE '{}' IN SESSION", key.to_uppercase());
            let conn = conn_ptr.clone();

            let response = rt.block_on(with_valid_session(&conn, |session_token| {
                let http_client = http_client.clone();
                let query_parameters = query_parameters.clone();
                let sql = sql.clone();
                let retry_policy = retry_policy.clone();
                async move {
                    snowflake_query_with_client(
                        &http_client,
                        query_parameters,
                        session_token,
                        sql,
                        None,  // No parameter bindings
                        &retry_policy,
                        QueryExecutionMode::Blocking,
                    )
                    .await
                }
            }))?;

            // Parse the response to extract the parameter value
            // SHOW PARAMETERS returns columns: key, value, default, level, description, type
            if let Some(rowset) = &response.data.rowset {
                if !rowset.is_empty() && !rowset[0].is_empty() {
                    // Find the "value" column index
                    if let Some(row_type) = &response.data.row_type {
                        if let Some(value_idx) = row_type.iter().position(|col| col.name.to_uppercase() == "VALUE") {
                            if let Some(value) = rowset[0].get(value_idx) {
                                let param_value = value.to_string();

                                // Update cache with the retrieved value
                                {
                                    let conn = conn_ptr
                                        .lock()
                                        .map_err(|_| ConnectionLockingSnafu {}.build())?;

                                    if let Ok(mut cache) = conn.session_parameters.write() {
                                        cache.insert(key.to_uppercase(), param_value.clone());
                                    }
                                }

                                return Ok(Some(param_value));
                            }
                        }
                    }
                }
            }

            // No matching parameter found
            Ok(None)
        }
        None => InvalidArgumentSnafu {
            argument: "Connection handle not found".to_string(),
        }
        .fail(),
    }
}
