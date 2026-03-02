use snafu::{OptionExt, ResultExt};
use std::future::Future;
use std::sync::atomic::{AtomicBool, Ordering};
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

            // Parse and validate logout configuration from settings
            // This follows the same pattern as LoginParameters::from_settings
            conn.logout_config =
                LogoutConfig::from_settings(&conn.settings).context(ConfigurationSnafu)?;

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

            // Store setting - validation happens in connection_init via LogoutConfig::from_settings
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
    /// Mapping of query_id to get_result_url for async queries
    pub async_query_urls: Arc<RwLock<HashMap<String, String>>>,
    /// Flag indicating if connection has been closed
    pub is_closed: Arc<AtomicBool>,

    /// Logout configuration (set via ConnectionSetOption* before init, parsed at init time)
    pub logout_config: LogoutConfig,
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
            async_query_urls: Arc::new(RwLock::new(HashMap::new())),
            is_closed: Arc::new(AtomicBool::new(false)),
            logout_config: LogoutConfig::default(),
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

/// Data extracted from a locked connection needed to perform HTTP logout.
struct LogoutData {
    client: reqwest::Client,
    url: String,
    info: ClientInfo,
    retry_policy: RetryPolicy,
    refresh_ctx: RefreshContext,
}

/// Validate logout configuration values.
///
/// Checks for invalid timeout configurations that would cause immediate failure.
fn validate_logout_config(config: &LogoutConfig) -> Result<(), ApiError> {
    // Zero timeout means immediate failure - reject at configuration time
    if let Some(timeout) = config.logout_request_timeout
        && timeout.is_zero()
    {
        return Err(InvalidArgumentSnafu {
            argument:
                "logout_request_timeout: 0s. Zero timeout means immediate failure. Must be positive."
                    .to_string(),
        }
        .build());
    }
    Ok(())
}

/// Atomically mark connection as closed.
///
/// Returns true if the connection was already closed (duplicate close attempt).
/// This provides idempotent close() behavior - multiple calls are safe.
fn mark_connection_closed(conn_ptr: &Arc<Mutex<Connection>>) -> Result<bool, ApiError> {
    let conn = conn_ptr
        .lock()
        .map_err(|_| ConnectionLockingSnafu {}.build())?;

    // Atomic swap returns the previous value
    Ok(conn.is_closed.swap(true, Ordering::SeqCst))
}

/// Extract logout data from a locked connection and determine whether to send logout.
///
/// Precondition: Connection must not be marked as closed yet (caller should check first).
fn prepare_logout(
    conn_ptr: &Arc<Mutex<Connection>>,
    config: &LogoutConfig,
) -> Result<(bool, Option<String>, Option<LogoutData>), ApiError> {
    // Validate config first
    validate_logout_config(config)?;
    let conn = conn_ptr
        .lock()
        .map_err(|_| ConnectionLockingSnafu {}.build())?;

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
                if let Some(max_attempts) = config.max_attempts {
                    retry_policy.max_attempts = max_attempts;
                }
                retry_policy.max_elapsed = config.logout_total_timeout;
                retry_policy.per_request_timeout = config.logout_request_timeout;

                tracing::debug!(
                    total_timeout_secs = config.logout_total_timeout.as_secs(),
                    max_attempts = retry_policy.max_attempts,
                    per_request_timeout_secs =
                        retry_policy.per_request_timeout.map(|t| t.as_secs()),
                    "Configured logout retry policy"
                );

                Some(LogoutData {
                    client,
                    url,
                    info,
                    retry_policy,
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
    // TODO: SNOW-2912513 - Flush telemetry cache
    // TODO: Implement QCC (query result cache) clearing

    Ok(())
}

/// Close a connection and optionally send logout request.
///
/// Behavior depends on `config.error_strategy`:
/// - `Strict`: surface errors to the caller (close() may fail)
/// - `BestEffort`: suppress errors, log WARN (close() always succeeds)
///
/// Close the connection using logout configuration set during initialization.
///
/// Logout behavior is determined by connection fields set via ConnectionSetOption*:
/// - `server_session_keep_alive`: Control server session lifecycle
/// - `enable_logout_auto_detection`: Enable async query detection
/// - `logout_error_strategy`: Error handling (Strict or BestEffort)
/// - `logout_total_timeout`: Total timeout budget
/// - `logout_max_attempts`: Maximum total attempts (1 = no retries, 3 = 2 retries)
/// - `logout_request_timeout`: Per-request timeout
///
/// This design matches all existing Snowflake drivers (Python, Go, JDBC, .NET, Node.js)
/// which configure logout behavior at connection initialization, not at close time.
pub fn connection_close(conn_handle: Handle) -> Result<(), ApiError> {
    let conn_ptr = CONN_HANDLE_MANAGER
        .get_obj(conn_handle)
        .context(InvalidArgumentSnafu {
            argument: "Connection handle not found".to_string(),
        })?;

    // Check if already closed (idempotent close)
    let was_already_closed = mark_connection_closed(&conn_ptr)?;
    if was_already_closed {
        tracing::debug!("Connection already closed, skipping duplicate close");
        return Ok(());
    }

    // Get logout config from connection (set during connection_init)
    let config = {
        let conn = conn_ptr
            .lock()
            .map_err(|_| ConnectionLockingSnafu {}.build())?;
        conn.logout_config.clone()
    };

    let (send_logout, skip_reason, logout_data) = prepare_logout(&conn_ptr, &config)?;

    // TODO: SNOW-2912513 - Record telemetry for logout decision

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

/// Query status information returned from async query status checks.
#[derive(Debug, Clone)]
pub struct QueryStatusInfo {
    pub query_id: String,
    pub status: QueryStatus,
    pub error_code: Option<String>,
    pub error_message: Option<String>,
}

/// Query execution status
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum QueryStatus {
    Running,
    Success,
    Failed,
}

/// Check the status of an async query (CONNECTION-level API).
///
/// This is the PRIMARY API for checking async query status. Query lifecycle
/// is independent of statement lifecycle - queries outlive statements.
///
/// This matches all legacy drivers:
/// - Python: `connection.get_query_status(query_id)`
/// - JDBC: `session.getQueryStatus(queryID)`
/// - Go: Session-level status checks
///
/// # Architecture
///
/// Query status is a CONNECTION concern, not a statement concern:
/// - Connection owns the session
/// - Session owns the queries
/// - Query can outlive statement (Fire-and-Forget pattern)
///
/// # Usage
///
/// ```ignore
/// // Submit async query (via any statement)
/// let result = statement_execute_async_non_blocking(...)?;
/// let query_id = result.query_id;
///
/// // Check status via CONNECTION (statement can be dropped)
/// let status = connection_get_query_status(conn_handle, &query_id)?;
/// ```
///
/// # Token Refresh
///
/// Automatically refreshes the session token if it expires (390112 error).
///
/// # Errors
///
/// Returns `ApiError` for invalid handles, unknown query_id, HTTP errors,
/// or token refresh failures.
pub fn connection_get_query_status(
    conn_handle: Handle,
    query_id: &str,
) -> Result<QueryStatusInfo, ApiError> {
    let conn_ptr = CONN_HANDLE_MANAGER.get_obj(conn_handle).ok_or_else(|| {
        InvalidArgumentSnafu {
            argument: "Connection handle not found".to_string(),
        }
        .build()
    })?;

    let rt = crate::async_bridge::runtime().context(RuntimeCreationSnafu)?;

    let (http_client, client_info, retry_policy, url_map) = {
        let conn = conn_ptr
            .lock()
            .map_err(|_| ConnectionLockingSnafu.build())?;
        (
            conn.http_client
                .clone()
                .context(ConnectionNotInitializedSnafu)?,
            conn.client_info
                .clone()
                .context(ConnectionNotInitializedSnafu)?,
            conn.retry_policy.clone(),
            conn.async_query_urls.clone(),
        )
    };

    // Retrieve get_result_url from the connection's mapping
    let get_result_url = {
        let url_map_guard = url_map.read().map_err(|_| ConnectionLockingSnafu.build())?;
        url_map_guard.get(query_id).cloned().ok_or_else(|| {
            InvalidArgumentSnafu {
                argument: format!(
                    "Query ID {} not found in async query URL mapping. \
                     This query may not have been submitted via execute_async() \
                     or may have already been fetched.",
                    query_id
                ),
            }
            .build()
        })?
    };

    let response = rt.block_on(async {
        let mut ctx = RefreshContext::from_arc(&conn_ptr)?;
        let mut last_error = None;
        loop {
            let session_token = ctx.refresh_token(last_error).await?;
            match crate::rest::snowflake::async_exec::get_query_status_by_id(
                &http_client,
                &client_info,
                &session_token,
                &get_result_url,
                &retry_policy,
            )
            .await
            {
                Ok(resp) => return Ok(resp),
                Err(e) => {
                    last_error = Some(crate::rest::snowflake::RestError::AsyncQuery {
                        source: e,
                        location: snafu::Location::new(file!(), line!(), 0),
                    });
                }
            }
        }
    })?;

    // Determine status from response
    let status = if response.success {
        // Check if query is still running (has get_result_url but no data)
        let has_data = response.data.rowset.is_some()
            || response.data.rowset_base64.is_some()
            || response
                .data
                .chunks
                .as_ref()
                .map(|c| !c.is_empty())
                .unwrap_or(false);
        if !has_data && response.data.get_result_url.is_some() {
            QueryStatus::Running
        } else {
            QueryStatus::Success
        }
    } else {
        // Check if query is still running (has get_result_url in failure response)
        if response.data.get_result_url.is_some() {
            QueryStatus::Running
        } else {
            QueryStatus::Failed
        }
    };

    Ok(QueryStatusInfo {
        query_id: query_id.to_string(),
        status,
        error_code: response.code,
        error_message: response.message,
    })
}

/// Fetch results for an async query, polling until completion (CONNECTION-level API).
///
/// This is the PRIMARY API for fetching async query results. Query lifecycle
/// is independent of statement lifecycle - can fetch results even after
/// statement is dropped.
///
/// # Architecture
///
/// This is a connection-level operation because:
/// - Query is tracked by connection (session)
/// - Query can outlive statement that submitted it
/// - Matches legacy driver patterns (connection-level result fetching)
///
/// # Polling Strategy
///
/// Uses exponential backoff:
/// - Initial burst: 5ms → 10ms → 20ms → 40ms (for fast queries)
/// - Exponential: Base 5ms, factor 2.0, cap 5000ms
///
/// # Registry Cleanup
///
/// After successful fetch, the query_id is:
/// - Unregistered from AsyncQueryRegistry (Fire-and-Forget tracking)
/// - Removed from URL mapping (cleanup)
///
/// # Token Refresh
///
/// Automatically refreshes the session token if it expires (390112 error).
///
/// # Errors
///
/// Returns `ApiError` for invalid handles, unknown query_id, query errors,
/// deadline exceeded, or token refresh failures.
pub fn connection_fetch_async_results(
    conn_handle: Handle,
    query_id: &str,
) -> Result<crate::rest::snowflake::query_response::Response, ApiError> {
    let conn_ptr = CONN_HANDLE_MANAGER.get_obj(conn_handle).ok_or_else(|| {
        InvalidArgumentSnafu {
            argument: "Connection handle not found".to_string(),
        }
        .build()
    })?;

    let rt = crate::async_bridge::runtime().context(RuntimeCreationSnafu)?;

    let (http_client, client_info, retry_policy, registry, url_map) = {
        let conn = conn_ptr
            .lock()
            .map_err(|_| ConnectionLockingSnafu.build())?;
        (
            conn.http_client
                .clone()
                .context(ConnectionNotInitializedSnafu)?,
            conn.client_info
                .clone()
                .context(ConnectionNotInitializedSnafu)?,
            conn.retry_policy.clone(),
            conn.async_query_registry.clone(),
            conn.async_query_urls.clone(),
        )
    };

    // Retrieve get_result_url from the connection's mapping
    let get_result_url = {
        let url_map_guard = url_map.read().map_err(|_| ConnectionLockingSnafu.build())?;
        url_map_guard.get(query_id).cloned().ok_or_else(|| {
            InvalidArgumentSnafu {
                argument: format!(
                    "Query ID {} not found in async query URL mapping. \
                     This query may not have been submitted via execute_async() \
                     or may have already been fetched.",
                    query_id
                ),
            }
            .build()
        })?
    };

    let response = rt.block_on(async {
        let mut ctx = RefreshContext::from_arc(&conn_ptr)?;
        let mut last_error = None;
        loop {
            let session_token = ctx.refresh_token(last_error).await?;
            match crate::rest::snowflake::async_exec::fetch_results_by_query_id(
                &http_client,
                &client_info,
                &session_token,
                &get_result_url,
                &retry_policy,
            )
            .await
            {
                Ok(resp) => {
                    // Unregister from AsyncQueryRegistry (Fire-and-Forget tracking)
                    if let Err(e) = registry.unregister(query_id) {
                        tracing::warn!(
                            query_id,
                            error = ?e,
                            "Failed to unregister async query"
                        );
                    }

                    // Remove query_id → URL mapping after successful fetch
                    if let Ok(mut url_map_guard) = url_map.write() {
                        url_map_guard.remove(query_id);
                    } else {
                        tracing::warn!(
                            query_id,
                            "Failed to acquire write lock for URL mapping cleanup"
                        );
                    }

                    return Ok(resp);
                }
                Err(e) => {
                    last_error = Some(crate::rest::snowflake::RestError::AsyncQuery {
                        source: e,
                        location: snafu::Location::new(file!(), line!(), 0),
                    });
                }
            }
        }
    })?;

    Ok(response)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[test]
    fn test_validate_logout_config_accepts_none() {
        let config = LogoutConfig {
            logout_request_timeout: None,
            ..Default::default()
        };
        assert!(validate_logout_config(&config).is_ok());
    }

    #[test]
    fn test_validate_logout_config_accepts_positive_timeout() {
        let config = LogoutConfig {
            logout_request_timeout: Some(Duration::from_secs(10)),
            ..Default::default()
        };
        assert!(validate_logout_config(&config).is_ok());
    }

    #[test]
    fn test_validate_logout_config_rejects_zero_timeout() {
        let config = LogoutConfig {
            logout_request_timeout: Some(Duration::ZERO),
            ..Default::default()
        };
        let result = validate_logout_config(&config);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(matches!(err, ApiError::InvalidArgument { .. }));
        assert!(err.to_string().contains("Zero timeout"));
    }
}
