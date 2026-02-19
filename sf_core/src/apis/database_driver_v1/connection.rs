use snafu::{OptionExt, ResultExt};
use std::future::Future;
use std::sync::atomic::{AtomicBool, Ordering};
use std::{collections::HashMap, sync::Arc, sync::Mutex};
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
use crate::rest::snowflake::logout::logout_session;
use crate::rest::snowflake::{self, RestError, SessionTokens, SnowflakeResponseError};
use crate::tls::client::create_tls_client_with_config;
use reqwest;

pub fn connection_init(conn_handle: Handle, _db_handle: Handle) -> Result<(), ApiError> {
    match CONN_HANDLE_MANAGER.get_obj(conn_handle) {
        Some(conn_ptr) => {
            let rt = crate::async_bridge::runtime().context(RuntimeCreationSnafu)?;

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
    let mut ctx = RefreshContext::from_arc(conn)?;
    let mut last_error: Option<RestError> = None;
    loop {
        let token = ctx.refresh_token(last_error).await?;
        match f(token).await {
            Ok(result) => return Ok(result),
            Err(e) => last_error = Some(e),
        }
    }
}

/// Context for automatic session token refresh.
///
/// Instead of a higher-order function pattern, `RefreshContext` gives callers
/// a loop-based API:
///
/// ```ignore
/// let mut ctx = RefreshContext::new(&conn)?;
/// let mut last_error: Option<RestError> = None;
/// loop {
///     let token = ctx.refresh_token(last_error).await?;
///     match do_something(token).await {
///         Ok(result) => return Ok(result),
///         Err(e) => last_error = Some(e),
///     }
/// }
/// ```
///
/// On first call (`last_error = None`), reads the session token (concurrent readers allowed).
/// On subsequent calls with a `SessionExpired` error, acquires write lock and refreshes.
/// On non-SessionExpired errors, propagates the error immediately.
/// Only one refresh attempt is allowed; a second SessionExpired error is propagated.
/// Tracks the state of the refresh lifecycle.
enum RefreshState {
    /// No token has been issued yet (initial call).
    Initial,
    /// A token was issued but hasn't been refreshed yet. Holds the token string
    /// so we can detect if another request already refreshed while we waited.
    FirstToken(String),
    /// A refresh has already been performed. A second SessionExpired will be propagated.
    Refreshed,
}

pub struct RefreshContext {
    tokens_lock: Arc<AsyncRwLock<Option<SessionTokens>>>,
    http_client: reqwest::Client,
    server_url: String,
    client_info: ClientInfo,
    state: RefreshState,
}

impl RefreshContext {
    pub fn from_arc(conn: &Arc<Mutex<Connection>>) -> Result<Self, ApiError> {
        let guard = conn.lock().map_err(|_| ConnectionLockingSnafu.build())?;
        Self::new(&guard)
    }
    /// Create a new `RefreshContext` by extracting connection info.
    pub fn new(conn: &Connection) -> Result<Self, ApiError> {
        Ok(Self {
            tokens_lock: conn.tokens.clone(),
            http_client: conn
                .http_client
                .clone()
                .context(ConnectionNotInitializedSnafu)?,
            server_url: conn
                .server_url
                .clone()
                .context(ConnectionNotInitializedSnafu)?,
            client_info: conn
                .client_info
                .clone()
                .context(ConnectionNotInitializedSnafu)?,
            state: RefreshState::Initial,
        })
    }

    /// Get a valid session token, optionally refreshing if the previous call failed.
    ///
    /// - `last_error = None`: reads the current session token (first call).
    /// - `last_error = Some(SessionExpired)`: refreshes the session and returns a new token.
    /// - `last_error = Some(other)`: propagates the error immediately.
    ///
    /// Only one refresh is allowed. If the refreshed token also triggers SessionExpired,
    /// the error is propagated on the next call.
    pub async fn refresh_token(
        &mut self,
        last_error: Option<RestError>,
    ) -> Result<String, ApiError> {
        match &self.state {
            // No token issued yet - read the current session token
            RefreshState::Initial => {
                let tokens_guard = self.tokens_lock.read().await;
                let token = tokens_guard
                    .as_ref()
                    .map(|t| t.session_token.clone())
                    .context(ConnectionNotInitializedSnafu)?;
                self.state = RefreshState::FirstToken(token.clone());
                Ok(token)
            }

            // First token was issued - check if it failed with SessionExpired
            RefreshState::FirstToken(failed_token) => match last_error {
                Some(RestError::InvalidSnowflakeResponse {
                    source: SnowflakeResponseError::SessionExpired { .. },
                    ..
                }) => {
                    tracing::info!("Session expired, attempting refresh");
                    let failed_token = failed_token.clone();
                    self.state = RefreshState::Refreshed;

                    // Acquire write lock - blocks other readers/writers during refresh
                    let mut tokens_guard = self.tokens_lock.write().await;

                    let tokens = tokens_guard
                        .as_ref()
                        .cloned()
                        .context(ConnectionNotInitializedSnafu)?;

                    // If another request already refreshed while we waited, use the new token.
                    if tokens.session_token != failed_token {
                        tracing::debug!("Session already refreshed by another request");
                        return Ok(tokens.session_token.clone());
                    }

                    // Check if master token is expired
                    if tokens.is_master_expired() {
                        tracing::error!("Master token expired, full re-authentication required");
                        return MasterTokenExpiredSnafu.fail();
                    }

                    // Refresh session (still holding write lock to prevent concurrent refreshes)
                    let new_tokens = snowflake::refresh_session(
                        &self.http_client,
                        &self.server_url,
                        &self.client_info,
                        &tokens,
                    )
                    .await
                    .context(SessionRefreshSnafu)?;

                    let new_session_token = new_tokens.session_token.clone();

                    // Update tokens
                    *tokens_guard = Some(new_tokens);
                    drop(tokens_guard);

                    tracing::info!("Session refreshed, retrying operation");

                    Ok(new_session_token)
                }
                Some(other) => Err(other).context(QuerySnafu),
                None => InvalidRefreshStateSnafu {
                    message: "refresh_token called with None after FirstToken".to_string(),
                }
                .fail(),
            },

            // Already refreshed once - propagate any error
            RefreshState::Refreshed => match last_error {
                Some(err) => Err(err).context(QuerySnafu),
                None => InvalidRefreshStateSnafu {
                    message: "refresh_token called with None after Refreshed".to_string(),
                }
                .fail(),
            },
        }
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
/// 3. Record telemetry (TODO: SNOW-2912513)
/// 4. Send logout if decision is "send"
/// 5. Clean up resources (tokens, HTTP client)
/// 6. Stop background tasks (TODO: SNOW-2881763 heartbeat, SNOW-2912513 telemetry, QCC clearing)
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

    // ======== Phase 1: Extract data under lock ========
    // Hold lock only long enough to set is_closed, determine logout decision,
    // and clone out the data needed for the HTTP logout call.
    let (send_logout, skip_reason, logout_data) = {
        let conn = conn_ptr
            .lock()
            .map_err(|_| ConnectionLockingSnafu {}.build())?;

        // 1. Check idempotency - early return if already closed
        if conn.is_closed.swap(true, Ordering::SeqCst) {
            tracing::debug!("Connection already closed, skipping duplicate close");
            return Ok(());
        }

        tracing::info!("Closing connection");

        // 2. Determine logout decision
        let (send_logout, skip_reason) =
            should_send_logout(&config, Some(&conn.async_query_registry));

        // 3. Extract data needed for logout (clone out of locked state)
        let logout_data = if send_logout {
            let tokens_guard = conn.tokens.blocking_read();
            let session_token = tokens_guard.as_ref().map(|t| t.session_token.clone());
            drop(tokens_guard);

            match (
                session_token,
                conn.http_client.clone(),
                conn.server_url.clone(),
                conn.client_info.clone(),
            ) {
                (Some(token), Some(client), Some(url), Some(info)) => {
                    // Build retry policy matching user's configuration
                    let mut logout_retry_policy = conn.retry_policy.clone();
                    if let Some(max_attempts) = config.max_retry_attempts {
                        logout_retry_policy.max_attempts = max_attempts;
                    }
                    // Total budget = timeout × max_attempts
                    logout_retry_policy.max_elapsed =
                        config.timeout * logout_retry_policy.max_attempts;

                    Some((token, client, url, info, logout_retry_policy))
                }
                _ => None,
            }
        } else {
            None
        };

        (send_logout, skip_reason, logout_data)
    }; // Lock released here — HTTP logout runs without holding the mutex

    // TODO: SNOW-2912513 - Record telemetry for logout decision
    tracing::debug!(
        send_logout,
        skip_reason = ?skip_reason,
        "TODO: SNOW-2912513 - Record logout decision metrics"
    );

    // ======== Phase 2: HTTP logout without holding lock ========
    let logout_result = if let Some((token, client, url, info, logout_retry_policy)) = logout_data {
        // Use shared runtime for async logout (same pattern as connection_init)
        let rt = crate::async_bridge::runtime().context(RuntimeCreationSnafu)?;

        let result = rt.block_on(async {
            logout_session(
                &client,
                &url,
                &token,
                &info,
                config.timeout,
                &logout_retry_policy,
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
    } else if !send_logout {
        tracing::info!(?skip_reason, "Skipping logout based on configuration");
        Ok(())
    } else {
        tracing::debug!("Connection was never fully initialized, skipping logout");
        Ok(())
    };

    // ======== Phase 3: Cleanup under lock ========
    {
        let mut conn = conn_ptr
            .lock()
            .map_err(|_| ConnectionLockingSnafu {}.build())?;

        // Clear tokens and HTTP client regardless of logout success/failure
        {
            let mut tokens_guard = conn.tokens.blocking_write();
            *tokens_guard = None;
            tracing::debug!("Cleared session tokens");
        }
        conn.http_client = None;
        tracing::debug!("Cleared HTTP client");
    }

    // TODO: SNOW-2881763 - Stop heartbeat thread
    tracing::debug!("TODO: SNOW-2881763 - Stop heartbeat");
    // TODO: SNOW-2912513 - Flush telemetry cache
    tracing::debug!("TODO: SNOW-2912513 - Flush telemetry");
    // TODO: Implement QCC (query result cache) clearing
    tracing::debug!("TODO: Clear query result cache");

    // Return logout result
    match logout_result {
        Ok(()) => {
            tracing::info!("Connection closed successfully");
            Ok(())
        }
        Err(logout_err) => Err(LogoutFailedSnafu {
            message: format!("{:?}", logout_err),
        }
        .build()),
    }
}
