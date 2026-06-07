use snafu::{OptionExt, ResultExt};
use std::future::Future;
use std::sync::RwLock;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;
use std::{collections::HashMap, sync::Arc};
use tokio::sync::Mutex;
use tokio::sync::RwLock as AsyncRwLock;
use tracing::Instrument;

use super::Setting;
use super::async_query_registry::AsyncQueryRegistry;
use super::error::*;
use super::global_state::DatabaseDriverV1;
use super::heartbeat::{HeartbeatHandle, compute_heartbeat_interval, spawn_heartbeat_task};
use super::logout;
use super::spcs_token::read_spcs_token;
use super::validation::{
    ValidationIssue, ValidationSeverity, canonicalize_setting_key, collect_unknown_settings,
    normalize_host_underscores, resolve_options, validate_connection_seed_write,
    validate_session_override_write,
};
use crate::config::ParamStore;
use crate::config::connection_config::ConnectionConfig;
use crate::config::logout::LogoutConfig;
use crate::config::param_registry::{ParamKey, ParamScope, param_names};
use crate::config::resolver;
use crate::config::rest_parameters::{
    ClientInfo, LoginMethod, LoginParameters, QueryParameters, resolve_log_max_query_length,
    resolve_log_query_parameters, resolve_log_query_text,
};
use crate::config::retry::RetryPolicy;
use crate::handle_manager::Handle;
use crate::rest::snowflake::{
    self, QueryExecutionMode, QueryInput, RestError, SessionTokens, SnowflakeResponseError,
    heartbeat, snowflake_query_with_client,
};
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
                    let sql = if autocommit {
                        "ALTER SESSION SET AUTOCOMMIT = TRUE"
                    } else {
                        "ALTER SESSION SET AUTOCOMMIT = FALSE"
                    };
                    self.execute_session_sql(&conn, sql).await
                } else {
                    conn.init_session_parameters
                        .get_or_insert_with(HashMap::new)
                        .insert("AUTOCOMMIT".to_string(), autocommit.to_string());
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
        // TODO: ensure that this is correct escaping
        let escaped = db.replace('"', "\"\"");
        let sql = format!("USE DATABASE \"{escaped}\"");

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
                self.execute_session_sql(&conn, &sql).await
            }
            None => InvalidArgumentSnafu {
                argument: "Connection handle not found".to_string(),
            }
            .fail(),
        }
    }

    /// Execute a session-scoped SQL command without creating a statement.
    async fn execute_session_sql(&self, conn: &Connection, sql: &str) -> Result<(), ApiError> {
        let query_input = QueryInput {
            sql: sql.to_string(),
            bindings: None,
            describe_only: None,
            query_parameters: None,
        };
        let query_parameters = conn.query_transport_parameters()?;
        let http_client = conn
            .http_client
            .clone()
            .context(ConnectionNotInitializedSnafu)?;
        let retry_policy = conn.retry_policy.clone();

        let mut ctx = RefreshContext::new(conn)?;
        let mut last_error = None;
        let response = loop {
            let session_token = ctx.refresh_token(last_error).await?;
            match snowflake_query_with_client(
                &http_client,
                query_parameters.clone(),
                session_token.reveal(),
                query_input.clone(),
                &retry_policy,
                QueryExecutionMode::Blocking,
            )
            .await
            {
                Ok(result) => break Ok(result),
                Err(e) => last_error = Some(e),
            }
        }?;

        if response.success {
            conn.update_session_params_cache(
                sql,
                response.data.parameters.as_ref(),
                &FinalSessionNames {
                    database: response.data.final_database_name.clone(),
                    schema: response.data.final_schema_name.clone(),
                    warehouse: response.data.final_warehouse_name.clone(),
                    role: response.data.final_role_name.clone(),
                },
            )
            .await;
        };

        Ok(())
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
                    let mut client_info =
                        ClientInfo::from_settings(&resolved).context(ConfigurationSnafu)?;
                    client_info.platforms = self.platforms().await.clone();
                    client_info.os_details = self.os_details().cloned();
                    let init_params = conn.init_session_parameters.clone();
                    let resolved_snapshot = resolved.clone();

                    // Forward unrecognized settings as session parameters so
                    // drivers can set arbitrary Snowflake session params
                    // via regular connection options.
                    let mut unknown_settings = collect_unknown_settings(&conn.connection_seed);
                    // `CLIENT_SESSION_KEEP_ALIVE` and
                    // `CLIENT_SESSION_KEEP_ALIVE_HEARTBEAT_FREQUENCY` are registered
                    // params so they don't show up as "unknown"; mirror them into
                    // login session parameters to match the Python connector.
                    if let Some(v) = resolved.get_bool(param_names::CLIENT_SESSION_KEEP_ALIVE) {
                        unknown_settings.insert(
                            param_names::CLIENT_SESSION_KEEP_ALIVE.as_str().to_string(),
                            v.to_string(),
                        );
                    }
                    if let Some(v) =
                        resolved.get_int(param_names::CLIENT_SESSION_KEEP_ALIVE_HEARTBEAT_FREQUENCY)
                    {
                        unknown_settings.insert(
                            param_names::CLIENT_SESSION_KEEP_ALIVE_HEARTBEAT_FREQUENCY
                                .as_str()
                                .to_string(),
                            v.to_string(),
                        );
                    }
                    if resolved
                        .get_bool(param_names::VALIDATE_DEFAULT_PARAMETERS)
                        .unwrap_or(false)
                    {
                        unknown_settings.insert(
                            "CLIENT_VALIDATE_DEFAULT_PARAMETERS".to_string(),
                            "true".to_string(),
                        );
                    }
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
                let login_parameters = LoginParameters::from_connection_config(
                    &config,
                    client_info,
                    None,
                    read_spcs_token(self.fs_adapter().as_ref()),
                );

                let token_caching_requested = matches!(
                    &login_parameters.login_method,
                    LoginMethod::UserPasswordMfa {
                        client_store_temporary_credential: true,
                        ..
                    } | LoginMethod::ExternalBrowser {
                        client_store_temporary_credential: true,
                        ..
                    }
                );
                // OAuth Authorization Code uses the same token cache to
                // (a) short-circuit the interactive flow with a cached
                // access token, (b) exchange a cached refresh token, and
                // (c) drive 390303/390318 refresh-on-failure eviction
                // (AC state machine: cache → refresh → interactive;
                // cross-driver eviction on 390303/390318). Client
                // Credentials intentionally never persists tokens
                // (CC stateless by design) and the legacy pre-acquired
                // `OAUTH` flow forwards a caller-supplied token, so
                // neither needs the cache wired here.
                let oauth_caching_requested = matches!(
                    &login_parameters.login_method,
                    LoginMethod::OAuthAuthorizationCode(cfg)
                        if cfg.client_store_temporary_credential
                );

                let token_cache = if token_caching_requested || oauth_caching_requested {
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
                let login_server_version = login_result.server_version;

                // `CLIENT_SESSION_KEEP_ALIVE` and
                // `CLIENT_SESSION_KEEP_ALIVE_HEARTBEAT_FREQUENCY` are read from
                // `merged_params` so the server-echoed value (or a server-side
                // default) takes effect even when the client did not pass them
                // explicitly. The client-mirrored values are already merged in
                // above (init_params + login_result.session_parameters).
                let keep_alive = merged_params
                    .get(param_names::CLIENT_SESSION_KEEP_ALIVE.as_str())
                    .map(|v| v.eq_ignore_ascii_case("true"))
                    .unwrap_or(false);
                let heartbeat_frequency_secs = merged_params
                    .get(param_names::CLIENT_SESSION_KEEP_ALIVE_HEARTBEAT_FREQUENCY.as_str())
                    .and_then(|v| v.parse::<u64>().ok());

                {
                    let logout_config = LogoutConfig::from_settings(&resolved_snapshot)
                        .context(ConfigurationSnafu)?;
                    let session_id = login_result.tokens.session_id;
                    let mut conn = conn_ptr.lock().await;

                    conn.initialize(
                        login_result.tokens,
                        http_client,
                        host,
                        port,
                        login_parameters.server_url.clone(),
                        login_parameters.client_info.clone(),
                        merged_params,
                        login_final_names,
                        login_server_version,
                        resolved_snapshot,
                        logout_config,
                    )
                    .await;

                    // Cache the session id on the Connection so entry-point
                    // methods can stamp `snowflake.session.id` on their spans
                    // by reading the field under the same mutex they already
                    // take to do their work.
                    conn.session_id = Some(session_id);

                    // Telemetry setup: check if the server has opted this session
                    // into in-band telemetry and a session registry is configured,
                    // then register the session so spans tagged with this
                    // session_id are routed to /telemetry/send.
                    let telemetry_enabled = self.telemetry_sessions().is_some()
                        && conn
                            .session_parameters
                            .read()
                            .await
                            .get(param_names::CLIENT_TELEMETRY_ENABLED.as_str())
                            .map(|v| v.eq_ignore_ascii_case("true"))
                            .unwrap_or(true);

                    if telemetry_enabled {
                        use crate::telemetry::snowflake_exporter::ExporterSession;

                        let Some(http_client) = conn.http_client.clone() else {
                            tracing::warn!(
                                "Skipping telemetry: http_client not set after connection init"
                            );
                            drop(conn);
                            return Ok(());
                        };

                        let query_parameters = conn.query_transport_parameters()?;
                        let exporter_session = Arc::new(ExporterSession {
                            client: http_client,
                            query_parameters,
                            session_token: conn.tokens.clone(),
                        });

                        // unwrap is safe: telemetry_enabled is only true when
                        // telemetry_sessions() is Some.
                        self.telemetry_sessions()
                            .unwrap()
                            .write()
                            .unwrap_or_else(|e| e.into_inner())
                            .insert(session_id, exporter_session);

                        let env_info = conn
                            .wrapper_identity
                            .as_ref()
                            .map(crate::telemetry::environment::EnvironmentInfo::with_wrapper)
                            .unwrap_or_else(crate::telemetry::environment::EnvironmentInfo::detect);
                        // Bounded `session_init` span: an event-carrier span whose
                        // sole purpose is to attach `snowflake.session.id` to the
                        // session_init OTEL event. It does NOT measure connection_init
                        // duration — login has already completed by the time this span
                        // opens. Each subsequent driver operation emits its own bounded
                        // span tagged with the same session id; they share no trace_id
                        // with this span.
                        // TODO: instrument the surrounding connection_init future so we
                        // capture login duration + Status::Error on failure.
                        let init_span = crate::snowflake_op_span!("session_init", Some(session_id));
                        let _guard = init_span.enter();
                        crate::telemetry::record_session_init(&env_info);
                    }

                    if keep_alive {
                        let interval = compute_heartbeat_interval(
                            conn.tokens
                                .read()
                                .await
                                .as_ref()
                                .and_then(|t| t.master_validity),
                            heartbeat_frequency_secs,
                        );
                        let handle = spawn_heartbeat_task(
                            conn.tokens.clone(),
                            conn.http_client
                                .clone()
                                .context(ConnectionNotInitializedSnafu)?,
                            conn.server_url
                                .clone()
                                .context(ConnectionNotInitializedSnafu)?,
                            conn.client_info
                                .clone()
                                .context(ConnectionNotInitializedSnafu)?,
                            interval,
                        );
                        conn.heartbeat_handle = Some(handle);
                    }
                }
                Ok(())
            }
            None => InvalidArgumentSnafu {
                argument: "Connection handle not found".to_string(),
            }
            .fail(),
        }
    }

    /// Flush all telemetry for this connection and clean up the session registry.
    ///
    /// Order matters:
    /// 1. Flush per-session buffered spans — awaited export to `/telemetry/send`
    ///    while session tokens are still alive for authentication.
    /// 2. Remove the session from the exporter registry so it no longer resolves.
    ///
    /// Must be called while session tokens are still alive (before logout).
    /// Idempotent. Each driver operation emits a bounded span that ends when the
    /// operation returns, so there is no long-lived parent span to drop here.
    pub async fn flush_connection_telemetry(&self, conn_handle: Handle) {
        let session_id = self.session_id_for_conn(conn_handle).await;

        if let Some(id) = session_id {
            self.flush_telemetry_session(id).await;

            if let Some(sessions) = self.telemetry_sessions() {
                sessions
                    .write()
                    .unwrap_or_else(|e| e.into_inner())
                    .remove(&id);
            }
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
        let mut conn = Connection::new();
        self.seed_log_defaults_into(&mut conn.connection_seed);
        self.connections.add_handle(Mutex::new(conn))
    }

    /// Seed process-wide defaults sourced from the `LogManager`
    /// (parsed from `sf.odbc.ini` / `[log]` TOML). Explicit per-connection
    /// settings still win because they are inserted unconditionally on top.
    fn seed_log_defaults_into(&self, seed: &mut ParamStore) {
        if let Some(v) = self.log_query_text() {
            inject_if_absent(seed, param_names::LOG_QUERY_TEXT.as_str(), Setting::Bool(v));
        }
        if let Some(v) = self.log_query_parameters() {
            inject_if_absent(
                seed,
                param_names::LOG_QUERY_PARAMETERS.as_str(),
                Setting::Bool(v),
            );
        }
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

    /// Store wrapper identity on a connection. Called once from `ConnectionInit`.
    ///
    /// In addition to storing the identity for telemetry, this injects the
    /// identity fields into the connection seed so that [`ClientInfo::from_settings`]
    /// picks them up during login. Seed entries set earlier by the wrapper
    /// (e.g. the user's `application` kwarg → `client_app_id`) are preserved
    /// because we only insert when the key is absent.
    pub async fn set_wrapper_identity(
        &self,
        conn_handle: Handle,
        identity: WrapperIdentity,
    ) -> Result<(), ApiError> {
        match self.connections.get_obj(conn_handle) {
            Some(conn_ptr) => {
                let mut conn = conn_ptr.lock().await;
                if conn.wrapper_identity.is_some() {
                    tracing::warn!(
                        "Wrapper identity set more than once on the same connection; overwriting previous identity"
                    );
                }

                // Inject identity fields into the connection seed so
                // ClientInfo::from_settings picks them up at login time.
                // Only insert when the key is absent to respect values the
                // wrapper already set via connection_set_option.
                inject_if_absent(
                    &mut conn.connection_seed,
                    "client_app_id",
                    Setting::String(identity.driver_name.clone()),
                );
                inject_if_absent(
                    &mut conn.connection_seed,
                    "client_app_version",
                    Setting::String(identity.driver_version.clone()),
                );
                inject_if_absent(
                    &mut conn.connection_seed,
                    "client_runtime_name",
                    Setting::String(identity.language_runtime.clone()),
                );
                inject_if_absent(
                    &mut conn.connection_seed,
                    "client_runtime_version",
                    Setting::String(identity.language_version.clone()),
                );
                if let Some(compiler) = identity.language_compiler.clone() {
                    inject_if_absent(
                        &mut conn.connection_seed,
                        "client_compiler",
                        Setting::String(compiler),
                    );
                }

                conn.wrapper_identity = Some(identity);
                Ok(())
            }
            None => InvalidArgumentSnafu {
                argument: "Invalid connection handle".to_string(),
            }
            .fail(),
        }
    }

    /// Read the stored wrapper identity for a connection, if set during `ConnectionInit`.
    pub async fn get_wrapper_identity(
        &self,
        conn_handle: Handle,
    ) -> Result<Option<WrapperIdentity>, ApiError> {
        match self.connections.get_obj(conn_handle) {
            Some(conn_ptr) => {
                let conn = conn_ptr.lock().await;
                Ok(conn.wrapper_identity.clone())
            }
            None => InvalidArgumentSnafu {
                argument: "Invalid connection handle".to_string(),
            }
            .fail(),
        }
    }
}

/// Insert a value into the seed only when the key is absent. For
/// `Setting::String`, the value is trimmed first and skipped when empty;
/// other variants are inserted as-is. Used both for wrapper-identity seeding
/// and for process-wide defaults parsed from `sf.odbc.ini` / `[log]` TOML.
fn inject_if_absent(seed: &mut ParamStore, key: &str, value: Setting) {
    let value = match value {
        Setting::String(s) => {
            let trimmed = s.trim();
            if trimmed.is_empty() {
                return;
            }
            if trimmed.len() == s.len() {
                Setting::String(s)
            } else {
                Setting::String(trimmed.to_owned())
            }
        }
        other => other,
    };
    if seed.get_any(key).is_none() {
        seed.insert(key.to_owned(), value);
    }
}

/// Wrapper identity set once via `ConnectionInit` and attached to all subsequent telemetry events.
#[derive(Debug, Clone, Default)]
pub struct WrapperIdentity {
    pub driver_name: String,
    pub driver_version: String,
    pub language_runtime: String,
    pub language_version: String,
    /// `None` means compiler info is not applicable for this language.
    pub language_compiler: Option<String>,
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
    /// Registry for tracking async queries (for Fire & Forget auto-detection)
    pub async_query_registry: AsyncQueryRegistry,
    /// Flag indicating if connection has been closed.
    ///
    /// `Arc<AtomicBool>` is used (rather than a `Mutex<State>` state machine) because:
    /// - `Arc` provides shared ownership across concurrent async tasks — the logout task,
    ///   token refresh, and query cancellation all need to observe the closed state.
    /// - `AtomicBool` allows lock-free closed-state checks without async lock acquisition.
    ///
    /// A state machine (`Open → Closing → Closed`) would require a single owner or
    /// message-passing, conflicting with the parallel-task architecture.
    pub is_closed: Arc<AtomicBool>,

    /// Logout configuration (set via ConnectionSetOption* before init, parsed at init time)
    pub logout_config: LogoutConfig,
    /// Server-echoed final names from login and query responses (e.g. after USE DATABASE).
    /// Stored separately from session_parameters to keep concerns distinct.
    pub final_session_names: RwLock<FinalSessionNames>,
    /// Snowflake server version reported in the login response (e.g. "9.34.0").
    /// Read by ODBC `SQLGetInfo(SQL_DBMS_VER)` and the equivalent JDBC
    /// `getDatabaseProductVersion`. `None` until login completes.
    pub server_version: RwLock<Option<String>>,
    /// Wrapper identity for telemetry, set once via ConnectionInit.
    pub wrapper_identity: Option<WrapperIdentity>,
    /// Snowflake session id, populated from `tokens.session_id` once
    /// `connection_init` succeeds. Read by entry-point methods (under the
    /// connection mutex) to stamp `snowflake.session.id` on telemetry spans.
    /// `None` until login completes; cleared on connection release.
    pub(crate) session_id: Option<i64>,
    /// Handle to the per-connection heartbeat background task (if keep-alive is enabled).
    pub(crate) heartbeat_handle: Option<HeartbeatHandle>,
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
            async_query_registry: AsyncQueryRegistry::new(),
            is_closed: Arc::new(AtomicBool::new(false)),
            logout_config: LogoutConfig::default(),
            final_session_names: RwLock::new(FinalSessionNames::default()),
            server_version: RwLock::new(None),
            wrapper_identity: None,
            session_id: None,
            heartbeat_handle: None,
        }
    }

    /// `true` after a successful [`Connection::initialize`] (post-login transport is ready).
    pub(crate) fn is_post_connect(&self) -> bool {
        self.http_client.is_some()
    }

    /// Resolves the `ENABLE_STAGE_S3_PRIVATELINK_FOR_US_EAST_1` session
    /// parameter to a boolean by briefly read-locking the parameter cache.
    /// Used by the PUT/GET dispatch path to OR into the S3 regional-URL
    /// decision. See `read_use_s3_regional_url_session_param`.
    pub(crate) async fn use_s3_regional_url_session_param(&self) -> bool {
        let params = self.session_parameters.read().await;
        crate::rest::snowflake::query_response::read_use_s3_regional_url_session_param(&params)
    }

    /// Server URL + client fingerprint for query and refresh calls (transport snapshot).
    pub(crate) fn query_transport_parameters(&self) -> Result<QueryParameters, ApiError> {
        let empty = ParamStore::new();
        let settings = self.resolved_connect.as_ref().unwrap_or(&empty);

        Ok(QueryParameters {
            server_url: self
                .server_url
                .clone()
                .context(ConnectionNotInitializedSnafu)?,
            client_info: self
                .client_info
                .clone()
                .context(ConnectionNotInitializedSnafu)?,
            log_max_query_length: resolve_log_max_query_length(settings),
            log_query_text: resolve_log_query_text(settings),
            log_query_parameters: resolve_log_query_parameters(settings),
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
        server_version: Option<String>,
        resolved_connect: ParamStore,
        logout_config: LogoutConfig,
    ) {
        *self.tokens.write().await = Some(tokens);
        self.http_client = Some(http_client);
        self.host = host;
        self.port = port;
        self.server_url = Some(server_url);
        self.client_info = Some(client_info);
        self.resolved_connect = Some(resolved_connect);
        self.session_overrides = ParamStore::new();
        self.logout_config = logout_config;

        let mut cache = self.session_parameters.write().await;
        *cache = session_params;
        drop(cache);

        if let Ok(mut names) = self.final_session_names.write() {
            *names = final_names;
        }

        if let Ok(mut version) = self.server_version.write() {
            *version = server_version;
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
    ctx.execute_with_refresh(f).await
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
    /// Create a `RefreshContext` for a query operation.
    ///
    /// Rejects context creation if the connection is already closed, preventing
    /// in-flight queries from performing token refreshes after `close()` is called.
    /// This is safe because the query's early `is_closed` check already rejected
    /// the operation; this provides a second gate for races between that check and
    /// the token refresh setup.
    pub async fn from_arc(conn: &Arc<Mutex<Connection>>) -> Result<Self, ApiError> {
        let guard = conn.lock().await;
        if guard.is_closed.load(Ordering::SeqCst) {
            return Err(ConnectionClosedSnafu {}.build());
        }
        Self::new(&guard)
    }

    /// Create a `RefreshContext` from individual components (no `Connection` needed).
    pub fn from_parts(
        tokens_lock: Arc<AsyncRwLock<Option<SessionTokens>>>,
        http_client: reqwest::Client,
        server_url: String,
        client_info: ClientInfo,
    ) -> Self {
        Self {
            tokens_lock,
            http_client,
            server_url,
            client_info,
            state: RefreshState::Initial,
        }
    }

    /// Create a new `RefreshContext` by extracting connection info.
    ///
    /// Does **not** check `is_closed`. Use `from_arc` for query paths (which
    /// must reject creation on a closed connection). The logout path calls
    /// `new` directly because logout itself runs after `is_closed` is set.
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

    /// Execute an async operation with automatic token refresh on session expiry.
    ///
    /// On `SessionExpired`, refreshes the session token via master token and
    /// retries exactly once. Implemented on top of the generic
    /// [`refresh::execute_with_refresh`](crate::refresh::execute_with_refresh)
    /// helper. Named to parallel
    /// [`execute_with_retry`](crate::http::retry::execute_with_retry).
    pub async fn execute_with_refresh<F, Fut, T>(&mut self, f: F) -> Result<T, ApiError>
    where
        F: Fn(SensitiveString) -> Fut,
        Fut: Future<Output = Result<T, RestError>>,
    {
        use snafu::IntoError;
        crate::refresh::execute_with_refresh(self, |token| {
            let fut = f(token);
            async move { fut.await.map_err(|e| QuerySnafu.into_error(e)) }
        })
        .await
    }
}

impl crate::refresh::Refresher<SensitiveString, ApiError> for RefreshContext {
    fn current(&mut self) -> crate::refresh::RefreshFuture<'_, Result<SensitiveString, ApiError>> {
        Box::pin(async move {
            let tokens_guard = self.tokens_lock.read().await;
            let token = tokens_guard
                .as_ref()
                .map(|t| t.session_token.clone())
                .context(ConnectionNotInitializedSnafu)?;
            // Stamp the token into the state machine on first read so
            // that `refresh()` can detect — by comparing this token
            // against the cache after acquiring the write lock — whether
            // another request already rotated while we were waiting.
            if matches!(self.state, RefreshState::Initial) {
                self.state = RefreshState::FirstToken(token.clone());
            }
            Ok(token)
        })
    }

    fn should_refresh(&self, err: &ApiError) -> bool {
        let ApiError::Query { source, .. } = err else {
            return false;
        };
        matches!(
            source.as_ref(),
            RestError::InvalidSnowflakeResponse {
                source: SnowflakeResponseError::SessionExpired { .. },
                ..
            },
        )
    }

    fn refresh(&mut self) -> crate::refresh::RefreshFuture<'_, Result<bool, ApiError>> {
        Box::pin(async move {
            // One-shot state machine: the first refresh call does the
            // work, subsequent calls return Ok(false) so the generic
            // helper propagates the original error. `Initial` is
            // unreachable in practice — `execute_with_refresh` always
            // calls `current()` first, which transitions Initial →
            // FirstToken — but we keep a safe fallback rather than
            // panicking on a misuse from a future caller.
            let failed_token = match &self.state {
                RefreshState::FirstToken(t) => t.clone(),
                RefreshState::Refreshed => return Ok(false),
                RefreshState::Initial => {
                    debug_assert!(
                        false,
                        "RefreshContext::refresh() called before current(); \
                         execute_with_refresh always calls current() first"
                    );
                    return Ok(false);
                }
            };
            self.state = RefreshState::Refreshed;

            // Acquire the write lock — blocks other readers/writers during
            // refresh.
            let mut tokens_guard = self.tokens_lock.write().await;
            let tokens = tokens_guard
                .as_ref()
                .cloned()
                .context(ConnectionNotInitializedSnafu)?;

            // If another request already refreshed while we waited, the
            // current token is fresh — treat as a successful rotation
            // without hitting GS.
            if tokens.session_token.reveal() != failed_token.reveal() {
                tracing::debug!("Session already refreshed by another request");
                return Ok(true);
            }

            if tokens.is_master_expired() {
                tracing::error!("Master token expired, full re-authentication required");
                return MasterTokenExpiredSnafu.fail();
            }

            tracing::info!("Session expired, attempting refresh");
            let new_tokens = snowflake::refresh_session(
                &self.http_client,
                &self.server_url,
                &self.client_info,
                &tokens,
            )
            .await
            .context(SessionRefreshSnafu)?;

            *tokens_guard = Some(new_tokens);
            drop(tokens_guard);

            tracing::info!("Session refreshed, retrying operation");
            Ok(true)
        })
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

    pub async fn connection_get_all_parameters(
        &self,
        conn_handle: Handle,
    ) -> Result<HashMap<String, String>, ApiError> {
        match self.connections.get_obj(conn_handle) {
            Some(conn_ptr) => {
                let conn = conn_ptr.lock().await;
                let mut result = HashMap::new();

                for (k, v) in conn.connection_seed.iter() {
                    if let Some(s) = setting_as_display_string(v) {
                        result.insert(k.to_uppercase(), s);
                    }
                }
                if let Some(resolved) = &conn.resolved_connect {
                    for (k, v) in resolved.iter() {
                        if let Some(s) = setting_as_display_string(v) {
                            result.insert(k.to_uppercase(), s);
                        }
                    }
                }
                for (k, v) in conn.session_overrides.iter() {
                    if let Some(s) = setting_as_display_string(v) {
                        result.insert(k.to_uppercase(), s);
                    }
                }

                let cache = conn.session_parameters.read().await;
                for (k, v) in cache.iter() {
                    if !v.is_empty() {
                        result.insert(k.clone(), v.clone());
                    }
                }

                Ok(result)
            }
            None => InvalidArgumentSnafu {
                argument: "Connection handle not found".to_string(),
            }
            .fail(),
        }
    }

    /// Returns the Snowflake server version cached from the login response
    /// (`serverVersion` in `/session/v1/login-request`).
    ///
    /// Backs `SQLGetInfo(SQL_DBMS_VER)` in ODBC and
    /// `getDatabaseProductVersion` in JDBC. Kept separate from
    /// [`Self::connection_get_info`] so probing this single attribute
    /// (Excel does it during `SQLDriverConnect`) doesn't pay for the
    /// full info aggregation — token guard, session-parameters async lock,
    /// final-session-names lock, and the seed/resolved/override lookups
    /// for role/database/schema/warehouse.
    ///
    /// Returns `Ok(None)` if the connection has not completed login yet,
    /// so callers can render the attribute as an empty string instead of
    /// failing the surrounding ODBC/JDBC call.
    pub async fn connection_get_server_version(
        &self,
        conn_handle: Handle,
    ) -> Result<Option<String>, ApiError> {
        match self.connections.get_obj(conn_handle) {
            Some(conn_ptr) => {
                let conn = conn_ptr.lock().await;
                let version = conn
                    .server_version
                    .read()
                    .map_err(|_| ConnectionLockingSnafu {}.build())?
                    .clone();
                Ok(version)
            }
            None => InvalidArgumentSnafu {
                argument: "Connection handle not found".to_string(),
            }
            .fail(),
        }
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

    /// Send a heartbeat to validate that the connection and session are still alive.
    ///
    /// Returns `true` if the session is valid, `false` otherwise.
    /// Automatically attempts one token refresh on 401 (session expired).
    pub async fn connection_heartbeat(&self, conn_handle: Handle) -> Result<bool, ApiError> {
        self.connection_heartbeat_with_timeout(conn_handle, None)
            .await
    }

    /// Like [`Self::connection_heartbeat`] but with an optional HTTP request timeout override.
    pub async fn connection_heartbeat_with_timeout(
        &self,
        conn_handle: Handle,
        timeout: Option<Duration>,
    ) -> Result<bool, ApiError> {
        let session_id = self.session_id_for_conn(conn_handle).await;
        async {
            let conn_ptr = self
                .connections
                .get_obj(conn_handle)
                .context(InvalidArgumentSnafu {
                    argument: "Connection handle not found".to_string(),
                })?;

            let mut ctx = match RefreshContext::from_arc(&conn_ptr).await {
                Ok(ctx) => ctx,
                Err(_) => return Ok(false),
            };

            let server_url = match url::Url::parse(&ctx.server_url) {
                Ok(u) => u,
                Err(_) => return Ok(false),
            };

            let mut last_error: Option<RestError> = None;

            loop {
                let token = match ctx.refresh_token(last_error.take()).await {
                    Ok(t) => t,
                    Err(_) => return Ok(false),
                };

                match heartbeat::send_heartbeat_with_timeout(
                    &ctx.http_client,
                    &server_url,
                    &ctx.client_info,
                    token.reveal(),
                    timeout,
                )
                .await
                {
                    Ok(()) => return Ok(true),
                    Err(
                        e @ RestError::InvalidSnowflakeResponse {
                            source: SnowflakeResponseError::SessionExpired { .. },
                            ..
                        },
                    ) => {
                        last_error = Some(e);
                    }
                    Err(_) => return Ok(false),
                }
            }
        }
        .instrument(crate::snowflake_op_span!(
            "connection_heartbeat",
            session_id
        ))
        .await
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

impl DatabaseDriverV1 {
    /// Check if a connection has been closed.
    ///
    /// Returns true if close() has been called, false otherwise.
    pub async fn connection_is_closed(&self, conn_handle: Handle) -> Result<bool, ApiError> {
        let conn_ptr = self
            .connections
            .get_obj(conn_handle)
            .context(InvalidArgumentSnafu {
                argument: "Connection handle not found".to_string(),
            })?;

        let conn = conn_ptr.lock().await;
        Ok(conn.is_closed.load(Ordering::SeqCst))
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
    /// - `enable_server_session_keep_alive_auto_detection`: Enable async query detection
    /// - `logout_error_strategy`: Error handling (Strict or BestEffort)
    /// - `logout_total_timeout`: Total timeout budget
    /// - `logout_max_attempts`: Maximum total attempts (1 = no retries, 3 = 2 retries)
    /// - `logout_request_timeout`: Per-request timeout
    ///
    /// This design matches all existing Snowflake drivers (Python, Go, JDBC, .NET, Node.js)
    /// which configure logout behavior at connection initialization, not at close time.
    pub async fn connection_close(&self, conn_handle: Handle) -> Result<(), ApiError> {
        let conn_ptr = self
            .connections
            .get_obj(conn_handle)
            .context(InvalidArgumentSnafu {
                argument: "Connection handle not found".to_string(),
            })?;

        // Single lock acquisition for all pre-network synchronous work:
        // atomically mark closed, capture config, and extract logout data.
        // Holding the lock here prevents a concurrent set_option_int from racing
        // between the "mark closed" and "read config" steps.
        let prepare_result = {
            let conn = conn_ptr.lock().await;

            // Atomic swap returns true if connection was already closed (idempotent guard)
            let was_already_closed = conn.is_closed.swap(true, Ordering::SeqCst);
            if was_already_closed {
                tracing::debug!("Connection already closed, skipping duplicate close");
                // Return None to signal early exit after the block
                None
            } else {
                // Re-derive logout config from connection_seed at close()-time so
                // post-init overrides (e.g. retry=False sets logout_max_attempts=1)
                // take effect. Falls back to init-time defaults for unset keys.
                let config = LogoutConfig::from_settings(&conn.connection_seed)
                    .unwrap_or_else(|e| {
                        tracing::warn!(error = %e, "Failed to re-derive LogoutConfig at close-time; using init-time config");
                        conn.logout_config.clone()
                    });
                let error_strategy = config.error_strategy;

                // TODO: SNOW-2912513 - Record telemetry for logout decision

                // Prepare logout data while holding the lock (pure reads, no network I/O)
                let logout_data = logout::prepare_logout_from_conn(&conn, &config)?;

                Some((logout_data, error_strategy))
            }
        };

        let Some((logout_data, error_strategy)) = prepare_result else {
            return Ok(());
        };

        // Flush telemetry before logout — session tokens are still alive and
        // telemetry records the connection's lifetime events (session_init, api_usage).
        self.flush_connection_telemetry(conn_handle).await;

        // Execute logout — network I/O, lock must not be held
        let logout_result = logout::execute_logout_with_strategy(logout_data, error_strategy).await;

        // Cleanup connection resources (separate lock acquisition after I/O)
        cleanup_connection(&conn_ptr).await?;

        if logout_result.is_ok() {
            tracing::info!("Connection closed successfully");
        }
        logout_result
    }
}

/// Clear tokens, HTTP client, and stop background tasks.
async fn cleanup_connection(conn_ptr: &Arc<Mutex<Connection>>) -> Result<(), ApiError> {
    let mut conn = conn_ptr.lock().await;
    if let Some(mut hb) = conn.heartbeat_handle.take() {
        hb.cancel_and_wait().await;
    }
    *conn.tokens.write().await = None;
    conn.http_client = None;
    tracing::debug!("Cleared session tokens and HTTP client");

    // Telemetry is flushed before logout in connection_close (flush_connection_telemetry).
    // TODO: Implement QCC (query result cache) clearing

    Ok(())
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
    async fn connection_new_seeds_log_query_defaults_from_log_manager() {
        use crate::apis::database_driver_v1::global_state::DriverProviders;
        use crate::fs_adapter::RealFs;
        use crate::logging::LogManager;
        use std::sync::Arc;

        let lm = LogManager::with_none_subscriber(Arc::new(RealFs))
            .with_query_log_defaults(Some(true), Some(false));
        let ds = DatabaseDriverV1::with_providers(DriverProviders {
            log_manager: Some(lm),
            ..Default::default()
        });

        let handle = ds.connection_new();
        let conn_ptr = ds.connections.get_obj(handle).unwrap();
        let conn = conn_ptr.lock().await;
        assert_eq!(
            conn.connection_seed.get_bool(param_names::LOG_QUERY_TEXT),
            Some(true)
        );
        assert_eq!(
            conn.connection_seed
                .get_bool(param_names::LOG_QUERY_PARAMETERS),
            Some(false)
        );
        drop(conn);
        ds.connection_release(handle).unwrap();
    }

    #[tokio::test]
    async fn connection_new_does_not_seed_when_log_manager_absent() {
        let ds = DatabaseDriverV1::new();
        let handle = ds.connection_new();
        let conn_ptr = ds.connections.get_obj(handle).unwrap();
        let conn = conn_ptr.lock().await;
        assert_eq!(
            conn.connection_seed.get_bool(param_names::LOG_QUERY_TEXT),
            None
        );
        assert_eq!(
            conn.connection_seed
                .get_bool(param_names::LOG_QUERY_PARAMETERS),
            None
        );
        drop(conn);
        ds.connection_release(handle).unwrap();
    }

    #[tokio::test]
    async fn connection_set_option_overrides_log_query_seed() {
        use crate::apis::database_driver_v1::global_state::DriverProviders;
        use crate::fs_adapter::RealFs;
        use crate::logging::LogManager;
        use std::sync::Arc;

        let lm = LogManager::with_none_subscriber(Arc::new(RealFs))
            .with_query_log_defaults(Some(true), Some(true));
        let ds = DatabaseDriverV1::with_providers(DriverProviders {
            log_manager: Some(lm),
            ..Default::default()
        });

        let handle = ds.connection_new();
        ds.connection_set_option(handle, "log_query_text".into(), Setting::Bool(false))
            .await
            .unwrap();

        let conn_ptr = ds.connections.get_obj(handle).unwrap();
        let conn = conn_ptr.lock().await;
        assert_eq!(
            conn.connection_seed.get_bool(param_names::LOG_QUERY_TEXT),
            Some(false),
            "explicit option must override the ini-derived seed default"
        );
        assert_eq!(
            conn.connection_seed
                .get_bool(param_names::LOG_QUERY_PARAMETERS),
            Some(true),
            "untouched seed default should still be honored"
        );
        drop(conn);
        ds.connection_release(handle).unwrap();
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

    /// On a pre-login connection the cached server version must surface as
    /// `None` so callers can render `SQL_DBMS_VER` / `getDatabaseProductVersion`
    /// as empty rather than failing the surrounding ODBC/JDBC call.
    #[tokio::test]
    async fn connection_get_server_version_returns_none_before_login() {
        let ds = DatabaseDriverV1::new();
        let handle = ds.connection_new();

        let version = ds.connection_get_server_version(handle).await.unwrap();

        assert_eq!(version, None);

        ds.connection_release(handle).unwrap();
    }

    /// Whatever the login response wrote into `Connection.server_version`
    /// must round-trip through `connection_get_server_version` unchanged.
    /// This is the invariant that backs `SQLGetInfo(SQL_DBMS_VER)` in ODBC
    /// and `getDatabaseProductVersion` in JDBC.
    #[tokio::test]
    async fn connection_get_server_version_returns_cached_value() {
        let ds = DatabaseDriverV1::new();
        let handle = ds.connection_new();

        if let Some(conn_ptr) = ds.connections.get_obj(handle) {
            let conn = conn_ptr.lock().await;
            *conn.server_version.write().unwrap() = Some("9.34.0".into());
        }

        let version = ds.connection_get_server_version(handle).await.unwrap();

        assert_eq!(version, Some("9.34.0".into()));

        ds.connection_release(handle).unwrap();
    }

    /// Calling the getter on a released handle must produce a structured
    /// `invalid_argument`, mirroring how the other connection-scoped
    /// getters fail. Guards against accidental panics from `unwrap()` if
    /// someone re-wires `connections.get_obj` later.
    #[tokio::test]
    async fn connection_get_server_version_invalid_handle_returns_error() {
        let ds = DatabaseDriverV1::new();
        let handle = ds.connection_new();
        ds.connection_release(handle).unwrap();

        let err = ds
            .connection_get_server_version(handle)
            .await
            .expect_err("released handle must not succeed");
        assert!(
            matches!(err, ApiError::InvalidArgument { .. }),
            "expected InvalidArgument, got {err:?}"
        );
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
                master_validity: None,
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

    async fn setup_connection_for_heartbeat_tests(
        ds: &DatabaseDriverV1,
        server_url: &str,
    ) -> Handle {
        let handle = ds.connection_new();
        if let Some(c) = ds.connections.get_obj(handle) {
            let mut conn = c.lock().await;
            conn.http_client = Some(reqwest::Client::new());
            conn.server_url = Some(server_url.to_string());
            conn.client_info =
                Some(crate::config::rest_parameters::test_fixtures::test_client_info());
            let tokens = SessionTokens {
                session_token: "test-session-token".into(),
                master_token: "test-master-token".into(),
                session_id: 1,
                session_expires_at: None,
                master_expires_at: Some(
                    std::time::Instant::now() + std::time::Duration::from_secs(14400),
                ),
                master_validity: Some(std::time::Duration::from_secs(14400)),
            };
            *conn.tokens.write().await = Some(tokens);
        }
        handle
    }

    #[tokio::test]
    async fn heartbeat_returns_true_on_success() {
        use wiremock::matchers::{method, path};
        use wiremock::{Mock, MockServer, ResponseTemplate};

        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/session/heartbeat"))
            .respond_with(
                ResponseTemplate::new(200).set_body_json(serde_json::json!({"success": true})),
            )
            .mount(&server)
            .await;

        let ds = DatabaseDriverV1::new();
        let handle = setup_connection_for_heartbeat_tests(&ds, &server.uri()).await;

        let valid = ds.connection_heartbeat(handle).await.unwrap();
        assert!(valid, "heartbeat should return true on success");

        ds.connection_release(handle).unwrap();
    }

    #[tokio::test]
    async fn heartbeat_returns_false_on_failure_response() {
        use wiremock::matchers::{method, path};
        use wiremock::{Mock, MockServer, ResponseTemplate};

        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/session/heartbeat"))
            .respond_with(ResponseTemplate::new(200).set_body_json(
                serde_json::json!({"success": false, "message": "Heartbeat failed", "code": "390100"}),
            ))
            .mount(&server)
            .await;

        let ds = DatabaseDriverV1::new();
        let handle = setup_connection_for_heartbeat_tests(&ds, &server.uri()).await;

        let valid = ds.connection_heartbeat(handle).await.unwrap();
        assert!(!valid, "heartbeat should return false on failure response");

        ds.connection_release(handle).unwrap();
    }

    #[tokio::test]
    async fn heartbeat_returns_false_on_network_error() {
        // Bind to an ephemeral port, capture the address, then close the listener
        // so the port is guaranteed to refuse connections.
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap();
        drop(listener);

        let ds = DatabaseDriverV1::new();
        let handle = setup_connection_for_heartbeat_tests(&ds, &format!("http://{addr}")).await;

        let valid = ds.connection_heartbeat(handle).await.unwrap();
        assert!(!valid, "heartbeat should return false on network error");

        ds.connection_release(handle).unwrap();
    }

    #[tokio::test]
    async fn heartbeat_returns_false_when_not_initialized() {
        let ds = DatabaseDriverV1::new();
        let handle = ds.connection_new();

        let valid = ds.connection_heartbeat(handle).await.unwrap();
        assert!(
            !valid,
            "heartbeat should return false on uninitialized connection"
        );

        ds.connection_release(handle).unwrap();
    }

    #[tokio::test]
    async fn connection_get_all_parameters_returns_empty_by_default() {
        let ds = DatabaseDriverV1::new();
        let handle = ds.connection_new();

        let params = ds.connection_get_all_parameters(handle).await.unwrap();
        assert!(params.is_empty());

        ds.connection_release(handle).unwrap();
    }

    #[tokio::test]
    async fn connection_get_all_parameters_returns_cached_values() {
        let ds = DatabaseDriverV1::new();
        let handle = ds.connection_new();

        if let Some(c) = ds.connections.get_obj(handle) {
            let conn = c.lock().await;
            let mut cache = conn.session_parameters.write().await;
            cache.insert("TIMEZONE".into(), "America/Los_Angeles".into());
            cache.insert("QUERY_TAG".into(), "test_tag".into());
        }

        let params = ds.connection_get_all_parameters(handle).await.unwrap();
        assert_eq!(params.get("TIMEZONE").unwrap(), "America/Los_Angeles");
        assert_eq!(params.get("QUERY_TAG").unwrap(), "test_tag");
        assert_eq!(params.len(), 2);

        ds.connection_release(handle).unwrap();
    }
}
