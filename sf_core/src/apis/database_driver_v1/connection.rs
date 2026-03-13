use snafu::{OptionExt, ResultExt};
use std::future::Future;
use std::{collections::HashMap, sync::Arc, sync::Mutex, sync::RwLock};
use tokio::sync::RwLock as AsyncRwLock;

use super::Setting;
use super::error::*;
use super::global_state::DatabaseDriverV1;
use super::validation::{ValidationIssue, resolve_and_apply_options};
use crate::config::ParamStore;
use crate::config::config_manager;
use crate::config::param_registry::{ParamKey, param_names};
use crate::config::path_resolver::ConfigPaths;
use crate::config::rest_parameters::{ClientInfo, LoginParameters};
use crate::config::retry::RetryPolicy;
use crate::handle_manager::Handle;
use crate::rest::snowflake::{self, RestError, SessionTokens, SnowflakeResponseError};
use crate::sensitive::SensitiveString;
use crate::tls::client::create_tls_client_with_config;

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
        conn.settings.insert_if_absent(key, value);
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
        conn.settings.insert_if_absent(key, value);
    }
    Ok(())
}

impl DatabaseDriverV1 {
    pub fn connection_init(&self, conn_handle: Handle, _db_handle: Handle) -> Result<(), ApiError> {
        match self.connections.get_obj(conn_handle) {
            Some(conn_ptr) => {
                let mut conn = conn_ptr
                    .lock()
                    .map_err(|_| ConnectionLockingSnafu {}.build())?;

                // Check if connection_name is set and load from config if present
                let connection_name =
                    conn.settings
                        .get(param_names::CONNECTION_NAME)
                        .and_then(|s| {
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

                let login_final_names = FinalSessionNames {
                    database: login_result.database_name,
                    schema: login_result.schema_name,
                    warehouse: login_result.warehouse_name,
                    role: login_result.role_name,
                };

                conn_ptr
                    .lock()
                    .map_err(|_| ConnectionLockingSnafu {}.build())?
                    .initialize(
                        login_result.tokens,
                        http_client,
                        login_parameters.server_url.clone(),
                        login_parameters.client_info.clone(),
                        merged_params,
                        login_final_names,
                    );
                Ok(())
            }
            None => InvalidArgumentSnafu {
                argument: "Connection handle not found".to_string(),
            }
            .fail(),
        }
    }

    pub fn connection_set_option(
        &self,
        handle: Handle,
        key: String,
        value: Setting,
    ) -> Result<(), ApiError> {
        match self.connections.get_obj(handle) {
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

    pub fn connection_set_options(
        &self,
        handle: Handle,
        options: HashMap<String, Setting>,
    ) -> Result<Vec<ValidationIssue>, ApiError> {
        match self.connections.get_obj(handle) {
            Some(conn_ptr) => {
                let mut conn = conn_ptr
                    .lock()
                    .map_err(|_| ConnectionLockingSnafu {}.build())?;
                resolve_and_apply_options(&mut conn.settings, options)
            }
            None => InvalidArgumentSnafu {
                argument: "Connection handle not found".to_string(),
            }
            .fail(),
        }
    }

    pub fn connection_set_session_parameters(
        &self,
        handle: Handle,
        parameters: HashMap<String, String>,
    ) -> Result<(), ApiError> {
        match self.connections.get_obj(handle) {
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

    pub fn connection_new(&self) -> Handle {
        self.connections.add_handle(Mutex::new(Connection::new()))
    }

    pub fn connection_release(&self, conn_handle: Handle) -> Result<(), ApiError> {
        match self.connections.delete_handle(conn_handle) {
            true => Ok(()),
            false => InvalidArgumentSnafu {
                argument: "Failed to release connection handle".to_string(),
            }
            .fail(),
        }
    }
}

pub struct Connection {
    pub(crate) settings: ParamStore,
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
    /// Server-echoed final names from login and query responses (e.g. after USE DATABASE).
    /// Stored separately from session_parameters to keep concerns distinct.
    pub final_session_names: RwLock<FinalSessionNames>,
}

impl Default for Connection {
    fn default() -> Self {
        Self::new()
    }
}

impl Connection {
    pub fn new() -> Self {
        Connection {
            settings: ParamStore::new(),
            tokens: Arc::new(AsyncRwLock::new(None)),
            http_client: None,
            retry_policy: RetryPolicy::default(),
            server_url: None,
            client_info: None,
            session_parameters: Arc::new(RwLock::new(HashMap::new())),
            init_session_parameters: None,
            final_session_names: RwLock::new(FinalSessionNames::default()),
        }
    }

    /// Convenience setter for tests and direct call sites.
    pub fn set_option(&mut self, key: String, value: Setting) {
        self.settings.insert(key, value);
    }

    pub(crate) fn initialize(
        &mut self,
        tokens: SessionTokens,
        http_client: reqwest::Client,
        server_url: String,
        client_info: ClientInfo,
        session_params: HashMap<String, String>,
        final_names: FinalSessionNames,
    ) {
        // Use blocking_write since we're in a sync context during connection_init
        *self.tokens.blocking_write() = Some(tokens);
        self.http_client = Some(http_client);
        self.server_url = Some(server_url);
        self.client_info = Some(client_info);

        if let Ok(mut cache) = self.session_parameters.write() {
            *cache = session_params;
        }

        if let Ok(mut names) = self.final_session_names.write() {
            *names = final_names;
        }
    }

    /// Update the session parameters cache after a successful query.
    pub fn update_session_params_cache(
        &self,
        query: &str,
        response_parameters: Option<
            &Vec<crate::rest::snowflake::query_response::NameValueParameter>,
        >,
        final_names: &FinalSessionNames,
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

        // 3. Server-echoed final names are stored separately in `final_session_names`
        //    so that conn.database etc. reflect changes from USE DATABASE, USE SCHEMA, etc.
        match self.final_session_names.write() {
            Ok(mut names) => {
                if final_names.database.is_some() {
                    names.database = final_names.database.clone();
                }
                if final_names.schema.is_some() {
                    names.schema = final_names.schema.clone();
                }
                if final_names.warehouse.is_some() {
                    names.warehouse = final_names.warehouse.clone();
                }
                if final_names.role.is_some() {
                    names.role = final_names.role.clone();
                }
            }
            Err(e) => {
                tracing::warn!(
                    error = %e,
                    "Failed to acquire write lock for final_session_names"
                );
            }
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

/// Server-echoed final names from query responses (e.g. after USE DATABASE).
#[derive(Debug, Clone, Default)]
pub struct FinalSessionNames {
    pub database: Option<String>,
    pub schema: Option<String>,
    pub warehouse: Option<String>,
    pub role: Option<String>,
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
    /// The Snowflake account name
    pub account: Option<String>,
    /// The authenticated user name
    pub user: Option<String>,
    /// The current role
    pub role: Option<String>,
    /// The current database
    pub database: Option<String>,
    /// The current schema
    pub schema: Option<String>,
    /// The current warehouse
    pub warehouse: Option<String>,
}

fn get_setting_string(conn: &Connection, key: ParamKey) -> Option<String> {
    conn.settings.get(key).and_then(|s| {
        if let Setting::String(v) = s {
            Some(v.clone())
        } else {
            None
        }
    })
}

/// Read a session-level value: check the session parameter cache first (server
/// may have changed it via USE DATABASE / USE ROLE / etc.), then fall back to
/// the original connection setting.
fn get_session_or_setting(
    conn: &Connection,
    param_name: &str,
    setting_key: ParamKey,
) -> Option<String> {
    if let Ok(cache) = conn.session_parameters.read()
        && let Some(v) = cache.get(param_name)
        && !v.is_empty()
    {
        return Some(v.clone());
    }
    get_setting_string(conn, setting_key)
}

impl DatabaseDriverV1 {
    /// Get connection information for the given connection handle
    pub fn connection_get_info(&self, conn_handle: Handle) -> Result<ConnectionInfo, ApiError> {
        match self.connections.get_obj(conn_handle) {
            Some(conn_ptr) => {
                let conn = conn_ptr
                    .lock()
                    .map_err(|_| ConnectionLockingSnafu {}.build())?;

                let host = get_setting_string(&conn, param_names::HOST);

                let port = conn.settings.get(param_names::PORT).and_then(|s| {
                    if let Setting::Int(v) = s {
                        Some(*v)
                    } else {
                        None
                    }
                });

                let server_url = conn.server_url.clone();

                let (session_token, session_id) = {
                    let tokens_guard = conn.tokens.blocking_read();
                    match tokens_guard.as_ref() {
                        Some(tokens) => {
                            (Some(tokens.session_token.clone()), Some(tokens.session_id))
                        }
                        None => (None, None),
                    }
                };

                let account = get_setting_string(&conn, param_names::ACCOUNT);
                let user = get_setting_string(&conn, param_names::USER);

                // Resolution order for session-scoped values:
                //   1. final_session_names  (server-echoed via login sessionInfo or query finalXxxName)
                //   2. session_parameters   (server-returned session params, only pre-login / missing sessionInfo)
                //   3. connection settings   (original kwargs supplied by the caller)
                // After a successful login, final_session_names is always populated, so the
                // or_else branch only fires before connection_init or when the server omits
                // the field from sessionInfo.
                let final_names = conn
                    .final_session_names
                    .read()
                    .map_err(|_| ConnectionLockingSnafu {}.build())?;
                let role = final_names
                    .role
                    .clone()
                    .or_else(|| get_session_or_setting(&conn, "ROLE", param_names::ROLE));
                let database = final_names
                    .database
                    .clone()
                    .or_else(|| get_session_or_setting(&conn, "DATABASE", param_names::DATABASE));
                let schema = final_names
                    .schema
                    .clone()
                    .or_else(|| get_session_or_setting(&conn, "SCHEMA", param_names::SCHEMA));
                let warehouse = final_names
                    .warehouse
                    .clone()
                    .or_else(|| get_session_or_setting(&conn, "WAREHOUSE", param_names::WAREHOUSE));
                drop(final_names);

                Ok(ConnectionInfo {
                    host,
                    port,
                    server_url,
                    session_token,
                    session_id,
                    account,
                    user,
                    role,
                    database,
                    schema,
                    warehouse,
                })
            }
            None => InvalidArgumentSnafu {
                argument: "Connection handle not found".to_string(),
            }
            .fail(),
        }
    }

    pub fn connection_get_parameter(
        &self,
        conn_handle: Handle,
        key: String,
    ) -> Result<Option<String>, ApiError> {
        match self.connections.get_obj(conn_handle) {
            Some(conn_ptr) => {
                let conn = conn_ptr
                    .lock()
                    .map_err(|_| ConnectionLockingSnafu {}.build())?;

                let cache = conn
                    .session_parameters
                    .read()
                    .map_err(|_| ConnectionLockingSnafu {}.build())?;

                let normalized_key = key.to_uppercase();
                Ok(cache.get(&normalized_key).cloned())
            }
            None => InvalidArgumentSnafu {
                argument: "Connection handle not found".to_string(),
            }
            .fail(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::apis::database_driver_v1::driver_state;
    use crate::config::param_registry::param_names;

    fn make_connection_with_settings(settings: Vec<(&str, Setting)>) -> Connection {
        let mut conn = Connection::new();
        for (key, value) in settings {
            conn.settings.insert(key.to_string(), value);
        }
        conn
    }

    #[test]
    fn get_setting_string_returns_value() {
        let conn =
            make_connection_with_settings(vec![("host", Setting::String("example.com".into()))]);
        assert_eq!(
            get_setting_string(&conn, param_names::HOST),
            Some("example.com".into())
        );
    }

    #[test]
    fn get_setting_string_returns_none_for_missing_key() {
        let conn = Connection::new();
        assert_eq!(get_setting_string(&conn, param_names::HOST), None);
    }

    #[test]
    fn get_setting_string_returns_none_for_non_string_type() {
        let conn = make_connection_with_settings(vec![("port", Setting::Int(443))]);
        assert_eq!(get_setting_string(&conn, param_names::PORT), None);
    }

    #[test]
    fn get_session_or_setting_prefers_session_parameter() {
        let conn =
            make_connection_with_settings(vec![("database", Setting::String("setting_db".into()))]);
        conn.session_parameters
            .write()
            .unwrap()
            .insert("DATABASE".into(), "session_db".into());

        assert_eq!(
            get_session_or_setting(&conn, "DATABASE", param_names::DATABASE),
            Some("session_db".into())
        );
    }

    #[test]
    fn get_session_or_setting_falls_back_to_setting() {
        let conn =
            make_connection_with_settings(vec![("database", Setting::String("setting_db".into()))]);

        assert_eq!(
            get_session_or_setting(&conn, "DATABASE", param_names::DATABASE),
            Some("setting_db".into())
        );
    }

    #[test]
    fn get_session_or_setting_ignores_empty_session_param() {
        let conn =
            make_connection_with_settings(vec![("role", Setting::String("setting_role".into()))]);
        conn.session_parameters
            .write()
            .unwrap()
            .insert("ROLE".into(), String::new());

        assert_eq!(
            get_session_or_setting(&conn, "ROLE", param_names::ROLE),
            Some("setting_role".into())
        );
    }

    #[test]
    fn get_session_or_setting_returns_none_when_both_absent() {
        let conn = Connection::new();
        assert_eq!(
            get_session_or_setting(&conn, "ROLE", param_names::ROLE),
            None
        );
    }

    #[test]
    fn connection_get_info_returns_all_settings() {
        let ds = driver_state();
        let handle = ds.connection_new();
        ds.connection_set_option(
            handle,
            "host".into(),
            Setting::String("snow.example.com".into()),
        )
        .unwrap();
        ds.connection_set_option(handle, "port".into(), Setting::Int(8080))
            .unwrap();
        ds.connection_set_option(
            handle,
            "account".into(),
            Setting::String("my_account".into()),
        )
        .unwrap();
        ds.connection_set_option(handle, "user".into(), Setting::String("my_user".into()))
            .unwrap();
        ds.connection_set_option(handle, "role".into(), Setting::String("my_role".into()))
            .unwrap();
        ds.connection_set_option(handle, "database".into(), Setting::String("my_db".into()))
            .unwrap();
        ds.connection_set_option(handle, "schema".into(), Setting::String("my_schema".into()))
            .unwrap();
        ds.connection_set_option(handle, "warehouse".into(), Setting::String("my_wh".into()))
            .unwrap();

        let info = ds.connection_get_info(handle).unwrap();

        assert_eq!(info.host, Some("snow.example.com".into()));
        assert_eq!(info.port, Some(8080));
        assert_eq!(info.account, Some("my_account".into()));
        assert_eq!(info.user, Some("my_user".into()));
        assert_eq!(info.role, Some("my_role".into()));
        assert_eq!(info.database, Some("my_db".into()));
        assert_eq!(info.schema, Some("my_schema".into()));
        assert_eq!(info.warehouse, Some("my_wh".into()));

        ds.connection_release(handle).unwrap();
    }

    #[test]
    fn connection_get_info_returns_none_for_unset_fields() {
        let ds = driver_state();
        let handle = ds.connection_new();

        let info = ds.connection_get_info(handle).unwrap();

        assert_eq!(info.host, None);
        assert_eq!(info.port, None);
        assert_eq!(info.account, None);
        assert_eq!(info.user, None);
        assert_eq!(info.role, None);
        assert_eq!(info.database, None);
        assert_eq!(info.schema, None);
        assert_eq!(info.warehouse, None);
        assert!(info.session_token.is_none());
        assert_eq!(info.session_id, None);

        ds.connection_release(handle).unwrap();
    }

    #[test]
    fn connection_get_info_session_params_override_settings() {
        let ds = driver_state();
        let handle = ds.connection_new();
        ds.connection_set_option(
            handle,
            "database".into(),
            Setting::String("original_db".into()),
        )
        .unwrap();
        ds.connection_set_option(
            handle,
            "role".into(),
            Setting::String("original_role".into()),
        )
        .unwrap();

        if let Some(conn_ptr) = ds.connections.get_obj(handle) {
            let conn = conn_ptr.lock().unwrap();
            conn.session_parameters
                .write()
                .unwrap()
                .insert("DATABASE".into(), "session_db".into());
            conn.session_parameters
                .write()
                .unwrap()
                .insert("ROLE".into(), "session_role".into());
        }

        let info = ds.connection_get_info(handle).unwrap();

        assert_eq!(info.database, Some("session_db".into()));
        assert_eq!(info.role, Some("session_role".into()));

        ds.connection_release(handle).unwrap();
    }

    #[test]
    fn connection_get_info_final_names_override_session_params() {
        let ds = driver_state();
        let handle = ds.connection_new();
        ds.connection_set_option(
            handle,
            "database".into(),
            Setting::String("setting_db".into()),
        )
        .unwrap();

        if let Some(conn_ptr) = ds.connections.get_obj(handle) {
            let conn = conn_ptr.lock().unwrap();
            conn.session_parameters
                .write()
                .unwrap()
                .insert("DATABASE".into(), "session_db".into());
            conn.final_session_names.write().unwrap().database = Some("final_db".into());
        }

        let info = ds.connection_get_info(handle).unwrap();
        assert_eq!(info.database, Some("final_db".into()));

        ds.connection_release(handle).unwrap();
    }

    #[test]
    fn update_session_params_cache_stores_final_names_separately() {
        let conn = Connection::new();
        let final_names = FinalSessionNames {
            database: Some("new_db".into()),
            schema: Some("new_schema".into()),
            warehouse: None,
            role: None,
        };

        conn.update_session_params_cache("SELECT 1", None, &final_names);

        let cache = conn.session_parameters.read().unwrap();
        assert!(
            cache.get("DATABASE").is_none(),
            "final names should not be stored in session_parameters"
        );
        assert!(
            cache.get("SCHEMA").is_none(),
            "final names should not be stored in session_parameters"
        );

        let names = conn.final_session_names.read().unwrap();
        assert_eq!(names.database, Some("new_db".into()));
        assert_eq!(names.schema, Some("new_schema".into()));
    }
}
