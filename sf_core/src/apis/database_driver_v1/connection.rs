use snafu::ResultExt;
use std::{collections::HashMap, sync::Mutex};

use super::Handle;
use super::Setting;
use super::error::*;
use super::global_state::CONN_HANDLE_MANAGER;
use crate::config::rest_parameters::LoginParameters;

pub fn connection_init(conn_handle: Handle, _db_handle: Handle) -> Result<(), ApiError> {
    match CONN_HANDLE_MANAGER.get_obj(conn_handle) {
        Some(conn_ptr) => {
            // Create a blocking runtime for the login process
            let rt = tokio::runtime::Runtime::new().context(RuntimeCreationSnafu)?;

            let login_parameters =
                LoginParameters::from_settings(&conn_ptr.lock().unwrap().settings)
                    .context(ConfigurationSnafu)?;

            let login_result = rt
                .block_on(async {
                    crate::rest::snowflake::snowflake_login(&login_parameters).await
                })
                .context(LoginSnafu)?;

            conn_ptr
                .lock()
                .map_err(|_| ConnectionLockingSnafu {}.build())?
                .session_token = Some(login_result);
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

pub struct Connection {
    pub settings: HashMap<String, Setting>,
    pub session_token: Option<String>,
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
        }
    }
}
