//! Connection management for the unified driver API.

use super::error::*;
use super::global_state::CONN_HANDLE_MANAGER;
use crate::config::rest_parameters::LoginParameters;
use crate::config::settings::Setting;
use crate::handle_manager::Handle;
use crate::rest::SnowflakeRestClient;
use crate::runtime::block_on;
use snafu::ResultExt;
use std::collections::HashMap;
use std::sync::Mutex;

/// Connection state container.
pub struct Connection {
    pub settings: HashMap<String, Setting>,
    pub session_token: Option<String>,
    pub client: Option<Box<dyn SnowflakeRestClient>>,
}

impl Connection {
    pub fn new() -> Self {
        Connection {
            settings: HashMap::new(),
            session_token: None,
            client: None,
        }
    }

    pub fn initialize(&mut self, token: String, client: Box<dyn SnowflakeRestClient>) {
        self.session_token = Some(token);
        self.client = Some(client);
    }
}

impl Default for Connection {
    fn default() -> Self {
        Self::new()
    }
}

/// Create a new connection handle.
pub fn connection_new() -> Handle {
    CONN_HANDLE_MANAGER.add_handle(Mutex::new(Connection::new()))
}

/// Release a connection handle.
pub fn connection_release(conn_handle: Handle) -> Result<(), ApiError> {
    match CONN_HANDLE_MANAGER.delete_handle(conn_handle) {
        true => Ok(()),
        false => InvalidArgumentSnafu {
            argument: "Failed to release connection handle".to_string(),
        }
        .fail(),
    }
}

/// Set an option on a connection handle.
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

/// Initialize a connection handle (perform login).
pub fn connection_init(conn_handle: Handle, _db_handle: Handle) -> Result<(), ApiError> {
    match CONN_HANDLE_MANAGER.get_obj(conn_handle) {
        Some(conn_ptr) => {
            // Extract login parameters from settings
            let settings_guard = conn_ptr
                .lock()
                .map_err(|_| ConnectionLockingSnafu {}.build())?;
            let login_parameters = LoginParameters::from_settings(&settings_guard.settings)
                .context(ConfigurationSnafu)?;
            drop(settings_guard);

            // Create the REST client using the factory function
            let mut client = crate::rest::create_client(&login_parameters.server_url);

            // Perform login using the platform-appropriate block_on
            let token = block_on(client.login(&login_parameters)).context(LoginSnafu)?;

            // Store the client and token in the connection
            conn_ptr
                .lock()
                .map_err(|_| ConnectionLockingSnafu {}.build())?
                .initialize(token, client);

            Ok(())
        }
        None => InvalidArgumentSnafu {
            argument: "Connection handle not found".to_string(),
        }
        .fail(),
    }
}
