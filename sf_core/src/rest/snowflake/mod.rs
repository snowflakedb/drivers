#![allow(clippy::result_large_err)]
pub mod async_exec;
mod auth;
pub mod error;
mod native_okta;
pub mod query_request;
pub mod query_response;

use std::collections::HashMap;

use crate::auth::{AuthError, Credentials, create_credentials};
use crate::config::rest_parameters::ClientInfo;
use crate::config::rest_parameters::{LoginMethod, LoginParameters, QueryParameters};
use crate::config::retry::RetryPolicy;
use crate::rest::snowflake::auth::{
    AuthRequest, AuthRequestClientEnvironment, AuthRequestData, AuthResponse,
};
use crate::rest::snowflake::error::SfError;
use crate::rest::snowflake::native_okta::fetch_native_okta_saml;
use crate::tls::client::create_tls_client_with_config;
use crate::tls::error::TlsError;
use reqwest::{self, header};
use serde_json;
use serde_json::value::RawValue;
use snafu::{IntoError, Location, OptionExt, ResultExt, Snafu};
use std::time::{Instant, SystemTime, UNIX_EPOCH};
use tracing;
use url::Url;

pub const STATEMENT_ASYNC_EXECUTION_OPTION: &str = "async_execution";
pub(crate) const QUERY_REQUEST_PATH: &str = "/queries/v1/query-request";
const TOKEN_REQUEST_PATH: &str = "/session/token-request";

/// Session tokens returned from login, used for authentication and refresh
#[derive(Debug, Clone)]
pub struct SessionTokens {
    /// Token used to authenticate API requests
    pub session_token: String,
    /// Token used to refresh an expired session token
    pub master_token: String,
    /// Server-assigned session ID
    pub session_id: i64,
    /// When the session token expires
    pub session_expires_at: Option<std::time::Instant>,
    /// When the master token expires (after this, full re-auth is needed)
    pub master_expires_at: Option<std::time::Instant>,
}

/// Result of a successful login to Snowflake
#[derive(Debug)]
pub struct LoginResult {
    /// Session tokens for authentication and refresh
    pub tokens: SessionTokens,
    /// Session parameters returned by the server
    pub session_parameters: Option<HashMap<String, String>>,
}

impl SessionTokens {
    /// Check if the master token is expired or about to expire
    pub fn is_master_expired(&self) -> bool {
        self.master_expires_at
            .map(|exp| exp < std::time::Instant::now())
            .unwrap_or(false)
    }

    /// Check if the session token is expired or about to expire
    pub fn is_session_expired(&self) -> bool {
        self.session_expires_at
            .map(|exp| exp < std::time::Instant::now())
            .unwrap_or(false)
    }

    /// Get remaining validity for the master token
    pub fn master_valid_for(&self) -> Option<std::time::Duration> {
        self.master_expires_at
            .and_then(|exp| exp.checked_duration_since(std::time::Instant::now()))
    }
}

/// Response from the session token refresh endpoint
#[derive(Debug, serde::Deserialize)]
struct RefreshSessionResponse {
    data: Option<RefreshSessionData>,
    message: Option<String>,
    code: Option<String>,
    success: bool,
}

#[derive(Debug, serde::Deserialize)]
struct RefreshSessionData {
    #[serde(rename = "sessionToken")]
    session_token: String,
    #[serde(rename = "masterToken")]
    master_token: String,
    #[serde(rename = "sessionId")]
    session_id: i64,
    #[serde(
        rename = "validityInSecondsST",
        deserialize_with = "auth::deserialize_seconds_as_duration",
        default
    )]
    validity: Option<std::time::Duration>,
    #[serde(
        rename = "validityInSecondsMT",
        deserialize_with = "auth::deserialize_seconds_as_duration",
        default
    )]
    master_validity: Option<std::time::Duration>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum QueryExecutionMode {
    Blocking,
    Async,
}

#[derive(Clone)]
pub struct QueryInput<'a> {
    pub sql: String,
    pub bindings: Option<&'a RawValue>,
    pub describe_only: Option<bool>,
}

pub fn user_agent(client_info: &ClientInfo) -> String {
    format!(
        "{}/{} ({}) CPython/3.11.6",
        client_info.application,
        client_info.version.clone(),
        client_info.os.clone()
    )
}

fn base_auth_request_data(login_parameters: &LoginParameters) -> AuthRequestData {
    AuthRequestData {
        account_name: login_parameters.account_name.clone(),
        client_app_id: login_parameters.client_info.application.clone(),
        client_app_version: login_parameters.client_info.version.clone(),
        client_environment: AuthRequestClientEnvironment {
            application: login_parameters.client_info.application.clone(),
            os: login_parameters.client_info.os.clone(),
            os_version: login_parameters.client_info.os_version.clone(),
            ocsp_mode: login_parameters.client_info.ocsp_mode.clone(),
            python_version: Some("3.11.6".to_string()),
            python_runtime: Some("CPython".to_string()),
            python_compiler: Some("Clang 13.0.0 (clang-1300.0.29.30)".to_string()),
        },
        ..Default::default()
    }
}

pub async fn auth_request_data(
    client: &reqwest::Client,
    login_parameters: &LoginParameters,
    session_parameters: Option<&HashMap<String, String>>,
) -> Result<AuthRequestData, RestError> {
    let mut data = base_auth_request_data(login_parameters);

    if let Some(params) = session_parameters {
        let json_params = params
            .iter()
            .map(|(k, v)| (k.clone(), serde_json::Value::String(v.clone())))
            .collect();
        data.session_parameters = Some(json_params);
    }

    match &login_parameters.login_method {
        LoginMethod::NativeOkta(okta_config) => {
            let retry_policy = RetryPolicy::default();
            let saml_html =
                fetch_native_okta_saml(client, login_parameters, &retry_policy, okta_config)
                    .await
                    .context(NativeOktaSnafu)?;

            data.login_name = Some(okta_config.username.clone());
            data.authenticator = Some(okta_config.okta_url.to_string());
            data.raw_saml_response = Some(saml_html);
        }
        _ => match create_credentials(login_parameters).context(AuthenticationSnafu)? {
            Credentials::Password { username, password } => {
                data.login_name = Some(username);
                data.password = Some(password);
                data.authenticator = Some("SNOWFLAKE".to_string());
            }
            Credentials::Jwt { username, token } => {
                data.login_name = Some(username);
                data.token = Some(token);
                data.authenticator = Some("SNOWFLAKE_JWT".to_string());
            }
            Credentials::Pat { username, token } => {
                data.login_name = Some(username);
                data.token = Some(token);
                data.authenticator = Some("PROGRAMMATIC_ACCESS_TOKEN".to_string());
            }
        },
    }
    Ok(data)
}

#[tracing::instrument(
    skip(login_parameters, session_parameters),
    fields(account_name, login_name)
)]
pub async fn snowflake_login(
    login_parameters: &LoginParameters,
    session_parameters: Option<&HashMap<String, String>>,
) -> Result<LoginResult, RestError> {
    let client = build_tls_http_client(&login_parameters.client_info)?;
    snowflake_login_with_client(&client, login_parameters, session_parameters).await
}

#[tracing::instrument(
    skip(client, login_parameters, session_parameters),
    fields(account_name, login_name)
)]
pub async fn snowflake_login_with_client(
    client: &reqwest::Client,
    login_parameters: &LoginParameters,
    session_parameters: Option<&HashMap<String, String>>,
) -> Result<LoginResult, RestError> {
    tracing::info!("Starting Snowflake login process");

    // Record key fields in the span
    tracing::Span::current().record("account_name", &login_parameters.account_name);

    // Optional settings
    tracing::debug!(
        account_name = %login_parameters.account_name,
        server_url = %login_parameters.server_url,
        database = ?login_parameters.database,
        schema = ?login_parameters.schema,
        warehouse = ?login_parameters.warehouse,
        "Extracted connection settings"
    );

    // Build the login request data (handles all auth methods including Okta SAML exchange)
    let auth_request_data = auth_request_data(client, login_parameters, session_parameters).await?;
    tracing::Span::current().record("login_name", &auth_request_data.login_name);
    let login_request = AuthRequest {
        data: auth_request_data,
    };

    tracing::debug!(
        "Login request: {}",
        serde_json::to_string_pretty(&login_request).unwrap()
    );

    let login_url = format!("{}/session/v1/login-request", login_parameters.server_url);
    tracing::info!(login_url = %login_url, "Making Snowflake login request");
    let request = client
        .post(&login_url)
        .query(&[
            (
                "databaseName",
                login_parameters.database.as_deref().unwrap_or_default(),
            ),
            (
                "schemaName",
                login_parameters.schema.as_deref().unwrap_or_default(),
            ),
            (
                "warehouse",
                login_parameters.warehouse.as_deref().unwrap_or_default(),
            ),
            (
                "roleName",
                login_parameters.role.as_deref().unwrap_or_default(),
            ),
        ])
        .json(&login_request)
        .header("accept", "application/snowflake")
        .header(
            "User-Agent",
            format!(
                "{}/{} ({}) CPython/3.11.6",
                login_parameters.client_info.application,
                login_parameters.client_info.version.clone(),
                login_parameters.client_info.os.clone()
            ),
        )
        .header("Authorization", "Snowflake Token=\"None\"")
        .build()
        .context(RequestConstructionSnafu { request: "login" })?;

    let response = client.execute(request).await.context(CommunicationSnafu {
        context: "Failed to execute login request",
    })?;

    let auth_response = read_response_json::<AuthResponse>(response)
        .await
        .context(InvalidSnowflakeResponseSnafu)?;

    if !auth_response.success {
        let message = auth_response
            .message
            .unwrap_or_else(|| "Unknown error".to_string());
        tracing::error!(message = %message, "Snowflake login failed");
        let code = auth_response
            ._code
            .as_deref()
            .and_then(|c| c.parse::<i32>().ok())
            .unwrap_or(-1);
        LoginSnafu { message, code }.fail()?;
    }

    tracing::debug!("Login successful, extracting session tokens");

    let session_token = auth_response
        .data
        .token
        .context(MissingResponseFieldSnafu {
            field: "session token",
        })?;

    let master_token = auth_response
        .data
        .master_token
        .context(MissingResponseFieldSnafu {
            field: "master token",
        })?;

    let session_id = auth_response
        .data
        .session_id
        .context(MissingResponseFieldSnafu {
            field: "session ID",
        })?;

    let now = std::time::Instant::now();
    let session_expires_at = auth_response.data.validity.map(|d| now + d);
    let master_expires_at = auth_response.data.master_validity.map(|d| now + d);

    // Extract session parameters from auth response
    let session_params = auth_response.data._parameters.map(|params| {
        params
            .iter()
            .filter_map(|param| {
                // Convert JSON value to string
                let value_str = match &param._value {
                    serde_json::Value::String(s) => Some(s.clone()),
                    serde_json::Value::Number(n) => Some(n.to_string()),
                    serde_json::Value::Bool(b) => Some(b.to_string()),
                    serde_json::Value::Null => None,
                    other => {
                        tracing::debug!(
                            param_name = %param._name,
                            param_value = ?other,
                            "Unexpected JSON type for session parameter, skipping"
                        );
                        None
                    }
                };

                value_str.map(|v| (param._name.to_uppercase(), v))
            })
            .collect::<HashMap<String, String>>()
    });

    tracing::info!(
        session_id,
        session_validity_secs = auth_response.data.validity.map(|d| d.as_secs()),
        master_validity_secs = auth_response.data.master_validity.map(|d| d.as_secs()),
        session_params_count = session_params.as_ref().map(|p| p.len()),
        "Snowflake login completed successfully"
    );
    Ok(LoginResult {
        tokens: SessionTokens {
            session_token,
            master_token,
            session_id,
            session_expires_at,
            master_expires_at,
        },
        session_parameters: session_params,
    })
}

/// Refresh an expired session token using the master token.
///
/// When a session token expires (indicated by HTTP 401), this function can be called
/// to obtain new tokens without requiring a full re-authentication.
#[tracing::instrument(skip(client, client_info, tokens))]
pub async fn refresh_session(
    client: &reqwest::Client,
    server_url: &str,
    client_info: &ClientInfo,
    tokens: &SessionTokens,
) -> Result<SessionTokens, RestError> {
    tracing::info!(session_id = tokens.session_id, "Refreshing session token");

    let refresh_url = Url::parse(server_url)
        .and_then(|base| base.join(TOKEN_REQUEST_PATH))
        .context(UrlJoinSnafu {
            path: TOKEN_REQUEST_PATH,
        })?;

    // Build request body per gosnowflake: {"oldSessionToken": "...", "requestType": "RENEW"}
    let body = serde_json::json!({
        "oldSessionToken": tokens.session_token,
        "requestType": "RENEW"
    });

    let request = client
        .post(refresh_url)
        .query(&[
            ("requestId", uuid::Uuid::new_v4().to_string()),
            ("request_guid", uuid::Uuid::new_v4().to_string()),
        ])
        // Authenticate with master token, not session token
        .header(
            header::AUTHORIZATION,
            format!("Snowflake Token=\"{}\"", tokens.master_token),
        )
        .header(header::ACCEPT, "application/json")
        .header("User-Agent", user_agent(client_info))
        .json(&body)
        .build()
        .context(RequestConstructionSnafu {
            request: "session refresh",
        })?;

    let response = client.execute(request).await.context(CommunicationSnafu {
        context: "Failed to execute session refresh request",
    })?;

    let status = response.status();
    if !status.is_success() {
        tracing::error!(status = %status, "Session refresh request failed");
        return SessionRefreshSnafu { status }.fail();
    }

    let refresh_response =
        response
            .json::<RefreshSessionResponse>()
            .await
            .context(CommunicationSnafu {
                context: "Failed to parse session refresh response",
            })?;

    if !refresh_response.success {
        let message = refresh_response
            .message
            .unwrap_or_else(|| "Unknown error".to_string());
        let code = refresh_response
            .code
            .as_deref()
            .and_then(|c| c.parse::<i32>().ok())
            .unwrap_or(-1);
        tracing::error!(code, message = %message, "Session refresh failed");
        return SessionRefreshFailedSnafu { message, code }.fail();
    }

    let data = refresh_response.data.context(MissingResponseFieldSnafu {
        field: "session refresh data",
    })?;

    let now = std::time::Instant::now();
    let session_expires_at = data.validity.map(|d| now + d);
    let master_expires_at = data.master_validity.map(|d| now + d);

    tracing::info!(
        session_id = data.session_id,
        session_validity_secs = data.validity.map(|d| d.as_secs()),
        master_validity_secs = data.master_validity.map(|d| d.as_secs()),
        "Session refreshed successfully"
    );

    Ok(SessionTokens {
        session_token: data.session_token,
        master_token: data.master_token,
        session_id: data.session_id,
        session_expires_at,
        master_expires_at,
    })
}

#[tracing::instrument(skip(query_parameters, session_token, query_input), fields(sql))]
pub async fn snowflake_query<'a>(
    query_parameters: QueryParameters,
    session_token: String,
    query_input: QueryInput<'a>,
    execution_mode: QueryExecutionMode,
) -> Result<query_response::Response, RestError> {
    let client = build_tls_http_client(&query_parameters.client_info)?;
    let policy = RetryPolicy::default();
    snowflake_query_with_client(
        &client,
        query_parameters,
        session_token,
        query_input,
        &policy,
        execution_mode,
    )
    .await
}

#[tracing::instrument(
    skip(client, query_parameters, session_token, query_input),
    fields(sql)
)]
pub async fn snowflake_query_with_client<'a>(
    client: &reqwest::Client,
    query_parameters: QueryParameters,
    session_token: String,
    query_input: QueryInput<'a>,
    retry_policy: &RetryPolicy,
    execution_mode: QueryExecutionMode,
) -> Result<query_response::Response, RestError> {
    // Async mode path (legacy, opt-in)
    if matches!(execution_mode, QueryExecutionMode::Async) {
        return execute_async_with_fallback(
            client,
            &query_parameters,
            session_token,
            query_input,
            retry_policy,
        )
        .await;
    }

    // Sync mode (default): use requestId-based retry for connection failures
    execute_sync_with_retry(
        client,
        &query_parameters,
        &session_token,
        &query_input,
        retry_policy,
    )
    .await
}

/// Execute query in async mode with fallback to sync for error 612.
async fn execute_async_with_fallback<'a>(
    client: &reqwest::Client,
    query_parameters: &QueryParameters,
    session_token: String,
    query_input: QueryInput<'a>,
    retry_policy: &RetryPolicy,
) -> Result<query_response::Response, RestError> {
    match snowflake_query_async_style(
        client,
        query_parameters,
        session_token.clone(),
        &query_input,
        retry_policy,
    )
    .await
    {
        Ok(response) => return Ok(response),
        Err(RestError::AsyncQuery {
            source:
                SfError::AsyncPollResultNotFound {
                    is_first_poll: true,
                    ..
                },
            ..
        }) => {
            // Error 612 "Result not found" on first poll - fall through to sync retry.
        }
        Err(
            e @ RestError::AsyncQuery {
                source:
                    SfError::AsyncPollResultNotFound {
                        is_first_poll: false,
                        ..
                    },
                ..
            },
        ) => {
            tracing::error!(
                sql_prefix = query_input.sql.chars().take(50).collect::<String>(),
                "Error 612 after prior successful polls; not retrying"
            );
            return Err(e);
        }
        Err(e) => return Err(e),
    }

    // Fallback to sync after 612
    let response = execute_sync_with_retry(
        client,
        query_parameters,
        &session_token,
        &query_input,
        retry_policy,
    )
    .await?;

    // Log based on actual command type after sync completes (we always get here via 612)
    let is_file_transfer = response
        .data
        .command
        .as_deref()
        .map(|c| c.eq_ignore_ascii_case("UPLOAD") || c.eq_ignore_ascii_case("DOWNLOAD"))
        .unwrap_or(false);
    if is_file_transfer {
        tracing::info!(
            command = response.data.command.as_deref(),
            "Retried async 612 with sync; confirmed file transfer"
        );
    } else {
        tracing::warn!(
            command = response.data.command.as_deref(),
            "Retried async 612 with sync; unexpected non-file-transfer query"
        );
    }

    Ok(response)
}

/// Execute query synchronously with requestId-based retry on transport failures.
///
/// On connection errors (network timeout, connection reset), the query is retried
/// with the same `requestId` and `retry=true`. Snowflake uses requestId for
/// idempotency - if the original query completed, the retry returns the existing result.
async fn execute_sync_with_retry<'a>(
    client: &reqwest::Client,
    query_parameters: &QueryParameters,
    session_token: &str,
    query_input: &QueryInput<'a>,
    retry_policy: &RetryPolicy,
) -> Result<query_response::Response, RestError> {
    // Generate requestId upfront - persisted across retries for idempotency
    let request_id = uuid::Uuid::new_v4();
    let sql_prefix = query_input.sql.chars().take(50).collect::<String>();

    tracing::debug!(
        request_id = %request_id,
        sql_prefix,
        "Executing sync query"
    );

    // First attempt
    match execute_sync_query(
        client,
        query_parameters,
        session_token,
        query_input,
        request_id,
        false, // not a retry
    )
    .await
    {
        Ok(response) => return Ok(response),
        Err(RestError::Communication {
            context, source, ..
        }) => {
            // Transport error - retry with same requestId
            tracing::warn!(
                request_id = %request_id,
                error = %source,
                context,
                "Transport error on sync query; retrying with same requestId"
            );
        }
        Err(e) => return Err(e),
    }

    // Retry with retry=true - Snowflake will return existing result if query completed
    let max_retries = retry_policy.max_attempts.saturating_sub(1).max(1);
    let mut last_error = None;

    for attempt in 1..=max_retries {
        let backoff = std::time::Duration::from_millis(
            (retry_policy.backoff.base.as_millis() as f64
                * retry_policy.backoff.factor.powi(attempt as i32)) as u64,
        )
        .min(retry_policy.backoff.cap);

        tokio::time::sleep(backoff).await;

        tracing::info!(
            request_id = %request_id,
            attempt,
            max_retries,
            backoff_ms = backoff.as_millis(),
            "Retrying sync query with retry=true"
        );

        match execute_sync_query(
            client,
            query_parameters,
            session_token,
            query_input,
            request_id,
            true, // is retry
        )
        .await
        {
            Ok(response) => {
                tracing::info!(
                    request_id = %request_id,
                    attempt,
                    query_id = response.data.query_id.as_deref().unwrap_or_default(),
                    "Sync query retry succeeded"
                );
                return Ok(response);
            }
            Err(RestError::Communication {
                context, source, ..
            }) => {
                tracing::warn!(
                    request_id = %request_id,
                    attempt,
                    error = %source,
                    context,
                    "Transport error on retry; will try again"
                );
                last_error = Some(CommunicationSnafu { context }.into_error(source));
            }
            Err(e) => return Err(e),
        }
    }

    // Exhausted retries - return the last transport error
    tracing::error!(
        request_id = %request_id,
        "Exhausted all retry attempts for sync query"
    );
    Err(last_error.expect("last_error must be set after retry loop"))
}

/// Execute a single sync query request.
async fn execute_sync_query<'a>(
    client: &reqwest::Client,
    query_parameters: &QueryParameters,
    session_token: &str,
    query_input: &QueryInput<'a>,
    request_id: uuid::Uuid,
    is_retry: bool,
) -> Result<query_response::Response, RestError> {
    let query_request = query_request::Request {
        sql_text: query_input.sql.clone(),
        async_exec: false,
        sequence_id: 1,
        query_submission_time: SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis() as i64,
        is_internal: false,
        describe_only: query_input.describe_only,
        parameters: None,
        bindings: query_input.bindings,
        bind_stage: None,
        query_context: query_request::QueryContext { entries: None },
    };

    let query_url = Url::parse(query_parameters.server_url.as_str())
        .and_then(|base| base.join(QUERY_REQUEST_PATH))
        .context(UrlJoinSnafu {
            path: QUERY_REQUEST_PATH,
        })?;

    // Build query parameters - include retry=true if this is a retry
    let mut query_params = vec![
        ("requestId", request_id.to_string()),
        ("request_guid", uuid::Uuid::new_v4().to_string()),
    ];
    if is_retry {
        query_params.push(("retry", "true".to_string()));
    }

    let request = apply_json_content_type(apply_query_headers(
        client.post(query_url),
        &query_parameters.client_info,
        session_token,
    ))
    .query(&query_params)
    .json(&query_request)
    .build()
    .context(RequestConstructionSnafu { request: "query" })?;

    let send_start = Instant::now();
    let response = client.execute(request).await.context(CommunicationSnafu {
        context: "Failed to execute query request",
    })?;

    let query_response = read_response_json::<query_response::Response>(response)
        .await
        .context(InvalidSnowflakeResponseSnafu)?;

    let elapsed_ms = send_start.elapsed().as_secs_f64() * 1000.0;
    tracing::debug!(
        elapsed_ms,
        request_id = %request_id,
        is_retry,
        query_id = query_response.data.query_id.as_deref().unwrap_or_default(),
        "Sync query completed"
    );

    if !query_response.success {
        let message = query_response
            .message
            .unwrap_or_else(|| "Unknown error".to_owned());
        return QueryFailedSnafu { message }.fail();
    }
    Ok(query_response)
}

/// New blocking facade that uses the async engine under the hood.
#[tracing::instrument(
    skip(client, query_parameters, session_token, query_input),
    fields(sql)
)]
pub async fn snowflake_query_async_style<'a>(
    client: &reqwest::Client,
    query_parameters: &QueryParameters,
    session_token: String,
    query_input: &QueryInput<'a>,
    retry_policy: &RetryPolicy,
) -> Result<query_response::Response, RestError> {
    let request_id = uuid::Uuid::new_v4();
    crate::rest::snowflake::async_exec::execute_blocking_with_async(
        client,
        query_parameters,
        &session_token,
        query_input,
        request_id,
        retry_policy,
    )
    .await
    .context(AsyncQuerySnafu)
}

async fn read_response_json<T>(response: reqwest::Response) -> Result<T, SnowflakeResponseError>
where
    T: serde::de::DeserializeOwned,
{
    let response_status = response.status();
    let response_text = response.text().await;

    if !response_status.is_success() {
        // Return SessionExpired so caller can refresh and retry
        if response_status == reqwest::StatusCode::UNAUTHORIZED {
            return SessionExpiredSnafu.fail();
        }
        return ResponseStatusSnafu {
            status: response_status,
            message: response_text.unwrap_or("Unknown error".to_string()),
        }
        .fail();
    }

    let response_text = response_text.context(ResponseTextSnafu)?;

    tracing::debug!("Response text: {response_text}");
    let response_data: T = serde_json::from_str(&response_text).context(ResponseFormatSnafu)?;

    Ok(response_data)
}

#[track_caller]
fn build_tls_http_client(client_info: &ClientInfo) -> Result<reqwest::Client, RestError> {
    create_tls_client_with_config(client_info.tls_config.clone()).context(CrlValidationSnafu)
}

pub(crate) fn authorization_header(session_token: &str) -> header::HeaderValue {
    let value = format!("Snowflake Token=\"{session_token}\"");
    header::HeaderValue::from_str(&value).expect("authorization header construction must succeed")
}

pub(crate) fn json_header_value() -> header::HeaderValue {
    header::HeaderValue::from_static("application/json")
}

pub(crate) fn apply_query_headers(
    builder: reqwest::RequestBuilder,
    client_info: &ClientInfo,
    session_token: &str,
) -> reqwest::RequestBuilder {
    builder
        .header(header::AUTHORIZATION, authorization_header(session_token))
        .header(header::ACCEPT, json_header_value())
        .header("User-Agent", user_agent(client_info))
}

pub(crate) fn apply_json_content_type(builder: reqwest::RequestBuilder) -> reqwest::RequestBuilder {
    builder.header(header::CONTENT_TYPE, json_header_value())
}

#[derive(Debug, Snafu, error_trace::ErrorTrace)]
pub enum RestError {
    #[snafu(display("Authentication failed"))]
    Authentication {
        source: AuthError,
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("Native Okta SSO failed"))]
    NativeOkta {
        source: native_okta::NativeOktaError,
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("Invalid Snowflake response"))]
    InvalidSnowflakeResponse {
        source: SnowflakeResponseError,
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("Failed to communicate with Snowflake"))]
    Communication {
        context: String,
        source: reqwest::Error,
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("Failed to build request: {request}"))]
    RequestConstruction {
        request: String,
        source: reqwest::Error,
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("TLS client creation failed"))]
    CrlValidation {
        source: TlsError,
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("Login error: {message}, code: {code}"))]
    LoginError {
        message: String,
        code: i32,
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("Async Snowflake query failed"))]
    AsyncQuery {
        source: SfError,
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("Failed to build Snowflake URL for {path}: {source}"))]
    UrlJoin {
        path: &'static str,
        source: url::ParseError,
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("Session refresh HTTP request failed with status {status}"))]
    SessionRefresh {
        status: reqwest::StatusCode,
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("Session refresh failed: {message} (code: {code})"))]
    SessionRefreshFailed {
        message: String,
        code: i32,
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("Missing response field: {field}"))]
    MissingResponseField {
        field: &'static str,
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("Query failed: {message}"))]
    QueryFailed {
        message: String,
        #[snafu(implicit)]
        location: Location,
    },
}
#[derive(Debug, Snafu, error_trace::ErrorTrace)]
pub enum SnowflakeResponseError {
    #[snafu(display("Failed to parse Snowflake response"))]
    ResponseFormat {
        source: serde_json::Error,
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("Failed to read Snowflake response text"))]
    ResponseText {
        source: reqwest::Error,
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("Snowflake responded with error status: {status}, message: {message}"))]
    ResponseStatus {
        status: reqwest::StatusCode,
        message: String,
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("Session expired - reauthentication required"))]
    SessionExpired {
        #[snafu(implicit)]
        location: Location,
    },
}
