use snafu::{OptionExt, ResultExt};
use std::future::Future;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;
use std::{collections::HashMap, sync::Arc, sync::Mutex, sync::RwLock};
use tokio::sync::RwLock as AsyncRwLock;

use super::Handle;
use super::Setting;
use super::async_query_registry::AsyncQueryRegistry;
use super::error::*;
use super::global_state::CONN_HANDLE_MANAGER;
use super::logout_decision::should_send_logout;
use crate::config::config_manager;
use crate::config::logout::LogoutConfig;
use crate::config::path_resolver::ConfigPaths;
use crate::config::rest_parameters::{ClientInfo, LoginParameters};
use crate::config::retry::RetryPolicy;
use crate::rest::snowflake::logout::logout_session;
use crate::rest::snowflake::{self, RestError, SessionTokens, SnowflakeResponseError};
use crate::tls::client::create_tls_client_with_config;
use reqwest;

/// Skip reason constant for when connection is already closed
const SKIP_REASON_ALREADY_CLOSED: &str = "already_closed";

/// Load configuration from TOML files for a named connection.
///
/// Takes a mutable reference to the connection to avoid double-locking.
/// Only sets config values for keys not already present (explicit settings win).
pub fn connection_load_from_config(
    conn: &mut Connection,
    connection_name: &str,
) -> Result<(), ApiError> {
    let config_settings =
        config_manager::load_connection_config(connection_name).context(ConfigurationSnafu)?;

    for (key, value) in config_settings {
        conn.settings.entry(key).or_insert(value);
    }
    Ok(())
}

/// Load configuration from TOML files using explicit config paths.
pub fn connection_load_from_config_with_paths(
    conn: &mut Connection,
    connection_name: &str,
    paths: &ConfigPaths,
) -> Result<(), ApiError> {
    let config_settings = config_manager::load_connection_config_with_paths(connection_name, paths)
        .context(ConfigurationSnafu)?;

    for (key, value) in config_settings {
        conn.settings.entry(key).or_insert(value);
    }
    Ok(())
}

pub fn connection_init(conn_handle: Handle, _db_handle: Handle) -> Result<(), ApiError> {
    match CONN_HANDLE_MANAGER.get_obj(conn_handle) {
        Some(conn_ptr) => {
            let mut conn = conn_ptr
                .lock()
                .map_err(|_| ConnectionLockingSnafu {}.build())?;

            // Check if connection_name is set and load from config if present
            let connection_name = conn.settings.get("connection_name").and_then(|s| {
                if let Setting::String(name) = s {
                    Some(name.clone())
                } else {
                    None
                }
            });

            if let Some(name) = connection_name {
                connection_load_from_config(&mut conn, &name)?;
            }

            let rt = crate::async_bridge::runtime().context(RuntimeCreationSnafu)?;

            let login_parameters =
                LoginParameters::from_settings(&conn.settings).context(ConfigurationSnafu)?;
            let init_params = conn.init_session_parameters.clone();
            drop(conn);

            let http_client =
                create_tls_client_with_config(login_parameters.client_info.tls_config.clone())
                    .context(TlsClientCreationSnafu)?;

            let login_result = rt
                .block_on(async {
                    crate::rest::snowflake::snowflake_login_with_client(
                        &http_client,
                        &login_parameters,
                        init_params.as_ref(),
                    )
                    .await
                })
                .context(LoginSnafu)?;

            // Initialize connection with session parameters from login response.
            // The server returns system-level parameters but may not echo back
            // user-set parameters (e.g. QUERY_TAG), so we merge in the
            // init_session_parameters the caller explicitly requested.
            let mut merged_params = init_params.unwrap_or_default();
            merged_params.extend(login_result.session_parameters.unwrap_or_default());

            conn_ptr
                .lock()
                .map_err(|_| ConnectionLockingSnafu {}.build())?
                .initialize(
                    login_result.tokens,
                    http_client,
                    login_parameters.server_url.clone(),
                    login_parameters.client_info.clone(),
                    merged_params,
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

pub fn connection_set_session_parameters(
    handle: Handle,
    parameters: HashMap<String, String>,
) -> Result<(), ApiError> {
    match CONN_HANDLE_MANAGER.get_obj(handle) {
        Some(conn_ptr) => {
            let mut conn = conn_ptr
                .lock()
                .map_err(|_| ConnectionLockingSnafu {}.build())?;
            conn.init_session_parameters = Some(parameters);
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
    /// Session parameters cache (populated after login)
    pub session_parameters: Arc<RwLock<HashMap<String, String>>>,
    /// Session parameters to send during initialization (set before connection_init)
    pub init_session_parameters: Option<HashMap<String, String>>,
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
            session_parameters: Arc::new(RwLock::new(HashMap::new())),
            init_session_parameters: None,
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
        session_params: HashMap<String, String>,
    ) {
        // Use blocking_write since we're in a sync context during connection_init
        *self.tokens.blocking_write() = Some(tokens);
        self.http_client = Some(http_client);
        self.server_url = Some(server_url);
        self.client_info = Some(client_info);

        // Populate session parameters cache (assume login always returns parameters)
        if let Ok(mut cache) = self.session_parameters.write() {
            *cache = session_params;
        }
    }

    /// Update the session parameters cache after a successful query.
    pub fn update_session_params_cache(
        &self,
        query: &str,
        response_parameters: Option<
            &Vec<crate::rest::snowflake::query_response::NameValueParameter>,
        >,
    ) {
        let mut cache = match self.session_parameters.write() {
            Ok(cache) => cache,
            Err(_) => return,
        };

        // 1. ALTER SESSION SET detection: optimistically update the cache based on user's query.
        // This is necessary as Snowflake returns only part of session parameters in response.
        // Details: SNOW-3104303
        cache.extend(
            super::alter_session_parser::parse_all_alter_sessions(query)
                .into_iter()
                .map(|p| {
                    tracing::debug!(
                        param_name = %p.name,
                        param_value = %p.value,
                        "Detected ALTER SESSION SET, updating cache optimistically"
                    );
                    (p.name.clone(), p.value.clone())
                }),
        );

        // 2. Response parameters: merge any server-returned session parameters into the cache.
        if let Some(parameters) = response_parameters {
            cache.extend(
                parameters
                    .iter()
                    .map(|param| {
                        let value_str = match &param.value {
                            serde_json::Value::String(s) => s.clone(),
                            serde_json::Value::Number(n) => n.to_string(),
                            serde_json::Value::Bool(b) => b.to_string(),
                            other => {
                                tracing::debug!(
                                    param_name = %param.name,
                                    param_value = ?other,
                                    "Unexpected JSON type for session parameter, skipping"
                                );
                                return (String::new(), String::new());
                            }
                        };
                        (param.name.to_uppercase(), value_str)
                    })
                    .filter(|(k, _)| !k.is_empty()),
            );
        }
    }
}

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

/// Connection information returned by connection_get_info
#[derive(Debug, Clone)]
pub struct ConnectionInfo {
    /// The host name of the Snowflake server
    pub host: Option<String>,
    /// The port number (if explicitly configured)
    pub port: Option<i64>,
    /// The full server URL
    pub server_url: Option<String>,
    /// The session token for authentication
    pub session_token: Option<String>,
    /// The server-assigned session ID
    pub session_id: Option<i64>,
}

/// Get connection information for the given connection handle
pub fn connection_get_info(conn_handle: Handle) -> Result<ConnectionInfo, ApiError> {
    match CONN_HANDLE_MANAGER.get_obj(conn_handle) {
        Some(conn_ptr) => {
            let conn = conn_ptr
                .lock()
                .map_err(|_| ConnectionLockingSnafu {}.build())?;

            // Extract host and port from settings
            let host = conn.settings.get("host").and_then(|s| {
                if let Setting::String(v) = s {
                    Some(v.clone())
                } else {
                    None
                }
            });

            let port = conn.settings.get("port").and_then(|s| {
                if let Setting::Int(v) = s {
                    Some(*v)
                } else {
                    None
                }
            });

            // Get server_url
            let server_url = conn.server_url.clone();

            // Get session token and session ID from tokens
            let (session_token, session_id) = {
                let tokens_guard = conn.tokens.blocking_read();
                match tokens_guard.as_ref() {
                    Some(tokens) => (Some(tokens.session_token.clone()), Some(tokens.session_id)),
                    None => (None, None),
                }
            };

            Ok(ConnectionInfo {
                host,
                port,
                server_url,
                session_token,
                session_id,
            })
        }
        None => InvalidArgumentSnafu {
            argument: "Connection handle not found".to_string(),
        }
        .fail(),
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

/// Minimum per-request timeout for logout HTTP calls.
/// Prevents unreasonably short timeouts when total budget is small.
const MIN_PER_REQUEST_TIMEOUT: Duration = Duration::from_secs(5);

/// Maximum per-request timeout for logout HTTP calls.
/// Individual requests shouldn't consume the entire budget.
const MAX_PER_REQUEST_TIMEOUT: Duration = Duration::from_secs(120);

/// Data extracted from a locked connection needed to perform HTTP logout.
struct LogoutData {
    client: reqwest::Client,
    url: String,
    info: ClientInfo,
    retry_policy: RetryPolicy,
    per_request_timeout: Duration,
    refresh_ctx: RefreshContext,
}

/// Extract logout data from a locked connection, set is_closed, and determine
/// whether to send logout. Returns early `Ok(())` if already closed.
fn prepare_logout(
    conn_ptr: &Arc<Mutex<Connection>>,
    config: &LogoutConfig,
) -> Result<(bool, Option<String>, Option<LogoutData>), ApiError> {
    let conn = conn_ptr
        .lock()
        .map_err(|_| ConnectionLockingSnafu {}.build())?;

    if conn.is_closed.swap(true, Ordering::SeqCst) {
        tracing::debug!("Connection already closed, skipping duplicate close");
        // Caller checks send_logout=false + logout_data=None to detect this case,
        // but we need a way to signal "already closed". Use send_logout=false + skip_reason.
        return Ok((false, Some(SKIP_REASON_ALREADY_CLOSED.to_string()), None));
    }

    tracing::info!("Closing connection");

    let (send_logout, skip_reason) = should_send_logout(config, Some(&conn.async_query_registry));

    let logout_data = if send_logout {
        let http_client = conn.http_client.clone();
        let server_url = conn.server_url.clone();
        let client_info = conn.client_info.clone();
        let refresh_ctx_result = RefreshContext::new(&conn);

        match (http_client, server_url, client_info) {
            (Some(client), Some(url), Some(info)) => {
                let mut retry_policy = conn.retry_policy.clone();
                if let Some(max_attempts) = config.max_retry_attempts {
                    retry_policy.max_attempts = max_attempts;
                }
                retry_policy.max_elapsed = config.logout_total_timeout;

                let per_request_timeout =
                    config.logout_total_timeout / retry_policy.max_attempts.max(1);
                let per_request_timeout =
                    per_request_timeout.clamp(MIN_PER_REQUEST_TIMEOUT, MAX_PER_REQUEST_TIMEOUT);

                Some(LogoutData {
                    client,
                    url,
                    info,
                    retry_policy,
                    per_request_timeout,
                    refresh_ctx: refresh_ctx_result?,
                })
            }
            _ => None,
        }
    } else {
        None
    };

    Ok((send_logout, skip_reason, logout_data))
}

/// Send the HTTP logout request with automatic token refresh on 390112.
/// Uses the same RefreshContext loop pattern as statement.rs.
fn send_logout_request(data: LogoutData) -> Result<(), ApiError> {
    let rt = crate::async_bridge::runtime().context(RuntimeCreationSnafu)?;
    let mut ctx = data.refresh_ctx;

    let result = rt.block_on(async {
        let mut last_error: Option<RestError> = None;
        loop {
            let session_token = ctx.refresh_token(last_error).await?;
            match logout_session(
                &data.client,
                &data.url,
                &session_token,
                &data.info,
                data.per_request_timeout,
                &data.retry_policy,
            )
            .await
            {
                Ok(()) => return Ok(()),
                Err(e) => last_error = Some(e),
            }
        }
    });

    // Remap ApiError::Query (from RefreshContext) to ApiError::LogoutFailed
    result.map_err(|e| match e {
        ApiError::Query { source, .. } => LogoutFailedSnafu {
            message: format!("{source}"),
        }
        .build(),
        other => other,
    })
}

/// Clear tokens, HTTP client, and stop background tasks.
fn cleanup_connection(conn_ptr: &Arc<Mutex<Connection>>) -> Result<(), ApiError> {
    let mut conn = conn_ptr
        .lock()
        .map_err(|_| ConnectionLockingSnafu {}.build())?;

    *conn.tokens.blocking_write() = None;
    conn.http_client = None;
    tracing::debug!("Cleared session tokens and HTTP client");

    // TODO: SNOW-2881763 - Stop heartbeat thread
    tracing::debug!("Heartbeat cleanup deferred");
    // TODO: SNOW-2912513 - Flush telemetry cache
    tracing::debug!("Telemetry flush deferred");
    // TODO: Implement QCC (query result cache) clearing
    tracing::debug!("Query result cache cleanup deferred");

    Ok(())
}

/// Close a connection and optionally send logout request.
///
/// Behavior depends on `config.error_strategy`:
/// - `Strict`: surface errors to the caller (close() may fail)
/// - `BestEffort`: suppress errors, log WARN (close() always succeeds)
pub fn connection_close(conn_handle: Handle, config: LogoutConfig) -> Result<(), ApiError> {
    let conn_ptr = CONN_HANDLE_MANAGER
        .get_obj(conn_handle)
        .context(InvalidArgumentSnafu {
            argument: "Connection handle not found".to_string(),
        })?;

    let (send_logout, skip_reason, logout_data) = prepare_logout(&conn_ptr, &config)?;

    if skip_reason.as_deref() == Some(SKIP_REASON_ALREADY_CLOSED) {
        return Ok(());
    }

    // TODO: SNOW-2912513 - Record telemetry for logout decision
    tracing::debug!(
        send_logout,
        skip_reason = ?skip_reason,
        "TODO: SNOW-2912513 - Record logout decision metrics"
    );

    let logout_result = match logout_data {
        Some(data) => {
            let result = send_logout_request(data);
            if result.is_ok() {
                tracing::info!("Logout completed successfully");
            }
            result
        }
        None if !send_logout => {
            tracing::info!(?skip_reason, "Skipping logout based on configuration");
            Ok(())
        }
        None => {
            tracing::debug!("Connection was never fully initialized, skipping logout");
            Ok(())
        }
    };

    let logout_result = config.error_strategy.handle_failed_logout(logout_result);

    cleanup_connection(&conn_ptr)?;

    if logout_result.is_ok() {
        tracing::info!("Connection closed successfully");
    }
    logout_result
}
