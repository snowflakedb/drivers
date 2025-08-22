mod auth;
pub mod query_request;
pub mod query_response;

use crate::auth::{Credentials, create_credentials};
use crate::config::rest_parameters::ClientInfo;
use crate::config::rest_parameters::{LoginParameters, QueryParameters};
use crate::rest::error::RestError;
use crate::rest::snowflake::auth::{
    AuthRequest, AuthRequestClientEnvironment, AuthRequestData, AuthResponse,
};
use reqwest;
use serde_json;
use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};
use tracing;

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

    match create_credentials(login_parameters).map_err(RestError::AuthError)? {
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

    // Create HTTP client
    tracing::debug!("Creating HTTP client and preparing login request");
    let client = reqwest::Client::new();
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
        .header("Authorization", "Snowflake Token=\"None\"");
    let request = request.build().unwrap();
    let response = client.execute(request).await.map_err(|e| {
        tracing::error!(error = %e, "HTTP request failed");
        RestError::Internal(format!("HTTP request failed: {e}"))
    })?;

    let status = response.status();
    tracing::debug!(status = %status, "Received login response");

    let response_text = response.text().await.unwrap_or_else(|_| {
        tracing::error!("Failed to read response body");
        "Unknown error".to_string()
    });
    tracing::debug!(response = %response_text, "Raw login response body");

    if !status.is_success() {
        tracing::error!(status = %status, error_text = %response_text, "Login request failed");
        return Err(RestError::Internal(format!(
            "Login failed with status {status}: {response_text}"
        )));
    }

    // Parse the response
    let auth_response: AuthResponse = serde_json::from_str(&response_text).map_err(|e| {
        RestError::InvalidSnowflakeResponse(format!("Failed to parse login response: {e}"))
    })?;

    if !auth_response.success {
        let message = auth_response
            .message
            .unwrap_or_else(|| "Unknown error".to_string());
        tracing::error!(message = %message, "Snowflake login failed");
        return Err(RestError::Status(status));
    }

    // Extract and store the session token
    tracing::debug!("Login successful, extracting session token");
    if let Some(token) = auth_response.data.token {
        tracing::info!("Snowflake login completed successfully");
        Ok(token)
    } else {
        tracing::error!("Login response missing token data");
        Err(RestError::Internal(
            "Login response missing token data".to_string(),
        ))
    }
}

#[tracing::instrument(skip(query_parameters, session_token, parameter_bindings), fields(sql))]
pub async fn snowflake_query(
    query_parameters: QueryParameters,
    session_token: String,
    sql: String,
    parameter_bindings: Option<HashMap<String, query_request::BindParameter>>,
) -> Result<query_response::Response, RestError> {
    let server_url = query_parameters.server_url;

    let client = reqwest::Client::new();
    let query_url = format!("{server_url}/queries/v1/query-request");

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
        .map_err(|e| {
            tracing::error!(error = %e, "Failed to build query request");
            RestError::Internal(format!("Failed to build query request: {e}"))
        })?;

    tracing::debug!("Query request: {:?}", request);
    tracing::debug!("Request headers: {:?}", request.headers());
    tracing::debug!("Request method: {:?}", request.method());
    tracing::debug!("Request url: {:?}", request.url());
    tracing::debug!("Request version: {:?}", request.version());
    // tracing::debug!("Request content-length: {:?}", request.content_length());
    // tracing::debug!("Request content-type: {:?}", request.content_type());
    // tracing::debug!("Request accept: {:?}", request.accept());
    // tracing::debug!("Request accept-encoding: {:?}", request.accept_encoding());

    let response = client.execute(request).await.map_err(|e| {
        tracing::error!(error = %e, "Failed to execute query request");
        RestError::Internal(format!("Failed to execute query request: {e}"))
    })?;

    if !response.status().is_success() {
        let status = response.status();
        let error_text = response
            .text()
            .await
            .unwrap_or_else(|_| "Unknown error".to_string());
        tracing::error!(status = %status, error_text = %error_text, "Query request failed");
        return Err(RestError::Status(status));
    }

    let response_text = response
        .text()
        .await
        .unwrap_or_else(|_| "Unknown error".to_string());

    tracing::debug!("Query response text: {}", response_text);

    let response_data: query_response::Response =
        serde_json::from_str(&response_text).map_err(|e| {
            tracing::trace!("Response text: {}", response_text);
            RestError::InvalidSnowflakeResponse(format!("Failed to parse query response: {e}"))
        })?;

    Ok(response_data)
}
