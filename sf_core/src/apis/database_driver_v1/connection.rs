use snafu::{OptionExt, ResultExt};
use std::future::Future;
use std::{collections::HashMap, sync::Arc, sync::Mutex};
use tokio::sync::Mutex as AsyncMutex;

use super::Handle;
use super::Setting;
use super::error::*;
use super::global_state::CONN_HANDLE_MANAGER;
use crate::config::rest_parameters::{ClientInfo, LoginParameters};
use crate::config::retry::RetryPolicy;
use crate::rest::snowflake::{self, RestError, SessionTokens, SnowflakeResponseError};
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
    /// Lock to prevent concurrent refresh attempts
    refresh_lock: Arc<AsyncMutex<()>>,
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
            refresh_lock: Arc::new(AsyncMutex::new(())),
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

/// Execute an operation with automatic session refresh on 401.
///
/// This function:
/// 1. Extracts the session token from the connection
/// 2. Runs the provided function with that token
/// 3. On SessionExpired error, refreshes the session and retries once
/// 4. Uses a lock to prevent concurrent refresh attempts
pub async fn with_valid_session<F, Fut, T>(
    conn: &Arc<Mutex<Connection>>,
    f: F,
) -> Result<T, ApiError>
where
    F: Fn(String) -> Fut,
    Fut: Future<Output = Result<T, RestError>>,
{
    // Extract what we need from the connection
    let (session_token, refresh_lock, http_client) = {
        let guard = conn.lock().map_err(|_| ConnectionLockingSnafu.build())?;
        (
            guard
                .session_token()
                .map(|s| s.to_string())
                .context(ConnectionNotInitializedSnafu)?,
            guard.refresh_lock.clone(),
            guard
                .http_client
                .clone()
                .context(ConnectionNotInitializedSnafu)?,
        )
    };

    // First attempt
    match f(session_token).await {
        Ok(result) => Ok(result),
        Err(RestError::InvalidSnowflakeResponse {
            source: SnowflakeResponseError::SessionExpired { .. },
            ..
        }) => {
            tracing::info!("Session expired, attempting refresh");

            // Acquire refresh lock - only one refresh at a time
            let _lock_guard = refresh_lock.lock().await;

            // Re-check tokens after acquiring lock (another request may have refreshed)
            let (tokens, server_url, client_info) = {
                let guard = conn.lock().map_err(|_| ConnectionLockingSnafu.build())?;
                (
                    guard
                        .tokens
                        .clone()
                        .context(ConnectionNotInitializedSnafu)?,
                    guard
                        .server_url
                        .clone()
                        .context(ConnectionNotInitializedSnafu)?,
                    guard
                        .client_info
                        .clone()
                        .context(ConnectionNotInitializedSnafu)?,
                )
            };

            // If another request already refreshed, use the new token
            if !tokens.is_session_expired() {
                tracing::debug!("Session already refreshed by another request");
                return f(tokens.session_token).await.context(QuerySnafu);
            }

            // Check if master token is expired
            if tokens.is_master_expired() {
                tracing::error!("Master token expired, full re-authentication required");
                return MasterTokenExpiredSnafu.fail();
            }

            // Refresh session
            let new_tokens =
                snowflake::refresh_session(&http_client, &server_url, &client_info, &tokens)
                    .await
                    .context(SessionRefreshSnafu)?;

            let new_session_token = new_tokens.session_token.clone();

            // Update connection with new tokens
            {
                let mut guard = conn.lock().map_err(|_| ConnectionLockingSnafu.build())?;
                guard.tokens = Some(new_tokens);
            }

            tracing::info!("Session refreshed, retrying operation");

            // Retry with new token
            f(new_session_token).await.context(QuerySnafu)
        }
        Err(e) => Err(e).context(QuerySnafu),
    }
}
