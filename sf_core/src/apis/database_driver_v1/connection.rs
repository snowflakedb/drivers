use snafu::{OptionExt, ResultExt};
use std::future::Future;
use std::sync::RwLock;
use std::{collections::HashMap, sync::Arc};
use tokio::sync::Mutex;
use tokio::sync::RwLock as AsyncRwLock;

use super::Setting;
use super::error::*;
use super::global_state::DatabaseDriverV1;
use super::validation::{
    ValidationIssue, ValidationSeverity, canonicalize_setting_key, collect_unknown_settings,
    normalize_host_underscores, resolve_options, validate_connection_seed_write,
    validate_session_override_write,
};
use crate::config::ParamStore;
use crate::config::connection_config::ConnectionConfig;
use crate::config::param_registry::{ParamKey, ParamScope, param_names};
use crate::config::resolver;
use crate::config::rest_parameters::{ClientInfo, LoginMethod, LoginParameters, QueryParameters};
use crate::config::retry::RetryPolicy;
use crate::handle_manager::Handle;
use crate::rest::snowflake::{self, RestError, SessionTokens, SnowflakeResponseError};
use crate::sensitive::SensitiveString;
use crate::tls::client::create_tls_client_with_config;
use crate::token_cache::TokenCache;

impl DatabaseDriverV1 {
    /// Set autocommit on the given connection.
    ///
    /// Pre-connect: stores `AUTOCOMMIT` in `init_session_parameters` so it is applied
    /// at login time — no SQL execution required.
    /// Post-connect: executes `ALTER SESSION SET AUTOCOMMIT = TRUE/FALSE`.
    pub async fn connection_set_autocommit(
        &self,
        conn_handle: Handle,
        autocommit: bool,
    ) -> Result<(), ApiError> {
        match self.connections.get_obj(conn_handle) {
            Some(conn_ptr) => {
                let mut conn = conn_ptr.lock().await;
                if conn.is_post_connect() {
                    // The guard is intentionally dropped before calling execute_session_sql,
                    // which internally re-acquires the lock, to avoid a deadlock on the same
                    // mutex. A TOCTOU gap exists between the check and the SQL execution, but
                    // this is safe in the ODBC single-threaded connection model where state
                    // transitions (connect/disconnect) are serialised at the handle level.
                    drop(conn);
                    let sql = if autocommit {
                        "ALTER SESSION SET AUTOCOMMIT = TRUE"
                    } else {
                        "ALTER SESSION SET AUTOCOMMIT = FALSE"
                    };
                    self.execute_session_sql(conn_handle, sql).await
                } else {
                    let value = if autocommit { "true" } else { "false" }.to_string();
                    conn.init_session_parameters
                        .get_or_insert_with(HashMap::new)
                        .insert("AUTOCOMMIT".to_string(), value);
                    Ok(())
                }
            }
            None => InvalidArgumentSnafu {
                argument: "Connection handle not found".to_string(),
            }
            .fail(),
        }
    }

    /// Execute `USE DATABASE "<name>"` on the given connection.
    /// The database name is escaped (internal `"` doubled).
    /// Must only be called after the connection is initialised (`is_post_connect()`).
    pub async fn connection_use_database(
        &self,
        conn_handle: Handle,
        database: &str,
    ) -> Result<(), ApiError> {
        let db = database.trim();
        if db.is_empty() {
            return InvalidArgumentSnafu {
                argument: "database name must not be empty".to_string(),
            }
            .fail();
        }
        match self.connections.get_obj(conn_handle) {
            Some(conn_ptr) => {
                let conn = conn_ptr.lock().await;
                if !conn.is_post_connect() {
                    return InvalidArgumentSnafu {
                        argument: "connection_use_database called before connection is open"
                            .to_string(),
                    }
                    .fail();
                }
                // The guard is intentionally dropped before calling execute_session_sql,
                // which internally re-acquires the lock, to avoid a deadlock on the same
                // mutex. A TOCTOU gap exists between the check and the SQL execution, but
                // this is safe in the ODBC single-threaded connection model where state
                // transitions (connect/disconnect) are serialised at the handle level.
                drop(conn);
            }
            None => {
                return InvalidArgumentSnafu {
                    argument: "Connection handle not found".to_string(),
                }
                .fail();
            }
        }
        let escaped = db.replace('"', "\"\"");
        self.execute_session_sql(conn_handle, &format!("USE DATABASE \"{escaped}\""))
            .await
    }

    /// Execute a session-scoped SQL command using a temporary statement.
    /// Allocates a statement, executes, then releases it regardless of outcome.
    async fn execute_session_sql(&self, conn_handle: Handle, sql: &str) -> Result<(), ApiError> {
        let stmt_handle = self.statement_new(conn_handle)?;
        let result = async {
            self.statement_set_sql_query(stmt_handle, sql.to_string())
                .await?;
            self.statement_execute_query(stmt_handle, None).await
        }
        .await;
        if let Err(e) = self.statement_release(stmt_handle) {
            tracing::warn!("execute_session_sql: failed to release statement: {e:?}");
        }
        result.map(|_| ())
    }

    pub async fn connection_init(
        &self,
        conn_handle: Handle,
        _db_handle: Handle,
    ) -> Result<(), ApiError> {
        match self.connections.get_obj(conn_handle) {
            Some(conn_ptr) => {
                let (config, host, port, client_info, init_params, resolved_snapshot) = {
                    let conn = conn_ptr.lock().await;
                    // TODO(sfc-gh-boler): Clone the mutable connection inputs under the mutex,
                    // then drop the lock before calling resolve/build. Those paths can do
                    // synchronous disk I/O and private-key parsing, which currently extends the
                    // connection mutex critical section and can block the async runtime thread.
                    let mut resolved = conn.resolved_settings().context(ConfigurationSnafu)?;
                    normalize_host_underscores(&mut resolved);
                    let config = ConnectionConfig::build(&resolved).context(ConfigurationSnafu)?;
                    let host = resolved.get_string(param_names::HOST);
                    let port = resolved.get_int(param_names::PORT);
                    let client_info =
                        ClientInfo::from_settings(&resolved).context(ConfigurationSnafu)?;
                    let init_params = conn.init_session_parameters.clone();
                    let resolved_snapshot = resolved.clone();

                    // Forward unrecognized settings as session parameters so
                    // drivers can set arbitrary Snowflake session params
                    // via regular connection options.
                    let unknown_settings = collect_unknown_settings(&conn.connection_seed);
                    let init_params = match init_params {
                        Some(explicit) => {
                            // Normalize explicit keys to uppercase so precedence
                            // is case-insensitive (unknown settings are uppercased).
                            let mut merged: HashMap<String, String> = explicit
                                .into_iter()
                                .map(|(k, v)| (k.to_uppercase(), v))
                                .collect();
                            // Explicit session params take precedence
                            for (k, v) in unknown_settings {
                                merged.entry(k).or_insert(v);
                            }
                            Some(merged)
                        }
                        None if !unknown_settings.is_empty() => Some(unknown_settings),
                        None => None,
                    };

                    (
                        config,
                        host,
                        port,
                        client_info,
                        init_params,
                        resolved_snapshot,
                    )
                };

                let http_client = create_tls_client_with_config(config.tls.clone())
                    .context(TlsClientCreationSnafu)?;
                let login_parameters =
                    LoginParameters::from_connection_config(&config, client_info, None);

                let mfa_caching_requested = matches!(
                    &login_parameters.login_method,
                    LoginMethod::UserPasswordMfa {
                        client_store_temporary_credential: true,
                        ..
                    }
                );

                let token_cache = if mfa_caching_requested {
                    Some(self.token_cache().context(TokenCacheInitializationSnafu)?)
                } else {
                    None
                };

                let login_result = crate::rest::snowflake::snowflake_login_with_client(
                    &http_client,
                    &login_parameters,
                    init_params.as_ref(),
                    token_cache.map(|c| c as &dyn TokenCache),
                )
                .await
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
                    .await
                    .initialize(
                        login_result.tokens,
                        http_client,
                        host,
                        port,
                        login_parameters.server_url.clone(),
                        login_parameters.client_info.clone(),
                        merged_params,
                        login_final_names,
                        resolved_snapshot,
                    )
                    .await;
                Ok(())
            }
            None => InvalidArgumentSnafu {
                argument: "Connection handle not found".to_string(),
            }
            .fail(),
        }
    }

    pub async fn connection_set_option(
        &self,
        handle: Handle,
        key: String,
        value: Setting,
    ) -> Result<(), ApiError> {
        match self.connections.get_obj(handle) {
            Some(conn_ptr) => {
                let mut conn = conn_ptr.lock().await;
                let post = conn.is_post_connect();
                let (canonical, def) = canonicalize_setting_key(&key);
                validate_connection_seed_write(post, def)?;
                conn.connection_seed.insert(canonical, value);
                Ok(())
            }
            None => InvalidArgumentSnafu {
                argument: "Connection handle not found".to_string(),
            }
            .fail(),
        }
    }

    pub async fn connection_set_options(
        &self,
        handle: Handle,
        options: HashMap<String, Setting>,
    ) -> Result<Vec<ValidationIssue>, ApiError> {
        match self.connections.get_obj(handle) {
            Some(conn_ptr) => {
                let mut conn = conn_ptr.lock().await;
                let post = conn.is_post_connect();
                let (resolved, issues) = resolve_options(options);
                let error_messages: Vec<String> = issues
                    .iter()
                    .filter(|i| i.severity == ValidationSeverity::Error)
                    .map(|i| i.to_string())
                    .collect();
                if !error_messages.is_empty() {
                    return InvalidArgumentSnafu {
                        argument: error_messages.join("; "),
                    }
                    .fail();
                }
                for key in resolved.keys() {
                    let def = crate::config::param_registry::registry().resolve(key.as_str());
                    validate_connection_seed_write(post, def)?;
                }
                for (key, value) in resolved {
                    conn.connection_seed.insert(key, value);
                }
                Ok(issues
                    .into_iter()
                    .filter(|i| i.severity == ValidationSeverity::Warning)
                    .collect())
            }
            None => InvalidArgumentSnafu {
                argument: "Connection handle not found".to_string(),
            }
            .fail(),
        }
    }

    pub async fn connection_validate_options(
        &self,
        conn_handle: Handle,
    ) -> Result<Vec<ValidationIssue>, ApiError> {
        match self.connections.get_obj(conn_handle) {
            Some(conn_ptr) => {
                let conn = conn_ptr.lock().await;
                // TODO(sfc-gh-boler): Clone `conn.connection_seed` under the mutex and run
                // resolve/validate after releasing the lock, since layered config resolution may
                // perform synchronous file I/O.
                let resolved = conn.resolved_settings().context(ConfigurationSnafu)?;
                Ok(crate::config::connection_config::validate_settings(
                    &resolved,
                ))
            }
            None => InvalidArgumentSnafu {
                argument: "Connection handle not found".to_string(),
            }
            .fail(),
        }
    }

    pub async fn connection_set_session_parameters(
        &self,
        handle: Handle,
        parameters: HashMap<String, String>,
    ) -> Result<(), ApiError> {
        match self.connections.get_obj(handle) {
            Some(conn_ptr) => {
                let mut conn = conn_ptr.lock().await;
                conn.init_session_parameters = Some(parameters);
                Ok(())
            }
            None => InvalidArgumentSnafu {
                argument: "Connection handle not found".to_string(),
            }
            .fail(),
        }
    }

    /// Set a session-scoped parameter for the current session only (post-connect).
    ///
    /// Connection-scoped and statement-scoped registry parameters must use their respective
    /// setters. Unknown keys are accepted as opaque session overrides.
    pub async fn connection_set_session_option(
        &self,
        handle: Handle,
        key: String,
        value: Setting,
    ) -> Result<(), ApiError> {
        match self.connections.get_obj(handle) {
            Some(conn_ptr) => {
                let mut conn = conn_ptr.lock().await;
                if !conn.is_post_connect() {
                    return InvalidArgumentSnafu {
                        argument:
                            "Session options are only available after the connection is established"
                                .to_string(),
                    }
                    .fail();
                }
                let (canonical, def) = canonicalize_setting_key(&key);
                validate_session_override_write(def)?;
                conn.session_overrides.insert(canonical, value);
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
    /// Explicit connection-string / API options (merged as the top layer in [`resolver::resolve`]).
    pub(crate) connection_seed: ParamStore,
    /// Resolved settings snapshot captured at successful login (defaults + files + seed).
    pub(crate) resolved_connect: Option<ParamStore>,
    /// Typed session overrides set after connect (session scope only).
    pub(crate) session_overrides: ParamStore,
    /// Session tokens - RwLock allows concurrent reads, exclusive writes for refresh
    pub tokens: Arc<AsyncRwLock<Option<SessionTokens>>>,
    pub http_client: Option<reqwest::Client>,
    pub retry_policy: RetryPolicy,
    /// Effective host after layered config resolution.
    pub host: Option<String>,
    /// Effective port after layered config resolution.
    pub port: Option<i64>,
    /// Server URL for refresh requests
    pub server_url: Option<String>,
    /// Client info for refresh requests
    pub client_info: Option<ClientInfo>,
    /// Session parameters cache (populated after login)
    pub session_parameters: Arc<AsyncRwLock<HashMap<String, String>>>,
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
            connection_seed: ParamStore::new(),
            resolved_connect: None,
            session_overrides: ParamStore::new(),
            tokens: Arc::new(AsyncRwLock::new(None)),
            http_client: None,
            retry_policy: RetryPolicy::default(),
            host: None,
            port: None,
            server_url: None,
            client_info: None,
            session_parameters: Arc::new(AsyncRwLock::new(HashMap::new())),
            init_session_parameters: None,
            final_session_names: RwLock::new(FinalSessionNames::default()),
        }
    }

    /// `true` after a successful [`Connection::initialize`] (post-login transport is ready).
    pub(crate) fn is_post_connect(&self) -> bool {
        self.http_client.is_some()
    }

    /// Server URL + client fingerprint for query and refresh calls (transport snapshot).
    pub(crate) fn query_transport_parameters(&self) -> Result<QueryParameters, ApiError> {
        Ok(QueryParameters {
            server_url: self
                .server_url
                .clone()
                .context(ConnectionNotInitializedSnafu)?,
            client_info: self
                .client_info
                .clone()
                .context(ConnectionNotInitializedSnafu)?,
        })
    }

    /// Convenience setter for tests and direct call sites.
    pub fn set_option(&mut self, key: String, value: Setting) {
        self.connection_seed.insert(key, value);
    }

    fn resolved_settings(&self) -> Result<ParamStore, crate::config::ConfigError> {
        resolver::resolve(&self.connection_seed)
    }

    #[allow(clippy::too_many_arguments)]
    pub(crate) async fn initialize(
        &mut self,
        tokens: SessionTokens,
        http_client: reqwest::Client,
        host: Option<String>,
        port: Option<i64>,
        server_url: String,
        client_info: ClientInfo,
        session_params: HashMap<String, String>,
        final_names: FinalSessionNames,
        resolved_connect: ParamStore,
    ) {
        *self.tokens.write().await = Some(tokens);
        self.http_client = Some(http_client);
        self.host = host;
        self.port = port;
        self.server_url = Some(server_url);
        self.client_info = Some(client_info);
        self.resolved_connect = Some(resolved_connect);
        self.session_overrides = ParamStore::new();

        let mut cache = self.session_parameters.write().await;
        *cache = session_params;
        drop(cache);

        if let Ok(mut names) = self.final_session_names.write() {
            *names = final_names;
        }
    }

    /// Update the session parameters cache after a successful query.
    pub async fn update_session_params_cache(
        &self,
        query: &str,
        response_parameters: Option<
            &Vec<crate::rest::snowflake::query_response::NameValueParameter>,
        >,
        final_names: &FinalSessionNames,
    ) {
        let mut cache = self.session_parameters.write().await;

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
    let mut ctx = RefreshContext::from_arc(conn).await?;
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
    pub async fn from_arc(conn: &Arc<Mutex<Connection>>) -> Result<Self, ApiError> {
        let guard = conn.lock().await;
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

/// HTTP response returned by connection_send_http_request
#[derive(Debug, Clone)]
pub struct HttpResponse {
    pub status_code: i32,
    pub headers: HashMap<String, String>,
    pub body: Vec<u8>,
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
    /// The master token for session renewal (redacted in Debug output)
    pub master_token: Option<SensitiveString>,
}

fn setting_as_display_string(setting: &Setting) -> Option<String> {
    match setting {
        Setting::String(s) => Some(s.clone()),
        Setting::Int(i) => Some(i.to_string()),
        Setting::Bool(b) => Some(b.to_string()),
        Setting::Double(d) => Some(d.to_string()),
        Setting::Bytes(_) => None,
    }
}

fn resolved_or_seed_string(conn: &Connection, key: ParamKey) -> Option<String> {
    if let Some(resolved) = &conn.resolved_connect
        && let Some(s) = resolved.get_string(key)
    {
        return Some(s);
    }
    conn.connection_seed.get_string(key)
}

/// Connection-scoped string from the live seed (writes rejected after connect when immutable).
fn get_connection_seed_string(conn: &Connection, key: ParamKey) -> Option<String> {
    conn.connection_seed.get(key).and_then(|s| {
        if let Setting::String(v) = s {
            Some(v.clone())
        } else {
            None
        }
    })
}

/// Session effective read: server cache → session overrides → resolved connect (then seed).
fn get_session_or_setting(
    conn: &Connection,
    param_name: &str,
    setting_key: ParamKey,
) -> Option<String> {
    if let Ok(cache) = conn.session_parameters.try_read()
        && let Some(v) = cache.get(param_name)
        && !v.is_empty()
    {
        return Some(v.clone());
    }
    if let Some(s) = conn
        .session_overrides
        .get(setting_key)
        .and_then(setting_as_display_string)
    {
        return Some(s);
    }
    resolved_or_seed_string(conn, setting_key)
}

impl DatabaseDriverV1 {
    /// Get connection information for the given connection handle
    pub async fn connection_get_info(
        &self,
        conn_handle: Handle,
    ) -> Result<ConnectionInfo, ApiError> {
        match self.connections.get_obj(conn_handle) {
            Some(conn_ptr) => {
                let conn = conn_ptr.lock().await;

                let host = conn
                    .host
                    .clone()
                    .or_else(|| conn.connection_seed.get_string(param_names::HOST));

                let port = conn
                    .port
                    .or_else(|| conn.connection_seed.get_int(param_names::PORT));

                let server_url = conn.server_url.clone();

                let (session_token, session_id, master_token) = {
                    let tokens_guard = conn.tokens.read().await;
                    match tokens_guard.as_ref() {
                        Some(tokens) => (
                            Some(tokens.session_token.clone()),
                            Some(tokens.session_id),
                            Some(tokens.master_token.clone()),
                        ),
                        None => (None, None, None),
                    }
                };

                let account = get_connection_seed_string(&conn, param_names::ACCOUNT);
                let user = get_connection_seed_string(&conn, param_names::USER);

                // Resolution order for session-scoped values:
                //   1. final_session_names  (server-echoed via login sessionInfo or query finalXxxName)
                //   2. session_parameters   (server-returned session params, only pre-login / missing sessionInfo)
                //   3. session_overrides     (post-connect session setters)
                //   4. resolved_connect (login snapshot) then connection_seed
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
                    master_token,
                })
            }
            None => InvalidArgumentSnafu {
                argument: "Connection handle not found".to_string(),
            }
            .fail(),
        }
    }

    pub async fn connection_get_query_status(
        &self,
        conn_handle: Handle,
        query_id: &str,
    ) -> Result<snowflake::QueryStatusResult, ApiError> {
        if query_id.is_empty() {
            return InvalidArgumentSnafu {
                argument: "query_id must be non-empty".to_string(),
            }
            .fail();
        }

        let conn_ptr = self.connections.get_obj(conn_handle).ok_or_else(|| {
            InvalidArgumentSnafu {
                argument: "Connection handle not found".to_string(),
            }
            .build()
        })?;

        let (http_client, server_url, client_info, retry_policy) = {
            let conn = conn_ptr.lock().await;
            (
                conn.http_client
                    .clone()
                    .context(ConnectionNotInitializedSnafu)?,
                conn.server_url
                    .clone()
                    .context(ConnectionNotInitializedSnafu)?,
                conn.client_info
                    .clone()
                    .context(ConnectionNotInitializedSnafu)?,
                conn.retry_policy.clone(),
            )
        };

        with_valid_session(&conn_ptr, |token| {
            let http_client = &http_client;
            let server_url = &server_url;
            let client_info = &client_info;
            let retry_policy = &retry_policy;
            async move {
                snowflake::get_query_status(
                    http_client,
                    server_url,
                    client_info,
                    &token,
                    query_id,
                    retry_policy,
                )
                .await
            }
        })
        .await
    }

    pub async fn connection_get_parameter(
        &self,
        conn_handle: Handle,
        key: String,
    ) -> Result<Option<String>, ApiError> {
        match self.connections.get_obj(conn_handle) {
            Some(conn_ptr) => {
                let conn = conn_ptr.lock().await;

                let cache = conn.session_parameters.read().await;

                let normalized_key = key.to_uppercase();
                if let Some(v) = cache.get(&normalized_key).filter(|s| !s.is_empty()) {
                    return Ok(Some(v.clone()));
                }
                drop(cache);

                let (canonical, def) = canonicalize_setting_key(&key);
                if let Some(d) = def
                    && d.scope == ParamScope::Session
                {
                    if let Some(s) = conn
                        .session_overrides
                        .get_any(&canonical)
                        .and_then(setting_as_display_string)
                    {
                        return Ok(Some(s));
                    }
                    return Ok(resolved_or_seed_string(&conn, ParamKey(d.canonical_name)));
                }

                if let Some(s) = conn
                    .session_overrides
                    .get_any(&canonical)
                    .and_then(setting_as_display_string)
                {
                    return Ok(Some(s));
                }

                Ok(conn
                    .resolved_connect
                    .as_ref()
                    .and_then(|r| r.get_any(&canonical))
                    .and_then(setting_as_display_string)
                    .or_else(|| {
                        conn.connection_seed
                            .get_any(&canonical)
                            .and_then(setting_as_display_string)
                    }))
            }
            None => InvalidArgumentSnafu {
                argument: "Connection handle not found".to_string(),
            }
            .fail(),
        }
    }

    /// Send an HTTP request through the connection's TLS-configured client.
    ///
    /// The `url` must be a relative path (e.g. `/session/token-request`).
    /// It is resolved against the connection's `server_url`. Absolute URLs
    /// are rejected to prevent SSRF / token leakage to arbitrary hosts.
    /// Auth is always the current session token managed by sf_core.
    pub async fn connection_send_http_request(
        &self,
        conn_handle: Handle,
        method: String,
        url: String,
        headers: HashMap<String, String>,
        body: Option<Vec<u8>>,
    ) -> Result<HttpResponse, ApiError> {
        if url.starts_with("http://") || url.starts_with("https://") || url.starts_with("//") {
            return InvalidArgumentSnafu {
                argument: format!(
                    "Absolute URLs are not allowed; pass a relative path instead: {url}"
                ),
            }
            .fail();
        }
        if reqwest::Url::parse(&url).is_ok() {
            return InvalidArgumentSnafu {
                argument: format!(
                    "Absolute URLs are not allowed; pass a relative path instead: {url}"
                ),
            }
            .fail();
        }

        let conn_ptr = self
            .connections
            .get_obj(conn_handle)
            .context(InvalidArgumentSnafu {
                argument: "Connection handle not found".to_string(),
            })?;

        // Extract needed fields under the lock, then release before network I/O
        let (http_client, server_url, token) = {
            let conn = conn_ptr.lock().await;

            let http_client = conn
                .http_client
                .clone()
                .context(ConnectionNotInitializedSnafu)?;

            let server_url = conn
                .server_url
                .clone()
                .context(ConnectionNotInitializedSnafu)?;

            let tokens_guard = conn.tokens.read().await;
            let token = tokens_guard
                .as_ref()
                .context(ConnectionNotInitializedSnafu)?
                .session_token
                .reveal()
                .to_string();

            (http_client, server_url, token)
        };

        let full_url = reqwest::Url::parse(&server_url)
            .and_then(|base| base.join(&url))
            .map(|u| u.to_string())
            .map_err(|_| {
                InvalidArgumentSnafu {
                    argument: format!("Failed to resolve URL '{url}' against '{server_url}'"),
                }
                .build()
            })?;

        let method = method.to_uppercase();
        let reqwest_method = match method.as_str() {
            "GET" => reqwest::Method::GET,
            "POST" => reqwest::Method::POST,
            "PUT" => reqwest::Method::PUT,
            "DELETE" => reqwest::Method::DELETE,
            "PATCH" => reqwest::Method::PATCH,
            other => {
                return InvalidArgumentSnafu {
                    argument: format!("Unsupported HTTP method: {other}"),
                }
                .fail();
            }
        };

        let auth_value =
            reqwest::header::HeaderValue::from_str(&format!("Snowflake Token=\"{token}\""))
                .map_err(|_| {
                    InvalidArgumentSnafu {
                        argument: "Session token contains invalid header characters".to_string(),
                    }
                    .build()
                })?;

        let mut builder = http_client
            .request(reqwest_method, &full_url)
            .header(reqwest::header::AUTHORIZATION, auth_value);

        for (key, value) in &headers {
            let header_name =
                reqwest::header::HeaderName::from_bytes(key.as_bytes()).map_err(|_| {
                    InvalidArgumentSnafu {
                        argument: format!("Invalid header name: {key}"),
                    }
                    .build()
                })?;
            if header_name == reqwest::header::AUTHORIZATION || header_name == reqwest::header::HOST
            {
                tracing::warn!(
                    header = %header_name,
                    "Ignoring caller-supplied security-sensitive header; managed by sf_core"
                );
                continue;
            }
            let header_value = reqwest::header::HeaderValue::from_str(value).map_err(|_| {
                InvalidArgumentSnafu {
                    argument: format!("Invalid header value for '{key}'"),
                }
                .build()
            })?;
            builder = builder.header(header_name, header_value);
        }

        if let Some(body_bytes) = body {
            builder = builder.body(body_bytes);
        }

        let response = builder.send().await.context(HttpRequestSnafu {
            context: format!("Failed to send {method} {full_url}"),
        })?;

        let status_code = response.status().as_u16() as i32;
        let response_headers: HashMap<String, String> = response
            .headers()
            .iter()
            .map(|(k, v)| {
                let value = match v.to_str() {
                    Ok(s) => s.to_string(),
                    Err(_) => String::from_utf8_lossy(v.as_bytes()).into_owned(),
                };
                (k.to_string(), value)
            })
            .collect();
        let response_body = response.bytes().await.context(HttpRequestSnafu {
            context: "Failed to read response body".to_string(),
        })?;

        Ok(HttpResponse {
            status_code,
            headers: response_headers,
            body: response_body.to_vec(),
        })
    }

    /// Execute a token request (ISSUE/RENEW) using the connection's master token.
    pub async fn connection_token_request(
        &self,
        conn_handle: Handle,
        request_type: String,
    ) -> Result<snowflake::TokenRequestResult, ApiError> {
        if request_type != "ISSUE" && request_type != "RENEW" {
            return InvalidArgumentSnafu {
                argument: format!(
                    "Invalid request_type '{request_type}', must be 'ISSUE' or 'RENEW'"
                ),
            }
            .fail();
        }

        let conn_ptr = self
            .connections
            .get_obj(conn_handle)
            .context(InvalidArgumentSnafu {
                argument: "Connection handle not found".to_string(),
            })?;

        // Extract needed fields under the lock, then release before network I/O
        let (http_client, server_url, client_info, tokens) = {
            let conn = conn_ptr.lock().await;

            let http_client = conn
                .http_client
                .clone()
                .context(ConnectionNotInitializedSnafu)?;

            let server_url = conn
                .server_url
                .clone()
                .context(ConnectionNotInitializedSnafu)?;

            let client_info = conn
                .client_info
                .clone()
                .context(ConnectionNotInitializedSnafu)?;

            let tokens_guard = conn.tokens.read().await;
            let tokens = tokens_guard
                .as_ref()
                .context(ConnectionNotInitializedSnafu)?
                .clone();

            (http_client, server_url, client_info, tokens)
        };

        snowflake::token_request(
            &http_client,
            &server_url,
            &client_info,
            &tokens,
            &request_type,
        )
        .await
        .context(TokenRequestSnafu)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::ParamStore;
    use crate::config::param_registry::param_names;

    fn make_connection_with_settings(settings: Vec<(&str, Setting)>) -> Connection {
        let mut conn = Connection::new();
        for (key, value) in settings {
            conn.connection_seed.insert(key.to_string(), value);
        }
        conn
    }

    #[test]
    fn get_connection_seed_string_returns_value() {
        let conn =
            make_connection_with_settings(vec![("host", Setting::String("example.com".into()))]);
        assert_eq!(
            get_connection_seed_string(&conn, param_names::HOST),
            Some("example.com".into())
        );
    }

    #[test]
    fn get_connection_seed_string_returns_none_for_missing_key() {
        let conn = Connection::new();
        assert_eq!(get_connection_seed_string(&conn, param_names::HOST), None);
    }

    #[test]
    fn get_connection_seed_string_returns_none_for_non_string_type() {
        let conn = make_connection_with_settings(vec![("port", Setting::Int(443))]);
        assert_eq!(get_connection_seed_string(&conn, param_names::PORT), None);
    }

    #[test]
    fn get_session_or_setting_prefers_session_parameter() {
        let conn =
            make_connection_with_settings(vec![("database", Setting::String("setting_db".into()))]);
        conn.session_parameters
            .try_write()
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
            .try_write()
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
    fn get_session_or_setting_prefers_resolved_connect_over_seed() {
        let mut conn =
            make_connection_with_settings(vec![("database", Setting::String("seed_db".into()))]);
        let mut resolved = ParamStore::new();
        resolved.insert("database".into(), Setting::String("resolved_db".into()));
        conn.resolved_connect = Some(resolved);
        assert_eq!(
            get_session_or_setting(&conn, "DATABASE", param_names::DATABASE),
            Some("resolved_db".into())
        );
    }

    #[test]
    fn get_session_or_setting_prefers_session_overrides_over_resolved() {
        let mut conn =
            make_connection_with_settings(vec![("database", Setting::String("seed_db".into()))]);
        let mut resolved = ParamStore::new();
        resolved.insert("database".into(), Setting::String("resolved_db".into()));
        conn.resolved_connect = Some(resolved);
        conn.session_overrides
            .insert("database".into(), Setting::String("override_db".into()));
        assert_eq!(
            get_session_or_setting(&conn, "DATABASE", param_names::DATABASE),
            Some("override_db".into())
        );
    }

    #[tokio::test]
    async fn connection_rejects_statement_scoped_param_on_connection_seed() {
        let ds = DatabaseDriverV1::new();
        let handle = ds.connection_new();
        let err = ds
            .connection_set_option(handle, "async_execution".into(), Setting::Bool(true))
            .await
            .unwrap_err();
        let msg = err.to_string();
        assert!(msg.contains("statement-scoped"), "unexpected error: {msg}");
        ds.connection_release(handle).unwrap();
    }

    #[tokio::test]
    async fn connection_rejects_session_scoped_after_post_connect() {
        let ds = DatabaseDriverV1::new();
        let handle = ds.connection_new();
        if let Some(c) = ds.connections.get_obj(handle) {
            let mut conn = c.lock().await;
            conn.http_client = Some(reqwest::Client::new());
        }
        let err = ds
            .connection_set_option(handle, "database".into(), Setting::String("x".into()))
            .await
            .unwrap_err();
        assert!(
            err.to_string().contains("session-scoped"),
            "unexpected: {err}"
        );
        ds.connection_release(handle).unwrap();
    }

    #[tokio::test]
    async fn connection_rejects_immutable_connection_param_after_post_connect() {
        let ds = DatabaseDriverV1::new();
        let handle = ds.connection_new();
        if let Some(c) = ds.connections.get_obj(handle) {
            let mut conn = c.lock().await;
            conn.http_client = Some(reqwest::Client::new());
        }
        let err = ds
            .connection_set_option(handle, "account".into(), Setting::String("other".into()))
            .await
            .unwrap_err();
        assert!(
            err.to_string().contains("cannot be changed after connect"),
            "unexpected: {err}"
        );
        ds.connection_release(handle).unwrap();
    }

    #[tokio::test]
    async fn connection_set_session_option_rejects_before_post_connect() {
        let ds = DatabaseDriverV1::new();
        let handle = ds.connection_new();
        let err = ds
            .connection_set_session_option(handle, "database".into(), Setting::String("db".into()))
            .await
            .unwrap_err();
        assert!(
            err.to_string()
                .contains("after the connection is established")
        );
        ds.connection_release(handle).unwrap();
    }

    #[tokio::test]
    async fn connection_get_info_returns_all_settings() {
        let ds = DatabaseDriverV1::new();
        let handle = ds.connection_new();
        ds.connection_set_option(
            handle,
            "host".into(),
            Setting::String("snow.example.com".into()),
        )
        .await
        .unwrap();
        ds.connection_set_option(handle, "port".into(), Setting::Int(8080))
            .await
            .unwrap();
        ds.connection_set_option(
            handle,
            "account".into(),
            Setting::String("my_account".into()),
        )
        .await
        .unwrap();
        ds.connection_set_option(handle, "user".into(), Setting::String("my_user".into()))
            .await
            .unwrap();
        ds.connection_set_option(handle, "role".into(), Setting::String("my_role".into()))
            .await
            .unwrap();
        ds.connection_set_option(handle, "database".into(), Setting::String("my_db".into()))
            .await
            .unwrap();
        ds.connection_set_option(handle, "schema".into(), Setting::String("my_schema".into()))
            .await
            .unwrap();
        ds.connection_set_option(handle, "warehouse".into(), Setting::String("my_wh".into()))
            .await
            .unwrap();

        let info = ds.connection_get_info(handle).await.unwrap();

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

    #[tokio::test]
    async fn connection_get_info_returns_none_for_unset_fields() {
        let ds = DatabaseDriverV1::new();
        let handle = ds.connection_new();

        let info = ds.connection_get_info(handle).await.unwrap();

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

    #[tokio::test]
    async fn connection_get_info_session_params_override_settings() {
        let ds = DatabaseDriverV1::new();
        let handle = ds.connection_new();
        ds.connection_set_option(
            handle,
            "database".into(),
            Setting::String("original_db".into()),
        )
        .await
        .unwrap();
        ds.connection_set_option(
            handle,
            "role".into(),
            Setting::String("original_role".into()),
        )
        .await
        .unwrap();

        if let Some(conn_ptr) = ds.connections.get_obj(handle) {
            let conn = conn_ptr.lock().await;
            conn.session_parameters
                .write()
                .await
                .insert("DATABASE".into(), "session_db".into());
            conn.session_parameters
                .write()
                .await
                .insert("ROLE".into(), "session_role".into());
        }

        let info = ds.connection_get_info(handle).await.unwrap();

        assert_eq!(info.database, Some("session_db".into()));
        assert_eq!(info.role, Some("session_role".into()));

        ds.connection_release(handle).unwrap();
    }

    #[tokio::test]
    async fn connection_get_info_final_names_override_session_params() {
        let ds = DatabaseDriverV1::new();
        let handle = ds.connection_new();
        ds.connection_set_option(
            handle,
            "database".into(),
            Setting::String("setting_db".into()),
        )
        .await
        .unwrap();

        if let Some(conn_ptr) = ds.connections.get_obj(handle) {
            let conn = conn_ptr.lock().await;
            conn.session_parameters
                .write()
                .await
                .insert("DATABASE".into(), "session_db".into());
            conn.final_session_names.write().unwrap().database = Some("final_db".into());
        }

        let info = ds.connection_get_info(handle).await.unwrap();
        assert_eq!(info.database, Some("final_db".into()));

        ds.connection_release(handle).unwrap();
    }

    #[tokio::test]
    async fn update_session_params_cache_stores_final_names_separately() {
        let conn = Connection::new();
        let final_names = FinalSessionNames {
            database: Some("new_db".into()),
            schema: Some("new_schema".into()),
            warehouse: None,
            role: None,
        };

        conn.update_session_params_cache("SELECT 1", None, &final_names)
            .await;

        let cache = conn.session_parameters.read().await;
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

    async fn setup_connection_for_http_tests(ds: &DatabaseDriverV1) -> Handle {
        let handle = ds.connection_new();
        if let Some(c) = ds.connections.get_obj(handle) {
            let mut conn = c.lock().await;
            conn.http_client = Some(
                reqwest::Client::builder()
                    .timeout(std::time::Duration::from_millis(100))
                    .build()
                    .unwrap(),
            );
            conn.server_url = Some("https://192.0.2.1:9".into());
            let tokens = SessionTokens {
                session_token: "test-session-token".into(),
                master_token: "test-master-token".into(),
                session_id: 1,
                session_expires_at: None,
                master_expires_at: None,
            };
            *conn.tokens.write().await = Some(tokens);
        }
        handle
    }

    #[tokio::test]
    async fn send_http_rejects_absolute_https_url() {
        let ds = DatabaseDriverV1::new();
        let handle = setup_connection_for_http_tests(&ds).await;
        let err = ds
            .connection_send_http_request(
                handle,
                "GET".into(),
                "https://evil.com/steal".into(),
                HashMap::new(),
                None,
            )
            .await
            .unwrap_err();
        assert!(
            err.to_string().contains("Absolute URLs are not allowed"),
            "unexpected error: {err}"
        );
        ds.connection_release(handle).unwrap();
    }

    #[tokio::test]
    async fn send_http_rejects_absolute_http_url() {
        let ds = DatabaseDriverV1::new();
        let handle = setup_connection_for_http_tests(&ds).await;
        let err = ds
            .connection_send_http_request(
                handle,
                "GET".into(),
                "http://evil.com/steal".into(),
                HashMap::new(),
                None,
            )
            .await
            .unwrap_err();
        assert!(
            err.to_string().contains("Absolute URLs are not allowed"),
            "unexpected error: {err}"
        );
        ds.connection_release(handle).unwrap();
    }

    #[tokio::test]
    async fn send_http_rejects_scheme_relative_url() {
        let ds = DatabaseDriverV1::new();
        let handle = setup_connection_for_http_tests(&ds).await;
        let err = ds
            .connection_send_http_request(
                handle,
                "GET".into(),
                "//evil.com/path".into(),
                HashMap::new(),
                None,
            )
            .await
            .unwrap_err();
        assert!(
            err.to_string().contains("Absolute URLs are not allowed"),
            "unexpected error: {err}"
        );
        ds.connection_release(handle).unwrap();
    }

    #[tokio::test]
    async fn send_http_rejects_unsupported_method() {
        let ds = DatabaseDriverV1::new();
        let handle = setup_connection_for_http_tests(&ds).await;
        let err = ds
            .connection_send_http_request(
                handle,
                "TRACE".into(),
                "/session/token-request".into(),
                HashMap::new(),
                None,
            )
            .await
            .unwrap_err();
        assert!(
            err.to_string().contains("Unsupported HTTP method"),
            "unexpected error: {err}"
        );
        ds.connection_release(handle).unwrap();
    }

    #[tokio::test]
    async fn send_http_strips_authorization_header() {
        let ds = DatabaseDriverV1::new();
        let handle = setup_connection_for_http_tests(&ds).await;

        let mut headers = HashMap::new();
        headers.insert("Authorization".into(), "Bearer evil-token".into());
        headers.insert("Content-Type".into(), "application/json".into());

        // The call will fail at the network level (no real server), but it should
        // get past the header validation without error -- the Authorization header
        // is silently stripped, not rejected.
        let result = ds
            .connection_send_http_request(
                handle,
                "POST".into(),
                "/session/token-request".into(),
                headers,
                None,
            )
            .await;

        // We expect a network error (connection refused / DNS), not an InvalidArgument
        assert!(
            result.is_err(),
            "expected network error from non-existent server"
        );
        let err = result.unwrap_err();
        assert!(
            !err.to_string().contains("Authorization"),
            "Authorization header should be silently stripped, not cause an error: {err}"
        );
        ds.connection_release(handle).unwrap();
    }

    #[tokio::test]
    async fn send_http_rejects_invalid_header_name() {
        let ds = DatabaseDriverV1::new();
        let handle = setup_connection_for_http_tests(&ds).await;

        let mut headers = HashMap::new();
        headers.insert("Invalid Header\nName".into(), "value".into());

        let err = ds
            .connection_send_http_request(handle, "GET".into(), "/api/test".into(), headers, None)
            .await
            .unwrap_err();
        assert!(
            err.to_string().contains("Invalid header name"),
            "unexpected error: {err}"
        );
        ds.connection_release(handle).unwrap();
    }

    #[tokio::test]
    async fn token_request_rejects_invalid_request_type() {
        let ds = DatabaseDriverV1::new();
        let handle = setup_connection_for_http_tests(&ds).await;
        let err = ds
            .connection_token_request(handle, "INVALID".into())
            .await
            .unwrap_err();
        assert!(
            err.to_string().contains("must be 'ISSUE' or 'RENEW'"),
            "unexpected error: {err}"
        );
        ds.connection_release(handle).unwrap();
    }
}
