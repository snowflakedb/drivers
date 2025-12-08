mod auth;
pub mod bind_uploader;
pub mod query_request;
pub mod query_response;

use crate::auth::{AuthError, Credentials, create_credentials};
use crate::config::rest_parameters::ClientInfo;
use crate::config::rest_parameters::{LoginParameters, QueryParameters};
use crate::rest::snowflake::auth::{
    AuthRequest, AuthRequestClientEnvironment, AuthRequestData, AuthResponse,
};
use reqwest;
use serde_json;
use snafu::{Location, ResultExt, Snafu};
use std::collections::HashMap;
use std::env;

#[derive(Debug, Clone)]
pub struct LoginResult {
    pub session_token: String,
    pub session_timezone: Option<String>,
}

pub(crate) fn force_json_rowset() -> bool {
    env::var("UNIVERSAL_FORCE_JSON_ROWSET")
        .map(|v| {
            let lower = v.to_ascii_lowercase();
            lower == "1" || lower == "true" || lower == "json"
        })
        .unwrap_or(false)
}

fn should_retry_arrow_with_json(err: &RestError) -> bool {
    match err {
        RestError::QueryFailed {
            error_code,
            message,
            ..
        } => {
            matches!(error_code, Some(300002))
                || message.contains("error 300002")
                || message
                    .to_ascii_lowercase()
                    .contains("processing aborted due to error 300002")
        }
        _ => false,
    }
}

async fn enforce_json_rowset(
    client: &reqwest::Client,
    query_parameters: QueryParameters,
    session_token: String,
) -> Result<(), RestError> {
    let stmt = "alter session set ODBC_QUERY_RESULT_FORMAT='JSON', GO_QUERY_RESULT_FORMAT='JSON'";
    snowflake_query_impl(
        client,
        query_parameters,
        session_token,
        stmt.to_string(),
        None,
        None,
        false,
        None,
        true,
    )
    .await
    .map(|_| ())
}
use std::time::{SystemTime, UNIX_EPOCH};
use tracing;
use uuid::Uuid;

pub fn user_agent(client_info: &ClientInfo) -> String {
    format!(
        "{}/{} ({}) CPython/3.11.6",
        client_info.application,
        client_info.version.clone(),
        client_info.os.clone()
    )
}

pub fn auth_request_data(login_parameters: &LoginParameters) -> Result<AuthRequestData, RestError> {
    let mut data = AuthRequestData {
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
    };

    match create_credentials(login_parameters).context(AuthenticationSnafu)? {
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
        Credentials::OAuth { username, token } => {
            data.login_name = Some(username);
            data.token = Some(token);
            data.authenticator = Some("OAUTH".to_string());
        }
        Credentials::ExternalBrowser { username, token } => {
            data.login_name = Some(username);
            data.token = Some(token);
            data.authenticator = Some("EXTERNALBROWSER".to_string());
        }
    }
    Ok(data)
}

#[tracing::instrument(skip(login_parameters, client), fields(account_name, login_name))]
pub async fn snowflake_login_with_client(
    client: &reqwest::Client,
    login_parameters: &LoginParameters,
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

    // Build the login request
    let auth_request_data = auth_request_data(login_parameters)?;
    tracing::Span::current().record("login_name", &auth_request_data.login_name);
    let login_request = AuthRequest {
        data: auth_request_data,
    };

    tracing::debug!(
        "Login request: {}",
        serde_json::to_string_pretty(&login_request).unwrap()
    );

    // Use provided HTTP client
    tracing::debug!("Preparing login request with provided HTTP client");
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
            .map(|c| c.parse::<i32>().unwrap_or(-1))
            .unwrap_or(-1);
        LoginSnafu { message, code }.fail()?;
    }

    // Extract and store the session token
    tracing::debug!("Login successful, extracting session token");
    let token = match auth_response.data.token {
        Some(t) => t,
        None => {
            tracing::error!("Login response missing token data");
            return LoginSnafu {
                message: "Login response missing token".to_string(),
                code: -1,
            }
            .fail();
        }
    };

    // Extract session timezone from parameters
    let session_timezone = auth_response
        .data
        ._parameters
        .as_ref()
        .and_then(|params| {
            params
                .iter()
                .find(|p| p._name.eq_ignore_ascii_case("TIMEZONE"))
        })
        .and_then(|param| {
            if let serde_json::Value::String(tz) = &param._value {
                Some(tz.clone())
            } else {
                None
            }
        });

    if let Some(ref tz) = session_timezone {
        tracing::info!("Session timezone: {}", tz);
    }

    tracing::info!("Snowflake login completed successfully");
    Ok(LoginResult {
        session_token: token,
        session_timezone,
    })
}

#[tracing::instrument(skip(login_parameters), fields(account_name, login_name))]
pub async fn snowflake_login(login_parameters: &LoginParameters) -> Result<LoginResult, RestError> {
    let client = reqwest::Client::new();
    snowflake_login_with_client(&client, login_parameters).await
}

/// Internal query function that never uses bind stage upload.
/// Used by bind_uploader to avoid recursion.
pub async fn snowflake_query_internal(
    client: &reqwest::Client,
    query_parameters: QueryParameters,
    session_token: String,
    sql: String,
    parameter_bindings: Option<HashMap<String, query_request::BindParameter>>,
    multi_statement_count: Option<usize>,
    describe_only: bool,
) -> Result<query_response::Response, RestError> {
    snowflake_query_impl(
        client,
        query_parameters,
        session_token,
        sql,
        parameter_bindings,
        multi_statement_count,
        describe_only,
        None, // No bind stage - this is the internal function
        false,
    )
    .await
}

#[tracing::instrument(
    skip(client, query_parameters, session_token, parameter_bindings),
    fields(sql)
)]
pub async fn snowflake_query_with_client(
    client: &reqwest::Client,
    query_parameters: QueryParameters,
    session_token: String,
    sql: String,
    parameter_bindings: Option<HashMap<String, query_request::BindParameter>>,
    multi_statement_count: Option<usize>,
    describe_only: bool,
    session_timezone: Option<String>,
    force_json_override: bool,
) -> Result<query_response::Response, RestError> {
    use std::io::Write;
    let _ = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open("/tmp/rust_debug.log")
        .and_then(|mut f| {
            writeln!(
                f,
                "DEBUG snowflake_query: sql={}, bindings={:?}",
                &sql[..std::cmp::min(100, sql.len())],
                parameter_bindings.as_ref().map(|b| b.len())
            )
        });

    let (final_bindings, bind_stage) = if let Some(ref bindings) = parameter_bindings {
        let force_stage = bindings_require_stage(bindings);
        if force_stage
            || bind_uploader::should_use_bind_stage(
                bindings,
                bind_uploader::DEFAULT_BIND_STAGE_THRESHOLD,
            )
        {
            tracing::info!("Using bind stage upload for large array binding");
            let _ = std::fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open("/tmp/rust_debug.log")
                .and_then(|mut f| {
                    writeln!(
                        f,
                        "DEBUG snowflake_query: Using bind stage upload (force_stage={})",
                        force_stage
                    )
                });

            // Upload bindings to stage with session timezone for proper timestamp formatting
            match bind_uploader::upload_bindings_to_stage(
                client,
                query_parameters.clone(),
                session_token.clone(),
                bindings,
                session_timezone.clone(),
            )
            .await
            {
                Ok(result) => {
                    tracing::info!("Bind stage upload successful: {}", result.stage_path);
                    let metadata_only = bindings
                        .iter()
                        .map(|(k, v)| {
                            let mut param = v.clone();
                            param.value = serde_json::Value::Null;
                            (k.clone(), param)
                        })
                        .collect();
                    (Some(metadata_only), Some(result.stage_path))
                }
                Err(e) => {
                    tracing::warn!(
                        "Bind stage upload failed, falling back to inline bindings: {}",
                        e
                    );
                    // Fall back to inline bindings
                    (parameter_bindings.clone(), None)
                }
            }
        } else {
            // Debug: log the bindings
            let _ = std::fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open("/tmp/rust_debug.log")
                .and_then(|mut f| {
                    writeln!(
                        f,
                        "DEBUG snowflake_query: bindings sample: {:?}",
                        bindings.iter().take(1).collect::<Vec<_>>()
                    )
                });
            (parameter_bindings.clone(), None)
        }
    } else {
        (None, None)
    };

    let base_force_json = force_json_rowset() || force_json_override;
    let mut attempt_force_json = base_force_json;

    loop {
        let result = snowflake_query_impl(
            client,
            query_parameters.clone(),
            session_token.clone(),
            sql.clone(),
            final_bindings.clone(),
            multi_statement_count,
            describe_only,
            bind_stage.clone(),
            attempt_force_json,
        )
        .await;

        match result {
            Ok(resp) => return Ok(resp),
            Err(err) if !attempt_force_json && should_retry_arrow_with_json(&err) => {
                if let Err(enforce_err) =
                    enforce_json_rowset(client, query_parameters.clone(), session_token.clone())
                        .await
                {
                    tracing::warn!(
                        error = %enforce_err,
                        "Failed to enforce JSON rowset before retry; returning original error"
                    );
                    return Err(err);
                }
                tracing::warn!(
                    "Snowflake Arrow rowset failed ({}); enforced JSON rowset and retrying",
                    err
                );
                attempt_force_json = true;
                continue;
            }
            Err(err) => return Err(err),
        }
    }
}

fn bindings_require_stage(bindings: &HashMap<String, query_request::BindParameter>) -> bool {
    bindings.iter().any(|(_, param)| {
        matches!(
            param.type_.as_str(),
            "DATE" | "TIME" | "TIMESTAMP_LTZ" | "TIMESTAMP_NTZ" | "TIMESTAMP_TZ"
        )
    })
}

/// Core query implementation
async fn snowflake_query_impl(
    client: &reqwest::Client,
    query_parameters: QueryParameters,
    session_token: String,
    sql: String,
    parameter_bindings: Option<HashMap<String, query_request::BindParameter>>,
    multi_statement_count: Option<usize>,
    describe_only: bool,
    bind_stage: Option<String>,
    force_json_result: bool,
) -> Result<query_response::Response, RestError> {
    let server_url = query_parameters.server_url.clone();
    let query_url = format!("{server_url}/queries/v1/query-request");

    let mut parameters = HashMap::new();
    let force_json = force_json_rowset() || force_json_result;
    if force_json {
        parameters.insert(
            "GO_QUERY_RESULT_FORMAT".to_string(),
            serde_json::Value::String("JSON".to_string()),
        );
    }

    // Add multi-statement count if specified
    if let Some(count) = multi_statement_count {
        tracing::debug!("Setting MULTI_STATEMENT_COUNT parameter to {count}");
        parameters.insert(
            "MULTI_STATEMENT_COUNT".to_string(),
            serde_json::Value::Number(serde_json::Number::from(count)),
        );
    }

    let query_request = query_request::Request {
        sql_text: sql,
        async_exec: false,
        sequence_id: 1,
        query_submission_time: SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis() as i64,
        is_internal: false,
        describe_only: if describe_only { Some(true) } else { None },
        parameters: if parameters.is_empty() {
            None
        } else {
            Some(parameters)
        },
        bindings: parameter_bindings,
        bind_stage,
        query_context: query_request::QueryContext { entries: None },
    };

    let json_payload = serde_json::to_string_pretty(&query_request).unwrap();
    tracing::debug!("JSON Body Sent:\n{}", json_payload);

    // Debug: log bind_stage usage
    if query_request.bind_stage.is_some() {
        use std::io::Write;
        let _ = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open("/tmp/rust_debug.log")
            .and_then(|mut f| {
                writeln!(
                    f,
                    "DEBUG snowflake_query_impl: Using bind_stage={:?}, bindings={:?}\nJSON: {}",
                    query_request.bind_stage,
                    query_request.bindings.as_ref().map(|b| b.len()),
                    &json_payload[..std::cmp::min(500, json_payload.len())]
                )
            });
    }

    let request = client
        .post(&query_url)
        .header(
            "Authorization",
            &format!("Snowflake Token=\"{session_token}\""),
        )
        // we might want to add some logic to handle different content types later
        .header("Accept", "application/json")
        .header("User-Agent", user_agent(&query_parameters.client_info))
        .query(&[
            ("requestId", uuid::Uuid::new_v4().to_string()),
            ("request_guid", uuid::Uuid::new_v4().to_string()),
        ])
        .json(&query_request)
        .build()
        .context(RequestConstructionSnafu { request: "query" })?;

    tracing::debug!("Query request: {:?}", request);
    tracing::debug!("Request headers: {:?}", request.headers());
    tracing::debug!("Request method: {:?}", request.method());
    tracing::debug!("Request url: {:?}", request.url());
    tracing::debug!("Request version: {:?}", request.version());
    // tracing::debug!("Request content-length: {:?}", request.content_length());
    // tracing::debug!("Request content-type: {:?}", request.content_type());
    // tracing::debug!("Request accept: {:?}", request.accept());
    // tracing::debug!("Request accept-encoding: {:?}", request.accept_encoding());

    let response = client.execute(request).await.context(CommunicationSnafu {
        context: "Failed to execute query request",
    })?;

    let mut query_response = read_response_json::<query_response::Response>(response)
        .await
        .context(InvalidSnowflakeResponseSnafu)?;

    if !query_response.success {
        let message = query_response
            .message
            .unwrap_or_else(|| "Unknown error".to_string());
        return QueryFailedSnafu {
            message,
            sql_state: query_response.data.sql_state.clone(),
            error_code: query_response
                .code
                .as_ref()
                .and_then(|c| c.parse::<i32>().ok()),
            query_id: query_response.data.query_id.clone(),
        }
        .fail();
    }

    let mut attempts = 0;
    while !has_rowset(&query_response.data) {
        if let Some(result_url) = query_response.data.get_result_url.clone() {
            attempts += 1;
            tracing::info!("Polling async query result (attempt {attempts}) at {result_url}");
            query_response =
                fetch_query_result(client, &server_url, &session_token, &result_url).await?;
        } else {
            break;
        }
    }

    Ok(query_response)
}

#[tracing::instrument(skip(query_parameters, session_token, parameter_bindings), fields(sql))]
pub async fn snowflake_query(
    query_parameters: QueryParameters,
    session_token: String,
    sql: String,
    parameter_bindings: Option<HashMap<String, query_request::BindParameter>>,
    multi_statement_count: Option<usize>,
) -> Result<query_response::Response, RestError> {
    let client = reqwest::Client::new();
    snowflake_query_with_client(
        &client,
        query_parameters,
        session_token,
        sql,
        parameter_bindings,
        multi_statement_count,
        false,
        None, // No session timezone available in this context
        false,
    )
    .await
}

#[tracing::instrument(skip(query_parameters, session_token))]
pub async fn cancel_query(
    query_parameters: QueryParameters,
    session_token: String,
    query_id: &str,
) -> Result<(), RestError> {
    let client = reqwest::Client::new();
    cancel_query_with_client(&client, query_parameters, session_token, query_id).await
}

pub async fn cancel_query_with_client(
    client: &reqwest::Client,
    query_parameters: QueryParameters,
    session_token: String,
    query_id: &str,
) -> Result<(), RestError> {
    let server_url = query_parameters.server_url;
    let abort_url = format!("{server_url}/queries/v1/abort-request");
    let request_id = Uuid::new_v4().to_string();
    let payload = serde_json::json!({
        "requestId": request_id,
        "queryId": query_id,
    });

    let request = client
        .post(&abort_url)
        .header(
            "Authorization",
            &format!("Snowflake Token=\"{session_token}\""),
        )
        .header("Accept", "application/json")
        .query(&[("requestId", Uuid::new_v4().to_string())])
        .json(&payload)
        .build()
        .context(RequestConstructionSnafu {
            request: "cancel_query",
        })?;

    let response = client.execute(request).await.context(CommunicationSnafu {
        context: "Failed to execute cancel query request".to_string(),
    })?;

    // Response body is not used, but we validate success status
    let _: serde_json::Value = read_response_json(response)
        .await
        .context(InvalidSnowflakeResponseSnafu)?;

    Ok(())
}

fn has_rowset(data: &query_response::Data) -> bool {
    data.rowset
        .as_ref()
        .map(|rowset| !rowset.is_empty())
        .unwrap_or(false)
        || data.rowset_base64.is_some()
        || data
            .chunks
            .as_ref()
            .map(|chunks| !chunks.is_empty())
            .unwrap_or(false)
}

async fn fetch_query_result(
    client: &reqwest::Client,
    server_url: &str,
    session_token: &str,
    result_path: &str,
) -> Result<query_response::Response, RestError> {
    let result_url = format!("{server_url}{result_path}");
    tracing::debug!("Fetching async query result from {result_url}");
    let request = client
        .get(&result_url)
        .header(
            "Authorization",
            &format!("Snowflake Token=\"{session_token}\""),
        )
        .header("Accept", "application/json")
        .query(&[
            ("requestId", Uuid::new_v4().to_string()),
            ("request_guid", Uuid::new_v4().to_string()),
        ])
        .build()
        .context(RequestConstructionSnafu { request: "query" })?;

    let response = client.execute(request).await.context(CommunicationSnafu {
        context: "Failed to fetch async result",
    })?;

    let query_response = read_response_json::<query_response::Response>(response)
        .await
        .context(InvalidSnowflakeResponseSnafu)?;

    if !query_response.success {
        let message = query_response
            .message
            .unwrap_or_else(|| "Unknown error".to_string());
        return QueryFailedSnafu {
            message,
            sql_state: query_response.data.sql_state.clone(),
            error_code: query_response
                .code
                .as_ref()
                .and_then(|c| c.parse::<i32>().ok()),
            query_id: query_response.data.query_id.clone(),
        }
        .fail();
    }

    Ok(query_response)
}

/// Fetch a child query result for multi-statement queries
pub async fn fetch_child_query_result(
    client: &reqwest::Client,
    server_url: &str,
    session_token: &str,
    result_path: &str,
) -> Result<query_response::Response, RestError> {
    fetch_query_result(client, server_url, session_token, result_path).await
}

async fn read_response_json<T>(response: reqwest::Response) -> Result<T, SnowflakeResponseError>
where
    T: serde::de::DeserializeOwned,
{
    let response_status = response.status();
    let response_text = response.text().await;

    if !response_status.is_success() {
        return ResponseStatusSnafu {
            status: response_status,
            message: response_text.unwrap_or("Unknown error".to_string()),
        }
        .fail();
    }

    let response_text = response_text.context(ResponseTextSnafu)?;

    tracing::debug!("Response text: {}", response_text);
    let response_data: T = serde_json::from_str(&response_text).context(ResponseFormatSnafu)?;

    Ok(response_data)
}

#[derive(Debug, Snafu)]
pub enum RestError {
    #[snafu(display("Authentication failed"))]
    Authentication {
        source: AuthError,
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

    #[snafu(display("Login error: {message}, code: {code}"))]
    LoginError {
        message: String,
        code: i32,
        #[snafu(implicit)]
        location: Location,
    },

    #[snafu(display(
        "Query failed: {message} (sql_state={sql_state:?}, error_code={error_code:?}, query_id={query_id:?})"
    ))]
    QueryFailed {
        message: String,
        sql_state: Option<String>,
        error_code: Option<i32>,
        query_id: Option<String>,
        #[snafu(implicit)]
        location: Location,
    },
}

#[derive(Debug, Snafu)]
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
    #[snafu(display("{message}"))]
    InvalidResponse {
        message: String,
        #[snafu(implicit)]
        location: Location,
    },
}
