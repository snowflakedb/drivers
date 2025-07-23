mod auth;
mod data;
pub mod query;

use crate::driver::{Connection, Setting, Settings};
use crate::rest::error::RestError;
use crate::rest::snowflake::auth::{
    AuthRequest, AuthRequestClientEnvironment, AuthRequestData, AuthResponse,
};
use crate::rest::snowflake::data::{ClientInfo, LoginParameters};
use crate::rest::snowflake::query::{ExecRequest, ExecResponse, RequestQueryContext};
use reqwest;
use serde_json;
use std::collections::HashMap;
use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};
use tracing;

fn client_info(_conn: &Connection) -> Result<ClientInfo, RestError> {
    let client_info = ClientInfo {
        application: "PythonConnector".to_string(),
        version: "3.15.0".to_string(),
        os: "Darwin".to_string(),
        os_version: "macOS-15.5-arm64-arm-64bit".to_string(),
        ocsp_mode: Some("FAIL_OPEN".to_string()),
    };
    Ok(client_info)
}

fn get_server_url(conn: &Connection) -> Result<String, RestError> {
    if let Some(Setting::String(value)) = conn.settings.get("server_url") {
        return Ok(value.clone());
    }

    if let Some(Setting::String(account_name)) = conn.settings.get("account") {
        let protocol = conn
            .settings
            .get_string("protocol")
            .unwrap_or("https".to_string());
        if protocol != "https" && protocol != "http" {
            tracing::warn!(
                "Unexpected protocol specified during server url construction: {protocol}"
            );
        }
        return Ok(format!(
            "{protocol}://{account_name}.snowflakecomputing.com"
        ));
    }

    Err(RestError::MissingParameter(
        "server_url or account".to_string(),
    ))
}

fn get_login_parameters(conn: &Connection) -> Result<LoginParameters, RestError> {
    let params = LoginParameters {
        account_name: {
            tracing::debug!("Getting account name from connection settings");
            tracing::debug!("Connection settings: {:?}", conn.settings);
            if let Some(value) = conn.settings.get_string("account") {
                value
            } else {
                return Err(RestError::MissingParameter("account".to_string()));
            }
        },
        login_name: {
            if let Some(Setting::String(value)) = conn.settings.get("user") {
                value.clone()
            } else {
                return Err(RestError::MissingParameter("user".to_string()));
            }
        },
        password: {
            if let Some(Setting::String(value)) = conn.settings.get("password") {
                value.clone()
            } else {
                return Err(RestError::MissingParameter("password".to_string()));
            }
        },
        server_url: get_server_url(conn)?,
    };
    Ok(params)
}

#[tracing::instrument(skip(conn_ptr), fields(account_name, login_name))]
pub async fn snowflake_login(
    conn_ptr: &std::sync::Arc<Mutex<Connection>>,
) -> Result<(), RestError> {
    tracing::info!("Starting Snowflake login process");

    // Extract required settings from connection
    tracing::debug!("Extracting connection settings");
    let (login_parameters, client_info) = {
        let conn = conn_ptr.lock().unwrap();
        (get_login_parameters(&conn)?, client_info(&conn)?)
    };

    // Record key fields in the span
    tracing::Span::current().record("account_name", &login_parameters.account_name);
    tracing::Span::current().record("login_name", &login_parameters.login_name);

    // Optional settings
    tracing::debug!(
        account_name = %login_parameters.account_name,
        login_name = %login_parameters.login_name,
        server_url = %login_parameters.server_url,
        "Extracted connection settings"
    );

    // Build the login request
    let login_request = AuthRequest {
        data: AuthRequestData {
            client_app_id: client_info.application.clone(),
            client_app_version: client_info.version.clone(),
            _svn_revision: None,
            account_name: login_parameters.account_name,
            login_name: Some(login_parameters.login_name),
            password: Some(login_parameters.password),
            browser_mode_redirect_port: None,
            proof_key: None,
            client_environment: AuthRequestClientEnvironment {
                application: client_info.application.clone(),
                os: client_info.os.clone(),
                os_version: client_info.os_version.clone(),
                ocsp_mode: client_info.ocsp_mode,
                python_version: Some("3.11.6".to_string()),
                python_runtime: Some("CPython".to_string()),
                python_compiler: Some("Clang 13.0.0 (clang-1300.0.29.30)".to_string()),
            },
            session_parameters: Some(HashMap::new()),
            authenticator: Some("snowflake".to_string()),
            raw_saml_response: None,
            ext_authn_duo_method: None,
            passcode: None,
            token: None,
            oauth_type: None,
            provider: None,
        },
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
        .json(&login_request)
        .header("accept", "application/snowflake")
        .header(
            "User-Agent",
            format!(
                "{}/{} ({}) CPython/3.11.6",
                client_info.application,
                client_info.version.clone(),
                client_info.os.clone()
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
        conn_ptr.lock().unwrap().session_token = Some(token);
        tracing::info!("Snowflake login completed successfully");
        Ok(())
    } else {
        tracing::error!("Login response missing token data");
        Err(RestError::Internal(
            "Login response missing token data".to_string(),
        ))
    }
}

pub async fn snowflake_query(
    conn_ptr: &std::sync::Arc<Mutex<Connection>>,
    sql: String,
) -> Result<ExecResponse, RestError> {
    let (session_token, server_url, client_info) = {
        let conn = conn_ptr.lock().unwrap();
        let session_token = conn
            .session_token
            .clone()
            .ok_or(RestError::Internal("Session token not found".to_string()))?;
        (session_token, get_server_url(&conn)?, client_info(&conn)?)
    };

    let client = reqwest::Client::new();
    let query_url = format!("{server_url}/queries/v1/query-request");

    let query_request = ExecRequest {
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
        bindings: None,
        bind_stage: None,
        query_context: RequestQueryContext { entries: None },
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
        .header(
            "User-Agent",
            format!(
                "{}/{} ({}) CPython/3.11.6",
                client_info.application,
                client_info.version.clone(),
                client_info.os_version.clone()
            ),
        )
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

    let response_data: ExecResponse = serde_json::from_str(&response_text).map_err(|e| {
        tracing::trace!("Response text: {}", response_text);
        RestError::InvalidSnowflakeResponse(format!("Failed to parse query response: {e}"))
    })?;

    Ok(response_data)
}
