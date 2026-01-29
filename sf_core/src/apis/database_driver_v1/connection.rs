use snafu::{OptionExt, ResultExt};
use std::future::Future;
use std::{collections::HashMap, sync::Arc, sync::Mutex};
use std::sync::atomic::{AtomicBool, Ordering};
use tokio::sync::RwLock as AsyncRwLock;

use super::Handle;
use super::Setting;
use super::async_query_registry::AsyncQueryRegistry;
use super::error::*;
use super::global_state::CONN_HANDLE_MANAGER;
use super::logout_decision::should_send_logout;
use crate::config::logout::LogoutConfig;
use crate::config::rest_parameters::{ClientInfo, LoginParameters};
use crate::config::retry::RetryPolicy;
use crate::rest::snowflake::{self, RestError, SessionTokens, SnowflakeResponseError};
use crate::rest::snowflake::logout::logout_session;
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
    /// Session tokens - RwLock allows concurrent reads, exclusive writes for refresh
    pub tokens: Arc<AsyncRwLock<Option<SessionTokens>>>,
    pub http_client: Option<reqwest::Client>,
    pub retry_policy: RetryPolicy,
    /// Server URL for refresh requests
    pub server_url: Option<String>,
    /// Client info for refresh requests
    pub client_info: Option<ClientInfo>,
    /// Registry for tracking async queries (for Fire & Forget auto-detection)
    pub async_query_registry: AsyncQueryRegistry,
    /// Flag indicating if connection has been closed
    pub is_closed: Arc<AtomicBool>,
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
            tokens: Arc::new(AsyncRwLock::new(None)),
            http_client: None,
            retry_policy: RetryPolicy::default(),
            server_url: None,
            client_info: None,
            async_query_registry: AsyncQueryRegistry::new(),
            is_closed: Arc::new(AtomicBool::new(false)),
        }
    }

    fn initialize(
        &mut self,
        tokens: SessionTokens,
        http_client: reqwest::Client,
        server_url: String,
        client_info: ClientInfo,
    ) {
        // Use blocking_write since we're in a sync context during connection_init
        *self.tokens.blocking_write() = Some(tokens);
        self.http_client = Some(http_client);
        self.server_url = Some(server_url);
        self.client_info = Some(client_info);
    }
}

/// Execute an operation with automatic session refresh on 401.
///
/// This function:
/// 1. Reads the session token (allows concurrent readers)
/// 2. Runs the provided function with that token
/// 3. On SessionExpired error, acquires write lock, refreshes, and retries
pub async fn with_valid_session<F, Fut, T>(
    conn: &Arc<Mutex<Connection>>,
    f: F,
) -> Result<T, ApiError>
where
    F: Fn(String) -> Fut,
    Fut: Future<Output = Result<T, RestError>>,
{
    // Extract connection info and get Arc to tokens RwLock
    let (tokens_lock, http_client, server_url, client_info) = {
        let guard = conn.lock().map_err(|_| ConnectionLockingSnafu.build())?;
        (
            guard.tokens.clone(),
            guard
                .http_client
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

    // Read session token - concurrent readers allowed
    let session_token = {
        let tokens_guard = tokens_lock.read().await;
        tokens_guard
            .as_ref()
            .map(|t| t.session_token.clone())
            .context(ConnectionNotInitializedSnafu)?
    };

    // First attempt - save the token we used so we can detect if it changed
    let failed_token = session_token.clone();
    match f(session_token).await {
        Ok(result) => Ok(result),
        Err(RestError::InvalidSnowflakeResponse {
            source: SnowflakeResponseError::SessionExpired { .. },
            ..
        }) => {
            tracing::info!("Session expired, attempting refresh");

            // Acquire write lock - blocks other readers/writers during refresh
            let mut tokens_guard = tokens_lock.write().await;

            let tokens = tokens_guard
                .as_ref()
                .cloned()
                .context(ConnectionNotInitializedSnafu)?;

            // If another request already refreshed while we waited, use the new token.
            // Compare actual token strings - more reliable than expiration times.
            if tokens.session_token != failed_token {
                tracing::debug!("Session already refreshed by another request");
                let token = tokens.session_token.clone();
                drop(tokens_guard); // Release write lock before async call
                return f(token).await.context(QuerySnafu);
            }

            // Check if master token is expired (can't refresh without valid master token)
            if tokens.is_master_expired() {
                tracing::error!("Master token expired, full re-authentication required");
                return MasterTokenExpiredSnafu.fail();
            }

            // Refresh session (still holding write lock to prevent concurrent refreshes)
            let new_tokens =
                snowflake::refresh_session(&http_client, &server_url, &client_info, &tokens)
                    .await
                    .context(SessionRefreshSnafu)?;

            let new_session_token = new_tokens.session_token.clone();

            // Update tokens
            *tokens_guard = Some(new_tokens);
            drop(tokens_guard); // Release write lock before retry

            tracing::info!("Session refreshed, retrying operation");

            // Retry with new token
            f(new_session_token).await.context(QuerySnafu)
        }
        Err(e) => Err(e).context(QuerySnafu),
    }
}

/// Check if a connection has been closed
///
/// This function returns the current closed state of the connection.
/// Returns true if close() has been called, false otherwise.
///
/// # Arguments
///
/// * `conn_handle` - Handle to the connection to check
///
/// # Returns
///
/// * `Ok(bool)` - true if connection is closed, false if still open
/// * `Err(ApiError)` - Invalid connection handle
pub fn connection_is_closed(conn_handle: Handle) -> Result<bool, ApiError> {
    let conn_ptr = CONN_HANDLE_MANAGER
        .get_obj(conn_handle)
        .context(InvalidArgumentSnafu {
            argument: "Connection handle not found".to_string(),
        })?;

    let conn = conn_ptr
        .lock()
        .map_err(|_| ConnectionLockingSnafu {}.build())?;

    Ok(conn.is_closed.load(Ordering::SeqCst))
}

/// Close a connection and optionally send logout request
///
/// This function implements the complete connection close logic:
/// 1. Check idempotency (early return if already closed)
/// 2. Determine logout decision based on config and async query registry
/// 3. Record telemetry (stub for now)
/// 4. Send logout if decision is "send"
/// 5. Clean up resources (tokens, HTTP client)
/// 6. Stop background tasks (heartbeat, telemetry - stubs)
///
/// # Arguments
///
/// * `conn_handle` - Handle to the connection to close
/// * `config` - Logout configuration (keep-alive, auto-detection, error strategy, timeout)
///
/// # Returns
///
/// * `Ok(())` - Connection closed successfully
/// * `Err(ApiError)` - Close failed (only in Strict strategy)
///
/// # Error Handling
///
/// Behavior depends on `config.error_strategy`:
/// - `Strict`: Only SESSION_GONE (390111) is ignored, other errors are raised
/// - `BestEffort`: All errors are logged as WARN, close() always succeeds
pub fn connection_close(conn_handle: Handle, config: LogoutConfig) -> Result<(), ApiError> {
    let conn_ptr = CONN_HANDLE_MANAGER
        .get_obj(conn_handle)
        .context(InvalidArgumentSnafu {
            argument: "Connection handle not found".to_string(),
        })?;

    let mut conn = conn_ptr
        .lock()
        .map_err(|_| ConnectionLockingSnafu {}.build())?;

    // 1. Check idempotency - early return if already closed
    if conn.is_closed.swap(true, Ordering::SeqCst) {
        tracing::debug!("Connection already closed, skipping duplicate close");
        return Ok(());
    }

    tracing::info!("Closing connection");

    // 2. Determine logout decision
    let (send_logout, skip_reason) = should_send_logout(&config, Some(&conn.async_query_registry));

    // 3. Record telemetry (stub)
    // TODO: SNOW-2912513 - Implement telemetry
    tracing::info!(
        send_logout,
        skip_reason = ?skip_reason,
        "Recording logout decision metrics (stub)"
    );

    // 4. Send logout if decision is "send"
    let logout_result = if send_logout {
        // Extract necessary data for logout
        let tokens_guard = conn.tokens.blocking_read();
        let session_token = tokens_guard.as_ref().map(|t| t.session_token.clone());
        drop(tokens_guard);

        if let (Some(token), Some(client), Some(url), Some(info)) = (
            session_token,
            conn.http_client.as_ref(),
            conn.server_url.as_ref(),
            conn.client_info.as_ref(),
        ) {
            // Create runtime for async logout
            let rt = tokio::runtime::Runtime::new().context(RuntimeCreationSnafu)?;
            
            let result = rt.block_on(async {
                logout_session(
                    client,
                    url,
                    &token,
                    info,
                    config.timeout,
                    &conn.retry_policy,
                )
                .await
            });

            // Handle logout result using Strategy pattern
            let strategy = config.error_strategy.to_handler();
            
            match result {
                Ok(()) => {
                    tracing::info!("Logout completed successfully");
                    Ok(())
                }
                Err(logout_err) => {
                    // Use Strategy pattern for error handling
                    if strategy.should_ignore_error(&logout_err) {
                        strategy.log_error(&logout_err, false);
                        Ok(())
                    } else if strategy.should_raise_error(&logout_err) {
                        strategy.log_error(&logout_err, true);
                        Err(logout_err)
                    } else {
                        strategy.log_error(&logout_err, false);
                        Ok(())
                    }
                }
            }
        } else {
            tracing::debug!("Connection was never fully initialized, skipping logout");
            Ok(())
        }
    } else {
        tracing::info!(
            ?skip_reason,
            "Skipping logout based on configuration"
        );
        Ok(())
    };

    // 5. Essential cleanup: Clear tokens and HTTP client
    // This happens regardless of logout success/failure
    {
        let mut tokens_guard = conn.tokens.blocking_write();
        *tokens_guard = None;
        tracing::debug!("Cleared session tokens");
    }
    conn.http_client = None;
    tracing::debug!("Cleared HTTP client");

    // 6. Stubbed cleanup (TODO: implement when infrastructure is ready)
    // TODO: SNOW-2881763 - Stop heartbeat
    tracing::debug!("Heartbeat stopped (stub - TODO: SNOW-2881763)");
    
    // TODO: SNOW-2912513 - Flush telemetry
    tracing::debug!("Telemetry flushed (stub - TODO: SNOW-2912513)");
    
    // TODO: SNOW-xxxx - Clear QCC
    tracing::debug!("Query result cache cleared (stub - TODO: SNOW-xxxx)");

    // 7. Handle logout result based on error strategy
    match logout_result {
        Ok(()) => {
            tracing::info!("Connection closed successfully");
            Ok(())
        }
        Err(logout_err) => {
            // Convert LogoutError to ApiError
            Err(LogoutFailedSnafu {
                message: format!("{:?}", logout_err),
            }
            .build())
        }
    }
}
