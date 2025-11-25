#![allow(clippy::result_large_err)]
pub mod async_exec;
mod auth;
pub mod error;
pub mod query_request;
pub mod query_response;

use crate::auth::{AuthError, Credentials, create_credentials};
use crate::config::rest_parameters::ClientInfo;
use crate::config::rest_parameters::{LoginParameters, QueryParameters};
use crate::config::retry::RetryPolicy;
use crate::rest::snowflake::auth::{
    AuthRequest, AuthRequestClientEnvironment, AuthRequestData, AuthResponse,
};
use crate::rest::snowflake::error::SfError;
use crate::tls::client::create_tls_client_with_config;
use crate::tls::error::TlsError;
use reqwest::{self, header};
use serde_json;
use snafu::{Location, ResultExt, Snafu};
use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};
use tracing;
use url::Url;

pub const STATEMENT_ASYNC_EXECUTION_OPTION: &str = "async_execution";
pub(crate) const QUERY_REQUEST_PATH: &str = "/queries/v1/query-request";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum QueryExecutionMode {
    Blocking,
    Async,
}

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
    }
    Ok(data)
}

#[tracing::instrument(skip(login_parameters), fields(account_name, login_name))]
pub async fn snowflake_login(login_parameters: &LoginParameters) -> Result<String, RestError> {
    let client = build_tls_http_client(&login_parameters.client_info)?;
    snowflake_login_with_client(&client, login_parameters).await
}

#[tracing::instrument(skip(client, login_parameters), fields(account_name, login_name))]
pub async fn snowflake_login_with_client(
    client: &reqwest::Client,
    login_parameters: &LoginParameters,
) -> Result<String, RestError> {
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

    tracing::debug!("Login successful, extracting session token");
    if let Some(token) = auth_response.data.token {
        tracing::info!("Snowflake login completed successfully");
        Ok(token)
    } else {
        tracing::error!("Login response missing token data");
        InvalidResponseSnafu {
            message: "Login response missing token".to_string(),
        }
        .fail()
        .context(InvalidSnowflakeResponseSnafu)?
    }
}

#[tracing::instrument(skip(query_parameters, session_token, parameter_bindings), fields(sql))]
pub async fn snowflake_query(
    query_parameters: QueryParameters,
    session_token: String,
    sql: String,
    parameter_bindings: Option<HashMap<String, query_request::BindParameter>>,
    execution_mode: QueryExecutionMode,
) -> Result<query_response::Response, RestError> {
    let client = build_tls_http_client(&query_parameters.client_info)?;
    let policy = RetryPolicy::default();
    snowflake_query_with_client(
        &client,
        query_parameters,
        session_token,
        sql,
        parameter_bindings,
        &policy,
        execution_mode,
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
    retry_policy: &RetryPolicy,
    execution_mode: QueryExecutionMode,
) -> Result<query_response::Response, RestError> {
    if matches!(execution_mode, QueryExecutionMode::Async) {
        return snowflake_query_async_style(
            client,
            &query_parameters,
            session_token,
            sql,
            parameter_bindings,
            retry_policy,
        )
        .await;
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
        describe_only: None,
        parameters: None,
        bindings: parameter_bindings,
        bind_stage: None,
        query_context: query_request::QueryContext { entries: None },
    };

    let json_payload = serde_json::to_string_pretty(&query_request).unwrap();
    tracing::debug!("JSON Body Sent:\n{}", json_payload);
    let query_url = Url::parse(query_parameters.server_url.as_str())
        .and_then(|base| base.join(QUERY_REQUEST_PATH))
        .context(UrlJoinSnafu {
            path: QUERY_REQUEST_PATH,
        })?;

    let request = apply_json_content_type(apply_query_headers(
        client.post(query_url),
        &query_parameters.client_info,
        &session_token,
    ))
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

    let query_response = read_response_json::<query_response::Response>(response)
        .await
        .context(InvalidSnowflakeResponseSnafu)?;

    if !query_response.success {
        let message = query_response
            .message
            .unwrap_or_else(|| "Unknown error".to_string());
        InvalidResponseSnafu { message }
            .fail()
            .context(InvalidSnowflakeResponseSnafu)
    } else {
        Ok(query_response)
    }
}

/// New blocking facade that uses the async engine under the hood.
#[tracing::instrument(
    skip(client, query_parameters, session_token, parameter_bindings),
    fields(sql)
)]
pub async fn snowflake_query_async_style(
    client: &reqwest::Client,
    query_parameters: &QueryParameters,
    session_token: String,
    sql: String,
    parameter_bindings: Option<HashMap<String, query_request::BindParameter>>,
    retry_policy: &RetryPolicy,
) -> Result<query_response::Response, RestError> {
    let request_id = uuid::Uuid::new_v4();
    crate::rest::snowflake::async_exec::execute_blocking_with_async(
        client,
        query_parameters,
        &session_token,
        sql,
        parameter_bindings,
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
