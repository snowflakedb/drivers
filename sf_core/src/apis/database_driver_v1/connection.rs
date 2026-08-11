use snafu::{OptionExt, ResultExt};
use std::future::Future;
use std::sync::RwLock;
use std::sync::atomic::{AtomicBool, Ordering};

use crate::apis::operation_ctx::OperationCtx;
use crate::stage_binding::{AtomicStageState, StageState};
use std::time::Duration;
use std::{collections::HashMap, sync::Arc};
use tokio::sync::{Mutex, RwLock as AsyncRwLock};
use tracing::Instrument;

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
use super::{Setting, WrapperPresets};
use crate::config::{
    ParamStore,
    connection_config::{ConnectionConfig, DiagnosticConfig},
    logout::LogoutConfig,
    param_registry::{ParamKey, param_names},
    resolver,
    rest_parameters::{
        ClientInfo, LoginMethod, LoginParameters, QueryParameters, resolve_log_max_query_length,
        resolve_log_query_parameters, resolve_log_query_text,
    },
    retry::RetryPolicy,
    settings::Settings,
};
use crate::diagnostic::DiagnosticRunner;
use crate::handle_manager::Handle;
use crate::rest::snowflake::query_context_cache::QueryContextCacheAdapter;
use crate::rest::snowflake::{
    self, QueryInput, QueryOptions, RestError, SessionTokens, SnowflakeResponseError, heartbeat,
    query_request, snowflake_query_with_client,
};
use crate::sensitive::SensitiveString;
use crate::tls::config::ProxyConfig;

/// Whether `execute_session_sql` should refresh the connection's local
/// session-state cache (`session_parameters`, `final_session_names`) from
/// the response after a successful query.
#[derive(Copy, Clone, Debug)]
enum SessionStateRefresh {
    /// SQL may change session state (e.g. `ALTER SESSION SET`, `USE DATABASE`).
    Apply,
    /// SQL is non-stateful from the session's perspective (e.g. `COMMIT`,
    /// `ROLLBACK`).
    Skip,
}

/// Server session-parameter key for the JDBC PUT/GET disable switch. Read by
/// [`Connection::enable_put_get`]; upstream uppercases session-parameter names.
const JDBC_ENABLE_PUT_GET_SERVER_KEY: &str = "JDBC_ENABLE_PUT_GET";

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
                    self.execute_session_sql(&mut conn, sql, SessionStateRefresh::Apply)
                        .await
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
        let escaped = escape_sql_identifier(db);
        let sql = format!("USE DATABASE \"{escaped}\"");

        match self.connections.get_obj(conn_handle) {
            Some(conn_ptr) => {
                let mut conn = conn_ptr.lock().await;
                if !conn.is_post_connect() {
                    return InvalidArgumentSnafu {
                        argument: "connection_use_database called before connection is open"
                            .to_string(),
                    }
                    .fail();
                }
                self.execute_session_sql(&mut conn, &sql, SessionStateRefresh::Apply)
                    .await
            }
            None => InvalidArgumentSnafu {
                argument: "Connection handle not found".to_string(),
            }
            .fail(),
        }
    }

    /// Commit the open transaction on the given connection by executing `COMMIT`.
    /// Must only be called after the connection is initialised (`is_post_connect()`).
    pub async fn connection_commit(&self, conn_handle: Handle) -> Result<(), ApiError> {
        self.execute_transaction_control(conn_handle, "COMMIT")
            .await
    }

    /// Roll back the open transaction on the given connection by executing `ROLLBACK`.
    /// Must only be called after the connection is initialised (`is_post_connect()`).
    pub async fn connection_rollback(&self, conn_handle: Handle) -> Result<(), ApiError> {
        self.execute_transaction_control(conn_handle, "ROLLBACK")
            .await
    }

    /// Shared implementation for `connection_commit` / `connection_rollback`:
    /// run the transaction-control statement on the connection's session.
    async fn execute_transaction_control(
        &self,
        conn_handle: Handle,
        sql: &str,
    ) -> Result<(), ApiError> {
        match self.connections.get_obj(conn_handle) {
            Some(conn_ptr) => {
                let mut conn = conn_ptr.lock().await;
                if !conn.is_post_connect() {
                    return InvalidArgumentSnafu {
                        argument: format!("{sql} called before connection is open"),
                    }
                    .fail();
                }
                // COMMIT/ROLLBACK do not change session parameters or
                // current database/schema/warehouse/role, so skip the
                // session-state cache refresh.
                self.execute_session_sql(&mut conn, sql, SessionStateRefresh::Skip)
                    .await
            }
            None => InvalidArgumentSnafu {
                argument: "Connection handle not found".to_string(),
            }
            .fail(),
        }
    }

    /// Execute `USE SCHEMA "<name>"` or `USE SCHEMA "<db>"."<name>"` when a session database is set.
    /// Schema and database names are escaped (internal `"` doubled).
    /// Must only be called after the connection is initialised (`is_post_connect()`).
    pub async fn connection_use_schema(
        &self,
        conn_handle: Handle,
        schema: &str,
    ) -> Result<(), ApiError> {
        let schema = schema.trim();
        if schema.is_empty() {
            return InvalidArgumentSnafu {
                argument: "schema name must not be empty".to_string(),
            }
            .fail();
        }

        match self.connections.get_obj(conn_handle) {
            Some(conn_ptr) => {
                let mut conn = conn_ptr.lock().await;
                if !conn.is_post_connect() {
                    return InvalidArgumentSnafu {
                        argument: "connection_use_schema called before connection is open"
                            .to_string(),
                    }
                    .fail();
                }
                let database = resolve_session_database(&conn)?;
                let sql = build_use_schema_sql(database.as_deref(), schema);
                self.execute_session_sql(&mut conn, &sql, SessionStateRefresh::Apply)
                    .await
            }
            None => InvalidArgumentSnafu {
                argument: "Connection handle not found".to_string(),
            }
            .fail(),
        }
    }

    /// Execute a session-scoped SQL command without creating a statement.
    ///
    /// `refresh` controls whether the local session-state cache
    /// (`session_parameters`, `final_session_names`) is refreshed from the
    /// response after a successful query. Pass `Apply` for SQL that may
    /// change session state (e.g. `ALTER SESSION SET`, `USE DATABASE`); pass
    /// `Skip` for SQL that is non-stateful from the session's perspective
    /// (e.g. `COMMIT`, `ROLLBACK`) to avoid taking the cache write locks
    /// for a no-op merge.
    async fn execute_session_sql(
        &self,
        conn: &mut Connection,
        sql: &str,
        refresh: SessionStateRefresh,
    ) -> Result<(), ApiError> {
        let query_input = QueryInput::new(sql);
        let query_parameters = conn.query_transport_parameters()?;
        let http_client = conn
            .http_client
            .clone()
            .context(ConnectionNotInitializedSnafu)?;
        let retry_policy = conn.retry_policy.clone();

        let mut refresh_ctx = RefreshContext::new(conn)?;
        let mut last_error = None;
        let response = loop {
            let session_token = refresh_ctx.refresh_token(last_error).await?;
            match snowflake_query_with_client(
                &http_client,
                query_parameters.clone(),
                session_token.reveal(),
                query_input.clone(),
                QueryOptions {
                    retry_policy: retry_policy.clone(),
                    ..Default::default()
                },
            )
            .await
            {
                Ok(result) => break Ok(result),
                Err(e) => last_error = Some(e),
            }
        }?;

        conn.query_context_cache
            .update_query_context_cache(
                response.data.query_context.as_ref(),
                response.data.parameters.as_ref(),
            )
            .await;

        if response.success && matches!(refresh, SessionStateRefresh::Apply) {
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

    /// Establish the session for `conn_handle`.
    ///
    /// Options from `db_handle` form the base programmatic layer. Options set
    /// directly on the connection override database options before config-file
    /// and registry-default resolution.
    ///
    /// `operation_ctx` is the operation's cancellation context, or `None` when the caller
    /// reached core without an operation handle (a blocking FFI entry, an
    /// internal caller, a test) and therefore has no way to cancel. This is the
    /// single observation point for the operation: cancellation surfaces as
    /// [`ApiError::Cancelled`] and propagates like any other error.
    pub async fn connection_init(
        &self,
        operation_ctx: Option<&OperationCtx>,
        conn_handle: Handle,
        db_handle: Handle,
    ) -> Result<(), ApiError> {
        crate::apis::operation_ctx::run_opt(
            operation_ctx,
            "connection_init",
            Box::pin(self.connection_init_inner(conn_handle, db_handle)),
        )
        .await
    }

    async fn connection_init_inner(
        &self,
        conn_handle: Handle,
        db_handle: Handle,
    ) -> Result<(), ApiError> {
        match self.connections.get_obj(conn_handle) {
            Some(conn_ptr) => {
                let database_seed = self.database_settings(db_handle).await?;
                let (
                    config,
                    host,
                    port,
                    client_info,
                    init_params,
                    resolved_snapshot,
                    effective_seed,
                ) = {
                    let conn = conn_ptr.lock().await;
                    // TODO(sfc-gh-boler): Clone the mutable connection inputs under the mutex,
                    // then drop the lock before calling resolve/build. Those paths can do
                    // synchronous disk I/O and private-key parsing, which currently extends the
                    // connection mutex critical section and can block the async runtime thread.
                    let effective_seed = conn.effective_seed(&database_seed);
                    let mut resolved = resolver::resolve(
                        &effective_seed,
                        conn.no_connection_details && database_seed.is_empty(),
                    )
                    .context(ConfigurationSnafu)?;
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

                    // Session parameters to send in the login request. Seeded
                    // with unrecognized connection options (so drivers can set
                    // arbitrary Snowflake session params via regular connection
                    // options), then augmented below with registered session
                    // params that belong in SESSION_PARAMETERS
                    // (CLIENT_SESSION_KEEP_ALIVE, its heartbeat frequency,
                    // CLIENT_PREFETCH_THREADS, QUERY_TAG) to match the Python connector.
                    let mut login_session_params = collect_unknown_settings(&effective_seed);
                    if let Some(v) = resolved.get_bool(param_names::CLIENT_SESSION_KEEP_ALIVE) {
                        login_session_params.insert(
                            param_names::CLIENT_SESSION_KEEP_ALIVE.as_str().to_string(),
                            v.to_string(),
                        );
                    }
                    if let Some(v) =
                        resolved.get_int(param_names::CLIENT_SESSION_KEEP_ALIVE_HEARTBEAT_FREQUENCY)
                    {
                        login_session_params.insert(
                            param_names::CLIENT_SESSION_KEEP_ALIVE_HEARTBEAT_FREQUENCY
                                .as_str()
                                .to_string(),
                            v.to_string(),
                        );
                    }
                    if let Some(v) = resolved.get_int(param_names::CLIENT_PREFETCH_THREADS) {
                        login_session_params.insert(
                            param_names::CLIENT_PREFETCH_THREADS.as_str().to_string(),
                            v.to_string(),
                        );
                    }
                    if resolved
                        .get_bool(param_names::VALIDATE_DEFAULT_PARAMETERS)
                        .unwrap_or(false)
                    {
                        login_session_params.insert(
                            "CLIENT_VALIDATE_DEFAULT_PARAMETERS".to_string(),
                            "true".to_string(),
                        );
                    }
                    // QUERY_TAG is a registered session parameter (also
                    // statement-overridable), so it isn't "unknown"; mirror a
                    // connection-level value into the login session parameters.
                    if let Some(v) = resolved.get_string(param_names::QUERY_TAG) {
                        login_session_params.insert("QUERY_TAG".to_string(), v);
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
                            for (k, v) in login_session_params {
                                merged.entry(k).or_insert(v);
                            }
                            Some(merged)
                        }
                        None if !login_session_params.is_empty() => Some(login_session_params),
                        None => None,
                    };

                    (
                        config,
                        host,
                        port,
                        client_info,
                        init_params,
                        resolved_snapshot,
                        effective_seed,
                    )
                };

                warn_query_logging_risk(&resolved_snapshot);

                let timeout_config =
                    crate::config::retry::TimeoutConfig::from_params(&effective_seed);

                let (http_client, diag_rustls) =
                    crate::tls::client::build_tls_client_and_rustls_config(
                        &config.tls,
                        Some(&config.proxy),
                        self.crl_worker.clone(),
                        timeout_config.connect_timeout,
                    )
                    .context(TlsClientCreationSnafu)?;
                let login_parameters = LoginParameters::from_connection_config(
                    &config,
                    client_info,
                    None,
                    read_spcs_token(self.fs_adapter().as_ref()),
                );

                // ---- Diagnostics: pre-connect -----------------------------------
                // Run diagnostics when explicitly enabled OR when troubleshooting
                // is active (SNOWFLAKE_TROUBLESHOOTING_ENABLED=true implies diagnostics).
                let ts_path = self.troubleshooting_path();
                let effective_diag = match config.diagnostic {
                    DiagnosticConfig::Enabled {
                        ref log_path,
                        ref allowlist_path,
                    } => Some(DiagnosticConfig::Enabled {
                        log_path: log_path.clone().or_else(|| ts_path.clone()),
                        allowlist_path: allowlist_path.clone(),
                    }),
                    DiagnosticConfig::Disabled if ts_path.is_some() => {
                        Some(DiagnosticConfig::Enabled {
                            log_path: ts_path,
                            allowlist_path: None,
                        })
                    }
                    _ => None,
                };
                let mut diag_runner = if let Some(diag_cfg) = effective_diag {
                    let account = config.server.account.clone();
                    let host_str = host.clone().unwrap_or_default();
                    let diag_proxy = config.proxy.clone();
                    let diag_client_info = login_parameters.client_info.clone();
                    tokio::task::spawn_blocking(move || {
                        let mut runner = DiagnosticRunner::new(
                            &account,
                            &host_str,
                            diag_cfg,
                            diag_rustls,
                            diag_proxy,
                            &diag_client_info,
                        );
                        runner.run_pre_connect();
                        runner
                    })
                    .await
                    .ok()
                } else {
                    None
                };

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

                let retry_policy = RetryPolicy::login(&effective_seed);

                let login_fut = crate::rest::snowflake::snowflake_login_with_client(
                    &http_client,
                    &login_parameters,
                    init_params.as_ref(),
                    token_cache,
                    Some(&self.prompt_locks),
                    &retry_policy,
                );

                let login_result = if let Some(budget) = timeout_config.login_timeout {
                    match tokio::time::timeout(budget, login_fut).await {
                        Ok(inner) => inner,
                        Err(_) => Err(crate::rest::snowflake::OperationTimeoutSnafu {
                            operation: "login".to_string(),
                            budget,
                            ids: crate::rest::snowflake::QueryIds::default(),
                        }
                        .build()),
                    }
                } else {
                    login_fut.await
                };

                // ---- Diagnostics: post-connect ----------------------------------
                if let Some(mut runner) = diag_runner.take() {
                    tokio::task::spawn_blocking(move || {
                        runner.run_post_connect(None);
                        runner.write_report();
                    })
                    .await
                    .ok();
                }

                let login_result = login_result.context(LoginSnafu)?;

                // Initialize connection with session parameters from login response.
                // The server returns system-level parameters but may not echo back
                // user-set parameters (e.g. QUERY_TAG), so we merge in the
                // init_session_parameters the caller explicitly requested.
                let mut merged_params: HashMap<String, Setting> = init_params
                    .unwrap_or_default()
                    .into_iter()
                    .map(|(k, v)| (k, Setting::String(v)))
                    .collect();
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
                    .get_bool_or(param_names::CLIENT_SESSION_KEEP_ALIVE.as_str(), false);
                let heartbeat_frequency_secs = merged_params
                    .get_u64(param_names::CLIENT_SESSION_KEEP_ALIVE_HEARTBEAT_FREQUENCY.as_str());

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
                        database_seed,
                        resolved_snapshot,
                        logout_config,
                        timeout_config,
                        self.wrapper_presets.clone(),
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
                            .map(|v| v.coerce_bool().unwrap_or(false))
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

                        if let Some(sessions) = self.telemetry_sessions() {
                            sessions
                                .write()
                                .unwrap_or_else(|e| e.into_inner())
                                .insert(session_id, exporter_session);
                        } else {
                            // `telemetry_enabled` implies `telemetry_sessions()`
                            // was Some (checked above); reaching None here means
                            // the registry was cleared concurrently.
                            tracing::error!(
                                "telemetry session registry unexpectedly absent after \
                                 telemetry_enabled check; session {session_id} not \
                                 registered for telemetry"
                            );
                        }

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
                            conn.is_master_token_expired.clone(),
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
                let (canonical, def) =
                    canonicalize_setting_key(self.wrapper_presets.configuration_flavor, &key);
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
        no_connection_details: bool,
    ) -> Result<Vec<ValidationIssue>, ApiError> {
        match self.connections.get_obj(handle) {
            Some(conn_ptr) => {
                let mut conn = conn_ptr.lock().await;
                let post = conn.is_post_connect();
                // Latch the bare-connect signal so a later non-bare set_options
                // call cannot clear it. Only the wrapper can compute this (it
                // alone sees the raw caller input before bookkeeping is added).
                conn.no_connection_details |= no_connection_details;
                let (resolved, issues) =
                    resolve_options(self.wrapper_presets.configuration_flavor, options);
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
                let (canonical, def) =
                    canonicalize_setting_key(self.wrapper_presets.configuration_flavor, &key);
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
                if let Some(release_type) = identity.release_type.clone() {
                    inject_if_absent(
                        &mut conn.connection_seed,
                        "client_release_type",
                        Setting::String(release_type),
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
fn warn_query_logging_risk(settings: &ParamStore) {
    if resolve_log_query_text(settings) {
        tracing::warn!(
            "log_query_text is enabled: SQL query text will appear in logs - \
             ensure logs are protected as confidential data"
        );
    }
    if resolve_log_query_parameters(settings) {
        tracing::warn!(
            "log_query_parameters is enabled: bind parameter values will appear in logs - \
             ensure logs are protected as confidential data"
        );
    }
}

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
    /// Optional override for `CLIENT_ENVIRONMENT.RELEASE_TYPE`. When `None`,
    /// core derives the value from `driver_version` (Cargo/semver `-rc1` or
    /// PEP 440 `.dev0` / `rc1`). Set explicitly only when the wrapper needs a
    /// value different from that suffix. `None` on GA builds when the version
    /// has no suffix.
    pub release_type: Option<String>,
}

pub struct Connection {
    /// Options inherited from the database used to initialize this connection.
    /// Connection options override this base layer.
    pub(crate) database_seed: ParamStore,
    /// Explicit connection-string / API options. These override [`Self::database_seed`]
    /// in the top programmatic layer passed to [`resolver::resolve`].
    pub(crate) connection_seed: ParamStore,
    /// True when the caller invoked a bare `connect()` with no connection
    /// options — the wrapper's `is_kwargs_empty` signal, received via
    /// `ConnectionSetOptionsRequest::no_connection_details`. Drives the
    /// default-profile fallback in [`resolver::resolve`]. Latched on, so a
    /// later non-bare `set_options` call cannot clear it.
    pub(crate) no_connection_details: bool,
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
    pub session_parameters: Arc<AsyncRwLock<HashMap<String, Setting>>>,
    /// Query context cache (HTAP support)
    pub query_context_cache: QueryContextCacheAdapter,
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
    /// Flag indicating the master token has expired.
    ///
    /// Set to `true` when the server returns GS code 390114 (master token expired),
    /// or when a time-based check (`SessionTokens::is_master_expired`) confirms
    /// expiry just before a refresh attempt.  Once set, the session can never be
    /// renewed — full re-authentication is required.
    ///
    /// Mirrors `SnowflakeConnection.expired` in the legacy Python connector and is
    /// intended as a read-only signal for external pool / application code.
    pub is_master_token_expired: Arc<AtomicBool>,

    /// Logout configuration (set via ConnectionSetOption* before init, parsed at init time)
    pub logout_config: LogoutConfig,
    /// Resolved operation-level timeout configuration (populated at connect time).
    pub timeout_config: crate::config::retry::TimeoutConfig,
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

    /// Lifecycle state of the per-session `SYSTEM$BIND` stage.
    ///
    /// A single tri-state value is used instead of two booleans to make the
    /// illegal fourth state (`Created = true` **and** `Disabled = true`
    /// simultaneously) unrepresentable by construction.
    ///
    /// Wrappers should consult this — together with the session parameter
    /// `CLIENT_STAGE_ARRAY_BINDING_THRESHOLD` — when deciding whether to send
    /// CSV bindings.
    pub stage_state: Arc<AtomicStageState>,
}

impl Default for Connection {
    fn default() -> Self {
        Self::new()
    }
}

impl Connection {
    pub fn new() -> Self {
        Connection {
            database_seed: ParamStore::new(),
            connection_seed: ParamStore::new(),
            no_connection_details: false,
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
            query_context_cache: QueryContextCacheAdapter::new(),
            init_session_parameters: None,
            async_query_registry: AsyncQueryRegistry::new(),
            is_closed: Arc::new(AtomicBool::new(false)),
            is_master_token_expired: Arc::new(AtomicBool::new(false)),
            logout_config: LogoutConfig::default(),
            timeout_config: crate::config::retry::TimeoutConfig::default(),
            final_session_names: RwLock::new(FinalSessionNames::default()),
            server_version: RwLock::new(None),
            wrapper_identity: None,
            session_id: None,
            heartbeat_handle: None,
            stage_state: Arc::new(AtomicStageState::new(StageState::Unknown)),
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

    /// Reads `unsafe_file_write` from the effective connection/database seed. Default `false`
    /// (owner-only 0o600 permissions on GET downloads on Unix).
    pub(crate) fn unsafe_file_write(&self) -> bool {
        self.connection_seed
            .get_bool(param_names::UNSAFE_FILE_WRITE)
            .or_else(|| self.database_seed.get_bool(param_names::UNSAFE_FILE_WRITE))
            .unwrap_or(false)
    }

    /// Effective PUT/GET enablement. Legacy parity: PUT/GET is disabled if
    /// EITHER the client `enablePutGet` property or the server
    /// `JDBC_ENABLE_PUT_GET` session parameter is `false`; both default `true`.
    /// When disabled, the statement path rejects PUT/GET before dispatch with
    /// "File transfers have been disabled."
    ///
    /// Both flags are JDBC-specific — the caller only consults this on wrappers
    /// that opt in via `WrapperPresets::honor_put_get_disable` (JDBC only), so
    /// this accessor just reads the flags and does not itself scope by driver.
    pub(crate) async fn enable_put_get(&self) -> bool {
        let client_enabled = self
            .connection_seed
            .get_bool(param_names::ENABLE_PUT_GET)
            .unwrap_or(true);

        let server_enabled = self
            .session_parameters
            .read()
            .await
            .get(JDBC_ENABLE_PUT_GET_SERVER_KEY)
            .map(|value| {
                if let Setting::String(value) = value
                    && value.trim().is_empty()
                {
                    return true;
                }
                value.coerce_bool().unwrap_or(false)
            })
            .unwrap_or(true);

        client_enabled && server_enabled
    }

    /// Resolves a two-tier boolean override: session override first, else the
    /// connect-time seed. `None` if unset, so the caller falls back to the
    /// wrapper preset. Backs `put_fastfail`/`get_fastfail`.
    ///
    /// Only 2 tiers: these are client-only params never echoed by the server,
    /// so the session-param cache and resolved_connect don't apply.
    fn resolve_override_bool(&self, key: ParamKey) -> Option<bool> {
        self.session_overrides
            .get_bool(key)
            .or_else(|| self.connection_seed.get_bool(key))
            .or_else(|| self.database_seed.get_bool(key))
    }

    /// Resolves `PUT_FASTFAIL` (see [`Self::resolve_override_bool`]).
    pub(crate) fn put_fastfail(&self) -> Option<bool> {
        self.resolve_override_bool(param_names::PUT_FASTFAIL)
    }

    /// Resolves `GET_FASTFAIL` (see [`Self::resolve_override_bool`]).
    pub(crate) fn get_fastfail(&self) -> Option<bool> {
        self.resolve_override_bool(param_names::GET_FASTFAIL)
    }

    /// The resolved TLS config for this connection, read from the established
    /// [`ClientInfo`]. Falls back to the default `TlsConfig` before login
    /// (when `client_info` is unset). Used by the storage (S3/GCS/Azure) HTTP
    /// clients on the PUT/GET path to honour CRL, custom root stores, and the
    /// protocol-version window.
    pub(crate) fn tls_config(&self) -> crate::tls::config::TlsConfig {
        self.client_info
            .as_ref()
            .map(|ci| ci.tls_config.clone())
            .unwrap_or_default()
    }

    /// The resolved proxy config for this connection, read from the established
    /// [`ClientInfo`]. Falls back to the default `ProxyConfig` before login
    /// (when `client_info` is unset). Used by the storage (S3/GCS/Azure) HTTP
    /// clients on the PUT/GET path to honour `proxy_host`/`proxy_port`/
    /// `no_proxy`/`use_proxy_env` — mirroring the GS/REST connection client.
    pub(crate) fn proxy_config(&self) -> crate::tls::config::ProxyConfig {
        self.client_info
            .as_ref()
            .map(|ci| ci.proxy_config.clone())
            .unwrap_or_default()
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
        resolver::resolve(&self.connection_seed, self.no_connection_details)
    }

    fn effective_seed(&self, database_seed: &ParamStore) -> ParamStore {
        let mut effective = database_seed.clone();
        effective.extend_from_case_insensitive(&self.connection_seed);
        effective
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
        session_params: HashMap<String, Setting>,
        final_names: FinalSessionNames,
        server_version: Option<String>,
        database_seed: ParamStore,
        resolved_connect: ParamStore,
        logout_config: LogoutConfig,
        timeout_config: crate::config::retry::TimeoutConfig,
        wrapper_presets: WrapperPresets,
    ) {
        *self.tokens.write().await = Some(tokens);
        self.http_client = Some(http_client);
        self.database_seed = database_seed;
        self.retry_policy = RetryPolicy::http(&self.effective_seed(&self.database_seed));
        self.timeout_config = timeout_config;
        self.host = host;
        self.port = port;
        self.server_url = Some(server_url);
        self.client_info = Some(client_info);
        self.resolved_connect = Some(resolved_connect);
        self.query_context_cache
            .init(self.resolved_connect.as_ref(), &wrapper_presets);
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
                    (p.name.clone(), Setting::String(p.value.clone()))
                }),
        );

        // 2. Response parameters: merge any server-returned session parameters into the cache.
        if let Some(parameters) = response_parameters {
            cache.extend(parameters.iter().filter_map(|param| {
                Setting::from_session_parameter_json(&param.value)
                    .map(|setting| (param.name.to_uppercase(), setting))
            }));
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
    let mut refresh_ctx = RefreshContext::from_arc(conn).await?;
    refresh_ctx.execute_with_refresh(f).await
}

/// Context for automatic session token refresh.
///
/// Instead of a higher-order function pattern, `RefreshContext` gives callers
/// a loop-based API:
///
/// ```ignore
/// let mut refresh_ctx = RefreshContext::new(&conn)?;
/// let mut last_error: Option<RestError> = None;
/// loop {
///     let token = refresh_ctx.refresh_token(last_error).await?;
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
    /// Shared expired flag; set to `true` when master token expiry is detected
    /// during a refresh attempt so the owning `Connection` exposes it publicly.
    is_master_token_expired: Arc<AtomicBool>,
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
            return ConnectionClosedSnafu {}.fail();
        }
        Self::new(&guard)
    }

    /// Create a `RefreshContext` from individual components (no `Connection` needed).
    ///
    /// `is_master_token_expired` should be the owning connection's shared flag so
    /// that a master-token expiry detected here (e.g. on the background heartbeat
    /// path) is observable via `Connection::is_expired`.
    pub fn from_parts(
        tokens_lock: Arc<AsyncRwLock<Option<SessionTokens>>>,
        http_client: reqwest::Client,
        server_url: String,
        client_info: ClientInfo,
        is_master_token_expired: Arc<AtomicBool>,
    ) -> Self {
        Self {
            tokens_lock,
            http_client,
            server_url,
            client_info,
            state: RefreshState::Initial,
            is_master_token_expired,
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
            is_master_token_expired: conn.is_master_token_expired.clone(),
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
                // Server returned GS 390113/390114/390115 — master token can
                // never be renewed; mark expired immediately so the connection's
                // `is_master_token_expired` flag is visible to external observers
                // before the error propagates.
                Some(RestError::MasterTokenTerminal { code, .. }) => {
                    tracing::error!(
                        code,
                        "Server reported master token can never be renewed, full re-authentication required"
                    );
                    self.is_master_token_expired.store(true, Ordering::SeqCst);
                    MasterTokenTerminalSnafu {
                        master_token_gs_code: Some(code),
                    }
                    .fail()
                }
                Some(RestError::SessionExpired { .. }) => {
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

                    // Check if master token is expired. This is a client-side,
                    // time-based prediction — no server round-trip happened, so
                    // there is no real GS code to carry; never fabricate one.
                    if tokens.is_master_expired() {
                        tracing::error!("Master token expired, full re-authentication required");
                        self.is_master_token_expired.store(true, Ordering::SeqCst);
                        return MasterTokenTerminalSnafu {
                            master_token_gs_code: None,
                        }
                        .fail();
                    }

                    // Refresh session (still holding write lock to prevent concurrent refreshes)
                    let new_tokens = match snowflake::refresh_session(
                        &self.http_client,
                        &self.server_url,
                        &self.client_info,
                        &tokens,
                    )
                    .await
                    {
                        Ok(new_tokens) => new_tokens,
                        Err(refresh_err) => {
                            // GS 390113/390114/390115 from the refresh endpoint mean
                            // the master token can never be renewed: mark the
                            // connection expired before propagating, mirroring the
                            // query-response path.
                            if matches!(&refresh_err, RestError::MasterTokenTerminal { .. }) {
                                tracing::error!(
                                    "Server reported master token can never be renewed during refresh, full re-authentication required"
                                );
                                self.is_master_token_expired.store(true, Ordering::SeqCst);
                            }
                            return Err(refresh_err).context(SessionRefreshSnafu);
                        }
                    };

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
        crate::refresh::execute_with_refresh(self, |token| {
            let fut = f(token);
            async move { fut.await.context(QuerySnafu) }
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

    /// Whether `err` warrants a session refresh. Note the deliberate
    /// side-effect: for a non-renewable master-token terminal
    /// (GS 390113/390114/390115) this sets `is_master_token_expired` here
    /// (before declining to refresh), because `refresh()` is never reached
    /// for that error.
    fn should_refresh(&self, err: &ApiError) -> bool {
        let ApiError::Query { source, .. } = err else {
            return false;
        };
        // GS 390113/390114/390115: master token can never be renewed — mark
        // connection as expired and do NOT attempt a refresh.
        if matches!(source.as_ref(), RestError::MasterTokenTerminal { .. }) {
            tracing::error!(
                "Server reported master token can never be renewed, full re-authentication required"
            );
            self.is_master_token_expired.store(true, Ordering::SeqCst);
            return false;
        }
        matches!(source.as_ref(), RestError::SessionExpired { .. })
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
                // Client-side, time-based prediction — no server round-trip
                // happened, so there is no real GS code to carry; never
                // fabricate one (see the matching comment in refresh_token above).
                tracing::error!("Master token expired, full re-authentication required");
                self.is_master_token_expired.store(true, Ordering::SeqCst);
                return MasterTokenTerminalSnafu {
                    master_token_gs_code: None,
                }
                .fail();
            }

            tracing::info!("Session expired, attempting refresh");
            let new_tokens = match snowflake::refresh_session(
                &self.http_client,
                &self.server_url,
                &self.client_info,
                &tokens,
            )
            .await
            {
                Ok(new_tokens) => new_tokens,
                Err(refresh_err) => {
                    // The refresh endpoint can itself return GS 390113/390114/
                    // 390115 (master token can never be renewed).
                    // refresh_session surfaces that as RestError::MasterTokenTerminal;
                    // mark the connection expired before propagating, mirroring
                    // should_refresh() on the query path.
                    if matches!(&refresh_err, RestError::MasterTokenTerminal { .. }) {
                        tracing::error!(
                            "Server reported master token can never be renewed during refresh, full re-authentication required"
                        );
                        self.is_master_token_expired.store(true, Ordering::SeqCst);
                    }
                    return Err(refresh_err).context(SessionRefreshSnafu);
                }
            };

            *tokens_guard = Some(new_tokens);
            drop(tokens_guard);

            tracing::info!("Session refreshed, retrying operation");
            Ok(true)
        })
    }
}

pub use super::final_session_names::FinalSessionNames;

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
    /// The User-Agent string built by the core for this client
    pub user_agent: Option<String>,
    /// The configured HTTP proxy hostname, if any
    pub proxy_host: Option<String>,
    /// The configured HTTP proxy port, if any
    pub proxy_port: Option<i64>,
    /// The configured HTTP proxy username for Basic auth, if any
    pub proxy_user: Option<String>,
    /// The configured HTTP proxy password for Basic auth (redacted in Debug output)
    pub proxy_password: Option<SensitiveString>,
    /// Comma-separated list of hosts that bypass the proxy, if configured
    pub no_proxy: Option<String>,
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

fn is_empty_string_setting(setting: &Setting) -> bool {
    matches!(setting, Setting::String(s) if s.is_empty())
}

fn resolved_or_seed_string(conn: &Connection, key: ParamKey) -> Option<String> {
    if let Some(resolved) = &conn.resolved_connect
        && let Some(s) = resolved.get_string(key)
    {
        return Some(s);
    }
    conn.connection_seed
        .get_string(key)
        .or_else(|| conn.database_seed.get_string(key))
}

/// Session effective read: server cache → session overrides → resolved connect (then seed).
fn get_session_or_setting(
    conn: &Connection,
    param_name: &str,
    setting_key: ParamKey,
) -> Option<String> {
    if let Ok(cache) = conn.session_parameters.try_read()
        && let Some(v) = cache.get(param_name)
        && !is_empty_string_setting(v)
    {
        return setting_as_display_string(v);
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

// TODO: ensure that this is correct escaping
fn escape_sql_identifier(name: &str) -> String {
    name.replace('"', "\"\"")
}

fn build_use_schema_sql(database: Option<&str>, schema: &str) -> String {
    let escaped_schema = escape_sql_identifier(schema);
    match database.filter(|db| !db.is_empty()) {
        Some(db) => {
            let escaped_db = escape_sql_identifier(db);
            format!("USE SCHEMA \"{escaped_db}\".\"{escaped_schema}\"")
        }
        None => format!("USE SCHEMA \"{escaped_schema}\""),
    }
}

fn resolve_session_database(conn: &Connection) -> Result<Option<String>, ApiError> {
    let final_names = conn
        .final_session_names
        .read()
        .map_err(|_| ConnectionLockSnafu {}.build())?;
    Ok(final_names
        .database
        .clone()
        .or_else(|| get_session_or_setting(conn, "DATABASE", param_names::DATABASE))
        .filter(|db| !db.is_empty()))
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

                let account = resolved_or_seed_string(&conn, param_names::ACCOUNT);
                let user = resolved_or_seed_string(&conn, param_names::USER);

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
                    .map_err(|_| ConnectionLockSnafu {}.build())?;
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

                let user_agent = conn
                    .client_info
                    .as_ref()
                    .map(crate::rest::snowflake::user_agent);

                // Rebuild the effective proxy config from the resolved login
                // snapshot (falling back to the raw seed before login) so the
                // legacy `PROXY` URL form is merged with the individual
                // `proxy_*` fields exactly as the HTTP client sees them. Reuses
                // the single parser rather than re-reading fields piecemeal.
                let proxy_settings: &dyn Settings = conn
                    .resolved_connect
                    .as_ref()
                    .map(|s| s as &dyn Settings)
                    .unwrap_or(&conn.connection_seed);
                let proxy = ProxyConfig::from_settings(proxy_settings);

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
                    user_agent,
                    proxy_host: proxy.host,
                    proxy_port: proxy.port,
                    proxy_user: proxy.user,
                    proxy_password: proxy.password,
                    no_proxy: proxy.no_proxy,
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
        operation_ctx: Option<&OperationCtx>,
        conn_handle: Handle,
        query_id: &str,
    ) -> Result<snowflake::QueryStatusResult, ApiError> {
        let status = async {
            if query_id.is_empty() {
                return InvalidArgumentSnafu {
                    argument: "query_id must be non-empty".to_string(),
                }
                .fail();
            }

            let conn_ptr =
                self.connections
                    .get_obj(conn_handle)
                    .with_context(|| InvalidArgumentSnafu {
                        argument: "Connection handle not found".to_string(),
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
        };
        crate::apis::operation_ctx::run_opt(operation_ctx, "connection_get_query_status", status)
            .await
    }

    pub async fn connection_get_all_parameters(
        &self,
        conn_handle: Handle,
    ) -> Result<HashMap<String, Setting>, ApiError> {
        match self.connections.get_obj(conn_handle) {
            Some(conn_ptr) => {
                let conn = conn_ptr.lock().await;
                let mut result = HashMap::new();

                for (k, v) in conn.connection_seed.iter() {
                    if let Some(s) = setting_as_display_string(v) {
                        result.insert(k.to_uppercase(), Setting::String(s));
                    }
                }
                if let Some(resolved) = &conn.resolved_connect {
                    for (k, v) in resolved.iter() {
                        if let Some(s) = setting_as_display_string(v) {
                            result.insert(k.to_uppercase(), Setting::String(s));
                        }
                    }
                }
                for (k, v) in conn.session_overrides.iter() {
                    if let Some(s) = setting_as_display_string(v) {
                        result.insert(k.to_uppercase(), Setting::String(s));
                    }
                }

                let cache = conn.session_parameters.read().await;
                for (k, v) in cache.iter() {
                    if !is_empty_string_setting(v) {
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
                    .map_err(|_| ConnectionLockSnafu {}.build())?
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
    ) -> Result<Option<Setting>, ApiError> {
        match self.connections.get_obj(conn_handle) {
            Some(conn_ptr) => {
                let conn = conn_ptr.lock().await;

                let cache = conn.session_parameters.read().await;

                let normalized_key = key.to_uppercase();
                if let Some(v) = cache
                    .get(&normalized_key)
                    .filter(|s| !is_empty_string_setting(s))
                {
                    return Ok(Some(v.clone()));
                }
                drop(cache);

                let (canonical, def) =
                    canonicalize_setting_key(self.wrapper_presets.configuration_flavor, &key);
                if let Some(d) = def
                    && d.is_session_scoped()
                {
                    if let Some(s) = conn
                        .session_overrides
                        .get_any(&canonical)
                        .and_then(setting_as_display_string)
                    {
                        return Ok(Some(Setting::String(s)));
                    }
                    return Ok(
                        resolved_or_seed_string(&conn, ParamKey::new(d.canonical_name))
                            .map(Setting::String),
                    );
                }

                if let Some(s) = conn
                    .session_overrides
                    .get_any(&canonical)
                    .and_then(setting_as_display_string)
                {
                    return Ok(Some(Setting::String(s)));
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
                    })
                    .map(Setting::String))
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
    /// are rejected so requests stay on the configured host.
    /// Auth is always the current session token managed by sf_core.
    ///
    /// The connection's client carries no request timeout, so without a `operation_ctx`
    /// this call is bounded only by the server. Cancelling drops the request;
    /// what the server does with an already-received one is its own business,
    /// which is why the caller chooses the path and the method.
    pub async fn connection_send_http_request(
        &self,
        operation_ctx: Option<&OperationCtx>,
        conn_handle: Handle,
        method: String,
        url: String,
        headers: HashMap<String, String>,
        body: Option<Vec<u8>>,
    ) -> Result<HttpResponse, ApiError> {
        let send = async {
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
                            argument: "Session token contains invalid header characters"
                                .to_string(),
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
                if header_name == reqwest::header::AUTHORIZATION
                    || header_name == reqwest::header::HOST
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
        };
        crate::apis::operation_ctx::run_opt(operation_ctx, "connection_send_http", send).await
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

            let mut refresh_ctx = match RefreshContext::from_arc(&conn_ptr).await {
                Ok(refresh_ctx) => refresh_ctx,
                Err(_) => return Ok(false),
            };

            let server_url = match url::Url::parse(&refresh_ctx.server_url) {
                Ok(u) => u,
                Err(_) => return Ok(false),
            };

            let mut last_error: Option<RestError> = None;

            loop {
                let token = match refresh_ctx.refresh_token(last_error.take()).await {
                    Ok(t) => t,
                    Err(_) => return Ok(false),
                };

                match heartbeat::send_heartbeat_with_timeout(
                    &refresh_ctx.http_client,
                    &server_url,
                    &refresh_ctx.client_info,
                    token.reveal(),
                    timeout,
                )
                .await
                {
                    Ok(()) => return Ok(true),
                    Err(e @ RestError::SessionExpired { .. }) => {
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
    ///
    /// Cancelling drops the request without invalidating anything: the master
    /// token this authenticates with is unaffected, and a session token the
    /// server may have minted for a dropped ISSUE is simply never used.
    pub async fn connection_token_request(
        &self,
        operation_ctx: Option<&OperationCtx>,
        conn_handle: Handle,
        request_type: String,
    ) -> Result<snowflake::TokenRequestResult, ApiError> {
        let token = async {
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
        };
        crate::apis::operation_ctx::run_opt(operation_ctx, "connection_request_token", token).await
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

    /// Check if the connection's master token has expired.
    ///
    /// Returns `true` when the server returned GS code 390114 or a time-based
    /// check confirmed master-token expiry during a refresh attempt.  Once
    /// `true`, the session can never be renewed — full re-authentication is
    /// required.
    ///
    /// Mirrors `SnowflakeConnection.expired` in the legacy Python connector.
    pub async fn connection_is_expired(&self, conn_handle: Handle) -> Result<bool, ApiError> {
        let conn_ptr = self
            .connections
            .get_obj(conn_handle)
            .context(InvalidArgumentSnafu {
                argument: "Connection handle not found".to_string(),
            })?;

        let conn = conn_ptr.lock().await;
        Ok(conn.is_master_token_expired.load(Ordering::SeqCst))
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
                // Re-derive logout config from the database + connection layers
                // at close-time so post-init connection overrides (e.g.
                // retry=False sets logout_max_attempts=1) take effect.
                let effective_seed = conn.effective_seed(&conn.database_seed);
                let config =
                    LogoutConfig::from_settings(&effective_seed).unwrap_or_else(|e| {
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

        // Reap any upload/download stream sessions still open on this
        // connection before logout I/O invalidates its credentials — a
        // download's background tasks reading from cloud storage with a
        // soon-to-be-revoked session would otherwise keep running (or sit
        // stalled) until they hit their own error path.
        self.reap_connection_streams(conn_handle);

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
    // Upload/download stream sessions are reaped in connection_close, before
    // logout I/O, via DatabaseDriverV1::reap_connection_streams — not here.
    // TODO(SNOW-3704961): a session that outlives its connection because the
    // wrapper never calls connection_close (process crash, or a stream begun
    // and simply abandoned) still leaks until process exit; there is no
    // idle-timeout reaper for abandoned upload/download stream sessions yet.

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
    fn effective_seed_inherits_database_options() {
        let conn = Connection::new();
        let mut database_seed = ParamStore::new();
        database_seed.insert("account".into(), Setting::String("database-account".into()));

        let effective = conn.effective_seed(&database_seed);

        assert_eq!(
            effective.get_string(param_names::ACCOUNT),
            Some("database-account".into())
        );
    }

    #[test]
    fn effective_seed_prefers_connection_options() {
        let conn = make_connection_with_settings(vec![(
            "account",
            Setting::String("connection-account".into()),
        )]);
        let mut database_seed = ParamStore::new();
        database_seed.insert("account".into(), Setting::String("database-account".into()));
        database_seed.insert("user".into(), Setting::String("database-user".into()));

        let effective = conn.effective_seed(&database_seed);

        assert_eq!(
            effective.get_string(param_names::ACCOUNT),
            Some("connection-account".into())
        );
        assert_eq!(
            effective.get_string(param_names::USER),
            Some("database-user".into())
        );
    }

    #[test]
    fn effective_seed_applies_unknown_option_precedence_case_insensitively() {
        let conn = make_connection_with_settings(vec![(
            "custom_session_parameter",
            Setting::String("connection-value".into()),
        )]);
        let mut database_seed = ParamStore::new();
        database_seed.insert(
            "CUSTOM_SESSION_PARAMETER".into(),
            Setting::String("database-value".into()),
        );

        let effective = conn.effective_seed(&database_seed);
        let unknown = collect_unknown_settings(&effective);

        assert_eq!(unknown.len(), 1);
        assert_eq!(
            unknown.get("CUSTOM_SESSION_PARAMETER"),
            Some(&"connection-value".to_string())
        );
    }

    #[tokio::test]
    async fn connection_init_requires_a_valid_database_handle() {
        let driver = DatabaseDriverV1::new();
        let conn_handle = driver.connection_new();
        let missing_database = Handle {
            id: u64::MAX,
            magic: 0,
        };

        let error = driver
            .connection_init(None, conn_handle, missing_database)
            .await
            .unwrap_err();

        assert!(
            error.to_string().contains("Database handle not found"),
            "unexpected error: {error}"
        );
        driver.connection_release(conn_handle).unwrap();
    }

    // `enable_put_get()` reads the two flags only; the JDBC-only scoping lives in
    // the statement gate via `WrapperPresets::honor_put_get_disable` (asserted in
    // global_state's preset tests), so these cases need no wrapper identity.
    #[tokio::test]
    async fn enable_put_get_defaults_to_true_when_unset() {
        let conn = Connection::new();
        assert!(conn.enable_put_get().await);
    }

    #[tokio::test]
    async fn enable_put_get_false_when_client_property_disables() {
        let conn = make_connection_with_settings(vec![(
            param_names::ENABLE_PUT_GET.as_str(),
            Setting::Bool(false),
        )]);
        assert!(!conn.enable_put_get().await);
    }

    #[tokio::test]
    async fn enable_put_get_false_when_server_session_param_disables() {
        // Server `JDBC_ENABLE_PUT_GET=false` disables even with the client seed absent/true.
        let conn = Connection::new();
        conn.session_parameters.try_write().unwrap().insert(
            "JDBC_ENABLE_PUT_GET".into(),
            Setting::String("false".into()),
        );
        assert!(!conn.enable_put_get().await);
    }

    #[tokio::test]
    async fn enable_put_get_true_when_server_session_param_blank() {
        // A blank `JDBC_ENABLE_PUT_GET` is treated as unset, not as `false`.
        let conn = Connection::new();
        conn.session_parameters
            .try_write()
            .unwrap()
            .insert("JDBC_ENABLE_PUT_GET".into(), Setting::String("".into()));
        assert!(conn.enable_put_get().await);
    }

    #[tokio::test]
    async fn enable_put_get_true_when_both_flags_enabled() {
        let conn = make_connection_with_settings(vec![(
            param_names::ENABLE_PUT_GET.as_str(),
            Setting::Bool(true),
        )]);
        conn.session_parameters
            .try_write()
            .unwrap()
            .insert("JDBC_ENABLE_PUT_GET".into(), Setting::String("true".into()));
        assert!(conn.enable_put_get().await);
    }

    #[test]
    fn build_use_schema_sql_without_database() {
        assert_eq!(
            build_use_schema_sql(None, "PUBLIC"),
            "USE SCHEMA \"PUBLIC\""
        );
    }

    #[test]
    fn build_use_schema_sql_with_database() {
        assert_eq!(
            build_use_schema_sql(Some("TEST_DB"), "MY_SCHEMA"),
            "USE SCHEMA \"TEST_DB\".\"MY_SCHEMA\""
        );
    }

    #[test]
    fn build_use_schema_sql_escapes_quotes() {
        assert_eq!(
            build_use_schema_sql(Some("DB\"1"), "SC\"2"),
            "USE SCHEMA \"DB\"\"1\".\"SC\"\"2\""
        );
    }

    #[test]
    fn get_session_or_setting_prefers_session_parameter() {
        let conn =
            make_connection_with_settings(vec![("database", Setting::String("setting_db".into()))]);
        conn.session_parameters
            .try_write()
            .unwrap()
            .insert("DATABASE".into(), Setting::String("session_db".into()));

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
            .insert("ROLE".into(), Setting::String(String::new()));

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

    #[test]
    fn put_fastfail_returns_none_when_unset() {
        let conn = Connection::new();
        assert_eq!(conn.put_fastfail(), None);
    }

    #[test]
    fn put_fastfail_returns_connection_seed_value_when_only_seed_set() {
        let conn = make_connection_with_settings(vec![("put_fastfail", Setting::Bool(true))]);
        assert_eq!(conn.put_fastfail(), Some(true));
    }

    #[test]
    fn put_fastfail_prefers_session_override_over_connection_seed() {
        let mut conn = make_connection_with_settings(vec![("put_fastfail", Setting::Bool(true))]);
        conn.session_overrides
            .insert("put_fastfail".into(), Setting::Bool(false));
        assert_eq!(conn.put_fastfail(), Some(false));
    }

    #[test]
    fn get_fastfail_returns_none_when_unset() {
        let conn = Connection::new();
        assert_eq!(conn.get_fastfail(), None);
    }

    #[test]
    fn get_fastfail_returns_connection_seed_value_when_only_seed_set() {
        let conn = make_connection_with_settings(vec![("get_fastfail", Setting::Bool(true))]);
        assert_eq!(conn.get_fastfail(), Some(true));
    }

    #[test]
    fn get_fastfail_prefers_session_override_over_connection_seed() {
        let mut conn = make_connection_with_settings(vec![("get_fastfail", Setting::Bool(true))]);
        conn.session_overrides
            .insert("get_fastfail".into(), Setting::Bool(false));
        assert_eq!(conn.get_fastfail(), Some(false));
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
    async fn connection_set_options_records_no_connection_details_flag() {
        let ds = DatabaseDriverV1::new();
        let handle = ds.connection_new();

        // A bare connect() arrives as set_options with the flag set.
        ds.connection_set_options(handle, HashMap::new(), true)
            .await
            .unwrap();

        let conn_ptr = ds.connections.get_obj(handle).unwrap();
        let conn = conn_ptr.lock().await;
        assert!(conn.no_connection_details);
        drop(conn);
        ds.connection_release(handle).unwrap();
    }

    #[tokio::test]
    async fn connection_set_options_latches_no_connection_details_flag() {
        let ds = DatabaseDriverV1::new();
        let handle = ds.connection_new();

        // First call marks the connection as bare; a later non-bare call must
        // not clear the latch.
        ds.connection_set_options(handle, HashMap::new(), true)
            .await
            .unwrap();
        let mut later = HashMap::new();
        later.insert("user".to_owned(), Setting::String("alice".to_owned()));
        ds.connection_set_options(handle, later, false)
            .await
            .unwrap();

        let conn_ptr = ds.connections.get_obj(handle).unwrap();
        let conn = conn_ptr.lock().await;
        assert!(
            conn.no_connection_details,
            "no_connection_details must stay latched across later set_options calls"
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

    /// `set_option` with the canonical `put_get_max_attempts` key must land
    /// on `connection_seed` so the PUT/GET dispatch site can read it.
    #[tokio::test]
    async fn connection_set_option_put_get_max_attempts_lands_on_seed() {
        let ds = DatabaseDriverV1::new();
        let handle = ds.connection_new();
        ds.connection_set_option(
            handle,
            param_names::PUT_GET_MAX_ATTEMPTS.as_str().into(),
            Setting::Int(10),
        )
        .await
        .unwrap();

        let conn_ptr = ds.connections.get_obj(handle).unwrap();
        let conn = conn_ptr.lock().await;
        assert_eq!(
            conn.connection_seed
                .get_int(param_names::PUT_GET_MAX_ATTEMPTS),
            Some(10),
        );
        drop(conn);
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
        assert_eq!(info.proxy_host, None);
        assert_eq!(info.proxy_port, None);
        assert_eq!(info.proxy_user, None);
        assert!(info.proxy_password.is_none());
        assert_eq!(info.no_proxy, None);

        ds.connection_release(handle).unwrap();
    }

    #[tokio::test]
    async fn connection_get_info_returns_proxy_settings() {
        let ds = DatabaseDriverV1::new();
        let handle = ds.connection_new();
        ds.connection_set_option(
            handle,
            "proxy_host".into(),
            Setting::String("proxy.example.com".into()),
        )
        .await
        .unwrap();
        ds.connection_set_option(handle, "proxy_port".into(), Setting::Int(8080))
            .await
            .unwrap();
        ds.connection_set_option(handle, "proxy_user".into(), Setting::String("puser".into()))
            .await
            .unwrap();
        ds.connection_set_option(
            handle,
            "proxy_password".into(),
            Setting::String("ppass".into()),
        )
        .await
        .unwrap();
        ds.connection_set_option(
            handle,
            "no_proxy".into(),
            Setting::String("localhost,127.0.0.1".into()),
        )
        .await
        .unwrap();

        let info = ds.connection_get_info(handle).await.unwrap();

        assert_eq!(info.proxy_host, Some("proxy.example.com".into()));
        assert_eq!(info.proxy_port, Some(8080));
        assert_eq!(info.proxy_user, Some("puser".into()));
        assert_eq!(
            info.proxy_password.as_ref().map(|p| p.reveal().as_str()),
            Some("ppass")
        );
        assert_eq!(info.no_proxy, Some("localhost,127.0.0.1".into()));

        ds.connection_release(handle).unwrap();
    }

    /// The legacy ODBC `PROXY` URL form must be parsed and surfaced through
    /// `connection_get_info` exactly as the HTTP client sees it, since the
    /// info aggregation reuses `ProxyConfig::from_settings`.
    #[tokio::test]
    async fn connection_get_info_parses_legacy_proxy_url() {
        let ds = DatabaseDriverV1::new();
        let handle = ds.connection_new();
        ds.connection_set_option(
            handle,
            "proxy".into(),
            Setting::String("http://puser:ppass@proxy.example.com:8080".into()),
        )
        .await
        .unwrap();

        let info = ds.connection_get_info(handle).await.unwrap();

        assert_eq!(info.proxy_host, Some("proxy.example.com".into()));
        assert_eq!(info.proxy_port, Some(8080));
        assert_eq!(info.proxy_user, Some("puser".into()));
        assert_eq!(
            info.proxy_password.as_ref().map(|p| p.reveal().as_str()),
            Some("ppass")
        );

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
                .insert("DATABASE".into(), Setting::String("session_db".into()));
            conn.session_parameters
                .write()
                .await
                .insert("ROLE".into(), Setting::String("session_role".into()));
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
                .insert("DATABASE".into(), Setting::String("session_db".into()));
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
                None,
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
                None,
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
                None,
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
                None,
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
                None,
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
            .connection_send_http_request(
                None,
                handle,
                "GET".into(),
                "/api/test".into(),
                headers,
                None,
            )
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
            .connection_token_request(None, handle, "INVALID".into())
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
            cache.insert(
                "TIMEZONE".into(),
                Setting::String("America/Los_Angeles".into()),
            );
            cache.insert("QUERY_TAG".into(), Setting::String("test_tag".into()));
        }

        let params = ds.connection_get_all_parameters(handle).await.unwrap();
        assert_eq!(
            params.get("TIMEZONE").unwrap(),
            &Setting::String("America/Los_Angeles".into())
        );
        assert_eq!(
            params.get("QUERY_TAG").unwrap(),
            &Setting::String("test_tag".into())
        );
        assert_eq!(params.len(), 2);

        ds.connection_release(handle).unwrap();
    }

    #[tokio::test]
    async fn connection_get_info_user_agent_reflects_client_info() {
        use crate::config::rest_parameters::test_fixtures::test_client_info;

        let ds = DatabaseDriverV1::new();
        let handle = ds.connection_new();

        // No client_info set: user_agent must be None.
        let info = ds.connection_get_info(handle).await.unwrap();
        assert_eq!(info.user_agent, None);

        // Inject a ClientInfo and verify the UA string is populated.
        let ci = test_client_info();
        let expected_ua = crate::rest::snowflake::user_agent(&ci);
        if let Some(conn_ptr) = ds.connections.get_obj(handle) {
            let mut conn = conn_ptr.lock().await;
            conn.client_info = Some(ci);
        }

        let info = ds.connection_get_info(handle).await.unwrap();
        assert_eq!(info.user_agent, Some(expected_ua));

        ds.connection_release(handle).unwrap();
    }
}
