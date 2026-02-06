use snafu::{OptionExt, ResultExt};
use std::future::Future;
use std::{collections::HashMap, sync::Arc, sync::Mutex};
use tokio::sync::RwLock as AsyncRwLock;

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

#[repr(i32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QueryStatus {
    Running = 0,
    Aborting = 1,
    Success = 2,
    FailedWithError = 3,
    Aborted = 4,
    Queued = 5,
    FailedWithIncident = 6,
    Disconnected = 7,
    ResumingWarehouse = 8,
    QueuedReparingWarehouse = 9,
    Restarted = 10,
    Blocked = 11,
    NoData = 12,
}

impl QueryStatus {
    pub fn from_string(status: &str) -> Option<Self> {
        match status {
            "RUNNING" => Some(QueryStatus::Running),
            "ABORTING" => Some(QueryStatus::Aborting),
            "SUCCESS" => Some(QueryStatus::Success),
            "FAILED_WITH_ERROR" => Some(QueryStatus::FailedWithError),
            "ABORTED" => Some(QueryStatus::Aborted),
            "QUEUED" => Some(QueryStatus::Queued),
            "FAILED_WITH_INCIDENT" => Some(QueryStatus::FailedWithIncident),
            "DISCONNECTED" => Some(QueryStatus::Disconnected),
            "RESUMING_WAREHOUSE" => Some(QueryStatus::ResumingWarehouse),
            "QUEUED_REPARING_WAREHOUSE" => Some(QueryStatus::QueuedReparingWarehouse),
            "RESTARTED" => Some(QueryStatus::Restarted),
            "BLOCKED" => Some(QueryStatus::Blocked),
            "NO_DATA" => Some(QueryStatus::NoData),
            _ => None,
        }
    }
}

pub fn connection_get_query_status(
    conn_handle: Handle,
    query_id: String,
) -> Result<QueryStatus, ApiError> {
    let conn_ptr = CONN_HANDLE_MANAGER.get_obj(conn_handle).ok_or_else(|| {
        InvalidArgumentSnafu {
            argument: "Connection handle not found".to_string(),
        }
        .build()
    })?;

    // Create a blocking runtime for the REST API call
    let rt = tokio::runtime::Runtime::new().context(RuntimeCreationSnafu)?;

    let (http_client, server_url, retry_policy) = {
        let conn = conn_ptr
            .lock()
            .map_err(|_| ConnectionLockingSnafu {}.build())?;
        (
            conn.http_client
                .clone()
                .context(ConnectionNotInitializedSnafu)?,
            conn.server_url
                .clone()
                .context(ConnectionNotInitializedSnafu)?,
            conn.retry_policy.clone(),
        )
    };

    // Call monitoring API with automatic session refresh
    let conn = conn_ptr.clone();
    let status_response = rt.block_on(with_valid_session(&conn, |session_token| {
        let http_client = http_client.clone();
        let server_url = server_url.clone();
        let query_id = query_id.clone();
        let retry_policy = retry_policy.clone();
        async move {
            get_query_status_from_monitoring_api(
                &http_client,
                &server_url,
                &session_token,
                &query_id,
                &retry_policy,
            )
            .await
        }
    }))?;

    Ok(status_response)
}

pub fn connection_get_results_from_query_id(
    conn_handle: Handle,
    query_id: String,
) -> Result<super::statement::ExecuteResult, ApiError> {
    use super::error::StatementLockingSnafu;
    use super::global_state::STMT_HANDLE_MANAGER;
    use super::statement::{Statement, statement_execute_query};

    // Create a temporary statement to execute the RESULT_SCAN query
    let stmt_handle = {
        let conn_ptr = CONN_HANDLE_MANAGER.get_obj(conn_handle).ok_or_else(|| {
            InvalidArgumentSnafu {
                argument: "Connection handle not found".to_string(),
            }
            .build()
        })?;

        STMT_HANDLE_MANAGER.add_handle(std::sync::Mutex::new(Statement::new(conn_ptr.clone())))
    };

    // Set the RESULT_SCAN query
    let stmt_ptr = STMT_HANDLE_MANAGER.get_obj(stmt_handle).ok_or_else(|| {
        InvalidArgumentSnafu {
            argument: "Statement handle not found".to_string(),
        }
        .build()
    })?;

    {
        let mut stmt = stmt_ptr.lock().map_err(|_| StatementLockingSnafu.build())?;
        stmt.query = Some(format!("SELECT * FROM TABLE(RESULT_SCAN('{}'))", query_id));
    }

    // Execute the query
    let result = statement_execute_query(stmt_handle)?;

    // Clean up the temporary statement
    let _ = STMT_HANDLE_MANAGER.delete_handle(stmt_handle);

    Ok(result)
}

async fn get_query_status_from_monitoring_api(
    client: &reqwest::Client,
    server_url: &str,
    session_token: &str,
    query_id: &str,
    retry_policy: &RetryPolicy,
) -> Result<QueryStatus, RestError> {
    use reqwest::Method;
    use serde::Deserialize;
    use snafu::location;
    use url::Url;

    #[derive(Debug, Deserialize)]
    struct MonitoringResponse {
        data: MonitoringData,
    }

    #[derive(Debug, Deserialize)]
    struct MonitoringData {
        queries: Vec<QueryInfo>,
    }

    #[derive(Debug, Deserialize)]
    struct QueryInfo {
        status: String,
    }

    // Construct the monitoring URL
    let url = Url::parse(server_url)
        .and_then(|base| base.join(&format!("/monitoring/queries/{}", query_id)))
        .map_err(|source| RestError::UrlJoin {
            path: "/monitoring/queries",
            source,
            location: location!(),
        })?;

    // Make the request with retry
    use crate::http::retry::{HttpContext, execute_with_retry};

    let url_string = url.to_string();
    let request_fn = || {
        client
            .get(url_string.clone())
            .header(
                "Authorization",
                format!("Snowflake Token=\"{}\"", session_token),
            )
            .header("Accept", "application/json")
    };

    let ctx = HttpContext::new(Method::GET, url_string.clone());
    let response = execute_with_retry(request_fn, &ctx, retry_policy, |r| async move { Ok(r) })
        .await
        .map_err(|err| {
            // Convert HttpError to RestError
            match err {
                crate::http::retry::HttpError::Transport { source, .. } => {
                    RestError::Communication {
                        context: "get query status".to_string(),
                        source,
                        location: location!(),
                    }
                }
                _ => RestError::QueryFailed {
                    message: format!("HTTP request failed: {}", err),
                    location: location!(),
                },
            }
        })?;

    if !response.status().is_success() {
        return Err(RestError::InvalidSnowflakeResponse {
            source: SnowflakeResponseError::QueryNotFound {
                query_id: query_id.to_string(),
                location: location!(),
            },
            location: location!(),
        });
    }

    let body_bytes =
        response
            .bytes()
            .await
            .map_err(|source| RestError::InvalidSnowflakeResponse {
                source: SnowflakeResponseError::ResponseText {
                    source,
                    location: location!(),
                },
                location: location!(),
            })?;

    let monitoring_response: MonitoringResponse =
        serde_json::from_slice(&body_bytes).map_err(|source| {
            RestError::InvalidSnowflakeResponse {
                source: SnowflakeResponseError::ResponseFormat {
                    source,
                    location: location!(),
                },
                location: location!(),
            }
        })?;

    if monitoring_response.data.queries.is_empty() {
        return Err(RestError::InvalidSnowflakeResponse {
            source: SnowflakeResponseError::QueryNotFound {
                query_id: query_id.to_string(),
                location: location!(),
            },
            location: location!(),
        });
    }

    QueryStatus::from_string(&monitoring_response.data.queries[0].status).ok_or_else(|| {
        RestError::InvalidSnowflakeResponse {
            source: SnowflakeResponseError::UnexpectedResponse {
                message: format!(
                    "Unknown query status: {}",
                    monitoring_response.data.queries[0].status
                ),
                location: location!(),
            },
            location: location!(),
        }
    })
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
