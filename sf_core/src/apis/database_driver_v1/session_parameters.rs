use super::Handle;
use super::error::*;
use super::global_state::CONN_HANDLE_MANAGER;

/// Get a session parameter value from the cache
pub fn connection_get_parameter(
    conn_handle: Handle,
    key: String,
) -> Result<Option<String>, ApiError> {
    match CONN_HANDLE_MANAGER.get_obj(conn_handle) {
        Some(conn_ptr) => {
            // Check cache only - no SQL fallback
            let conn = conn_ptr
                .lock()
                .map_err(|_| ConnectionLockingSnafu {}.build())?;

            let cache = conn
                .session_parameters
                .read()
                .map_err(|_| ConnectionLockingSnafu {}.build())?;

            // Normalize key to uppercase for case-insensitive lookup
            let normalized_key = key.to_uppercase();
            Ok(cache.get(&normalized_key).cloned())
        }
        None => InvalidArgumentSnafu {
            argument: "Connection handle not found".to_string(),
        }
        .fail(),
    }
}
