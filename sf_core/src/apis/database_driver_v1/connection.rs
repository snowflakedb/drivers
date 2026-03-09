use snafu::{OptionExt, ResultExt};
use std::future::Future;
use std::{collections::HashMap, sync::Arc, sync::Mutex, sync::RwLock};
use tokio::sync::RwLock as AsyncRwLock;

use super::Handle;
use super::Setting;
use super::error::*;
use super::global_state::CONN_HANDLE_MANAGER;
use crate::config::config_manager;
use crate::config::path_resolver::ConfigPaths;
use crate::config::rest_parameters::{ClientInfo, LoginParameters};
use crate::config::retry::RetryPolicy;
use crate::rest::snowflake::{self, RestError, SessionTokens, SnowflakeResponseError};
use crate::sensitive::SensitiveString;
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
    F: Fn(SensitiveString) -> Fut,
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
    /// A token was issued but hasn't been refreshed yet. Holds the token
    /// so we can detect if another request already refreshed while we waited.
    FirstToken(SensitiveString),
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
    ) -> Result<SensitiveString, ApiError> {
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
                    if tokens.session_token.reveal() != failed_token.reveal() {
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
    /// The session token for authentication (redacted in Debug output)
    pub session_token: Option<SensitiveString>,
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
