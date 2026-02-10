//! Session logout functionality
//!
//! Handles HTTP requests to `/session?delete=true` to terminate server sessions.

use crate::config::rest_parameters::ClientInfo;
use crate::config::retry::RetryPolicy;
use crate::http::retry::{HttpContext, HttpError, execute_with_retry};
use reqwest::{Method, header};
use snafu::{Location, ResultExt, Snafu};
use std::time::Duration;
use url::Url;

/// Error codes from Snowflake GS
const SESSION_GONE: i32 = 390111;

#[derive(Debug, Snafu)]
pub enum LogoutError {
    #[snafu(display("Failed to build logout request URL"))]
    UrlConstruction {
        source: url::ParseError,
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("HTTP error during logout"))]
    Http {
        source: HttpError,
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("Failed to parse logout response"))]
    ResponseParse {
        source: reqwest::Error,
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("Logout failed with error: {message} (code: {code})"))]
    LogoutFailed {
        message: String,
        code: i32,
        #[snafu(implicit)]
        location: Location,
    },
}

/// Response from the logout endpoint
#[derive(Debug, serde::Deserialize)]
struct LogoutResponse {
    success: bool,
    message: Option<String>,
    code: Option<String>,
}

/// Send a logout request to terminate the Snowflake session.
///
/// This is a pure HTTP function that takes individual parameters and sends
/// a `POST /session?delete=true` request to the Snowflake server.
///
/// # Arguments
///
/// * `client` - HTTP client to use for the request
/// * `server_url` - Base URL of the Snowflake server
/// * `session_token` - Current session token for authentication
/// * `client_info` - Client information for User-Agent header
/// * `timeout` - Timeout duration for the request
/// * `retry_policy` - Retry policy to use for transient failures
///
/// # Returns
///
/// * `Ok(())` - Session logged out successfully or already gone (SESSION_GONE 390111)
/// * `Err(LogoutError)` - Logout failed with unrecoverable error
///
/// # Errors
///
/// * SESSION_GONE (390111) - Silently ignored (session already terminated)
/// * Other errors - Returned to caller for handling per error strategy
#[tracing::instrument(skip(client, session_token))]
pub async fn logout_session(
    client: &reqwest::Client,
    server_url: &str,
    session_token: &str,
    client_info: &ClientInfo,
    timeout: Duration,
    retry_policy: &RetryPolicy,
) -> Result<(), LogoutError> {
    tracing::info!("Initiating session logout");

    // Construct logout URL
    let logout_url = Url::parse(server_url)
        .and_then(|base| base.join("/session"))
        .context(UrlConstructionSnafu)?;

    // TODO: should be helper func
    // Generate UUIDs for request tracking
    let request_id = uuid::Uuid::new_v4();
    let request_guid = uuid::Uuid::new_v4();

    tracing::debug!(
        %request_id,
        %request_guid,
        %logout_url,
        timeout_secs = timeout.as_secs(),
        "Logout request parameters"
    );

    // TODO: should be static helper cached
    // Build User-Agent per UD spec: {WrapperUA} UD/{core_ver} Rust/{rust_ver}
    let rust_version = option_env!("CARGO_PKG_RUST_VERSION").unwrap_or("unknown");
    let user_agent = format!(
        "{}/{} ({}) UD/1.0.0 Rust/{}",
        &client_info.application, &client_info.version, &client_info.os, rust_version
    );

    // Build authorization header
    let auth_header = format!("Snowflake Token=\"{}\"", session_token);

    // Create HTTP context for retry logic
    // Logout is POST but idempotent server-side (safe to retry)
    let ctx = HttpContext::new(Method::POST, "/session")
        .with_idempotent(true)
        .allow_post_retry();

    // Execute with retry
    let build_request = || {
        // Note: request_guid is regenerated on each retry, requestId stays the same
        let retry_request_guid = uuid::Uuid::new_v4();

        client
            .post(logout_url.clone())
            .query(&[
                ("delete", "true"),
                ("requestId", &request_id.to_string()),
                ("request_guid", &retry_request_guid.to_string()),
            ])
            .header(header::AUTHORIZATION, &auth_header)
            .header(header::CONTENT_TYPE, "application/json")
            .header(header::ACCEPT, "application/snowflake")
            .header(header::USER_AGENT, &user_agent)
            .json(&serde_json::json!({})) // Empty JSON object body
            .timeout(timeout)
    };

    let response = execute_with_retry(&build_request, &ctx, retry_policy, |resp| async move {
        Ok(resp)
    })
    .await
    .context(HttpSnafu)?;

    // Parse response
    let status = response.status();
    let logout_response: LogoutResponse = response.json().await.context(ResponseParseSnafu)?;

    tracing::debug!(
        success = logout_response.success,
        status = %status,
        "Logout response received"
    );

    // Handle response
    if !logout_response.success {
        let message = logout_response
            .message
            .unwrap_or_else(|| "Unknown error".to_string());
        let code = logout_response
            .code
            .as_deref()
            .and_then(|c| c.parse::<i32>().ok())
            .unwrap_or(-1);

        // SESSION_GONE (390111) means session already terminated - this is success
        if code == SESSION_GONE {
            tracing::info!(
                code = SESSION_GONE,
                "Session already gone (390111) - treating as successful logout"
            );
            return Ok(());
        }

        // Other errors are returned to caller
        tracing::warn!(
            code,
            %message,
            "Logout failed with error"
        );
        return LogoutFailedSnafu { message, code }.fail();
    }

    tracing::info!("Session logout completed successfully");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_session_gone_error_code() {
        assert_eq!(SESSION_GONE, 390111);
    }
}
