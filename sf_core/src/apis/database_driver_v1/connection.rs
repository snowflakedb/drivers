use snafu::ResultExt;
use std::{collections::HashMap, sync::Mutex};

use super::Handle;
use super::Setting;
use super::error::*;
use super::global_state::CONN_HANDLE_MANAGER;
use crate::config::rest_parameters::{ClientInfo, LoginParameters};
use crate::config::retry::RetryPolicy;
use crate::rest::snowflake::SessionTokens;
use crate::tls::client::create_tls_client_with_config;
use reqwest;

pub fn connection_init(conn_handle: Handle, _db_handle: Handle) -> Result<(), ApiError> {
    match CONN_HANDLE_MANAGER.get_obj(conn_handle) {
        Some(conn_ptr) => {
            // Create a blocking runtime for the login process
            let rt = tokio::runtime::Runtime::new().context(RuntimeCreationSnafu)?;

            let settings_guard = conn_ptr
                .lock()
                .map_err(|_| ConnectionLockingSnafu {}.build())?;
            let login_parameters = LoginParameters::from_settings(&settings_guard.settings)
                .context(ConfigurationSnafu)?;
            drop(settings_guard);

            let http_client =
                create_tls_client_with_config(login_parameters.client_info.tls_config.clone())
                    .context(TlsClientCreationSnafu)?;

            let tokens = rt
                .block_on(async {
                    crate::rest::snowflake::snowflake_login_with_client(
                        &http_client,
                        &login_parameters,
                    )
                    .await
                })
                .context(LoginSnafu)?;

            conn_ptr
                .lock()
                .map_err(|_| ConnectionLockingSnafu {}.build())?
                .initialize(
                    tokens,
                    http_client,
                    login_parameters.server_url.clone(),
                    login_parameters.client_info.clone(),
                );
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
    /// Session tokens for authentication and refresh
    pub tokens: Option<SessionTokens>,
    pub http_client: Option<reqwest::Client>,
    pub retry_policy: RetryPolicy,
    /// Server URL for refresh requests
    pub server_url: Option<String>,
    /// Client info for refresh requests
    pub client_info: Option<ClientInfo>,
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
            tokens: None,
            http_client: None,
            retry_policy: RetryPolicy::default(),
            server_url: None,
            client_info: None,
        }
    }

    fn initialize(
        &mut self,
        tokens: SessionTokens,
        http_client: reqwest::Client,
        server_url: String,
        client_info: ClientInfo,
    ) {
        self.tokens = Some(tokens);
        self.http_client = Some(http_client);
        self.server_url = Some(server_url);
        self.client_info = Some(client_info);
    }

    /// Get the current session token, if authenticated
    pub fn session_token(&self) -> Option<&str> {
        self.tokens.as_ref().map(|t| t.session_token.as_str())
    }
}
