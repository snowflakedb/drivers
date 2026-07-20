//! Session logout functionality
//!
//! Handles HTTP requests to `/session?delete=true` to terminate server sessions.
//! Returns `RestError` like other Snowflake REST operations (login, query),
//! enabling `RefreshContext` to handle 390112 token refresh automatically.

use crate::config::rest_parameters::ClientInfo;
use crate::config::retry::RetryPolicy;
use crate::http::retry::{HttpContext, execute_with_retry};
use crate::rest::snowflake::error::map_http_error;
use crate::rest::snowflake::{
    AsyncQuerySnafu, LogoutSnafu, RestError, SESSION_GONE, SESSION_TOKEN_EXPIRED,
    SnowflakeResponseError, UrlJoinSnafu, user_agent,
};
use crate::sensitive::SensitiveString;
use reqwest::{Method, header};
use snafu::ResultExt;
use url::Url;

/// Response from the logout endpoint
#[derive(Debug, serde::Deserialize)]
struct LogoutResponse {
    success: bool,
    message: Option<String>,
    code: Option<String>,
}

/// Send a logout request to terminate the Snowflake session.
///
/// This is a pure HTTP function that sends `POST /session?delete=true` to the
/// Snowflake server. Returns `RestError` to enable `RefreshContext` token refresh.
///
/// # Error handling
///
/// - SESSION_GONE (390111) → `Ok(())` (session already terminated, true success)
/// - SESSION_TOKEN_EXPIRED (390112) → `Err(InvalidSnowflakeResponse { SessionExpired })`
///   (signals `RefreshContext` to refresh master token and retry)
/// - Other Snowflake codes → `Err(RestError::Logout { code, message })`
/// - Non-2xx with non-JSON body → `Err(RestError::InvalidSnowflakeResponse { ResponseStatus })`
/// - HTTP transport/retry errors → `Err(RestError::AsyncQuery { SfError })` (mapped from HttpError)
#[tracing::instrument(skip(client, session_token))]
pub async fn logout_session(
    client: &reqwest::Client,
    server_url: &str,
    session_token: &SensitiveString,
    client_info: &ClientInfo,
    retry_policy: &RetryPolicy,
    cancel: tokio_util::sync::CancellationToken,
) -> Result<(), RestError> {
    tracing::info!("Initiating session logout");

    // Construct logout URL
    let logout_url = Url::parse(server_url)
        .and_then(|base| base.join("/session"))
        .context(UrlJoinSnafu { path: "/session" })?;

    // Generate request ID (static across retries for idempotency)
    let request_id = uuid::Uuid::new_v4();

    tracing::debug!(
        %request_id,
        %logout_url,
        "Logout request parameters"
    );

    let user_agent = user_agent(client_info);

    // Logout is POST but idempotent server-side (safe to retry)
    let ctx = HttpContext::new(Method::POST, "/session")
        .with_idempotent(true)
        .allow_post_retry();

    let build_request = || {
        // request_guid is regenerated on each retry, requestId stays the same
        let retry_request_guid = uuid::Uuid::new_v4();

        client
            .post(logout_url.clone())
            .query(&[
                ("delete", "true"),
                ("requestId", &request_id.to_string()),
                ("request_guid", &retry_request_guid.to_string()),
            ])
            .header(
                header::AUTHORIZATION,
                format!("Snowflake Token=\"{}\"", session_token.reveal()),
            )
            .header(header::CONTENT_TYPE, "application/json")
            .header(header::ACCEPT, "application/snowflake")
            .header(header::USER_AGENT, &user_agent)
            .json(&serde_json::json!({}))
        // NO .timeout() - execute_with_retry applies it dynamically
    };

    let response = execute_with_retry(
        &build_request,
        &ctx,
        retry_policy,
        |resp| async move { Ok(resp) },
        cancel.clone(),
    )
    .await
    .map_err(map_http_error)
    .context(AsyncQuerySnafu {
        request_id: Some(request_id),
        query_id: None,
    })?;

    // Read response body as text first (avoids crash on non-JSON responses)
    let status = response.status();
    let body_text = tokio::select! {
        biased;
        _ = cancel.cancelled() => {
            return Err(map_http_error(crate::http::retry::CancelledSnafu.build())).context(
                AsyncQuerySnafu {
                    request_id: Some(request_id),
                    query_id: None,
                },
            );
        }
        text = response.text() => text.ok().unwrap_or_default(),
    };

    // Try to parse as JSON
    let parsed: Option<LogoutResponse> = serde_json::from_str(&body_text).ok();

    match parsed {
        Some(logout_response) => {
            tracing::debug!(
                success = logout_response.success,
                status = %status,
                "Logout response received"
            );
            handle_logout_response(logout_response)
        }
        None => {
            // Non-JSON response (e.g. proxy HTML page)
            if status.is_success() {
                // 2xx but non-JSON is unexpected — treat as success with warning
                tracing::warn!(
                    status = %status,
                    body_len = body_text.len(),
                    "Logout returned 2xx with non-JSON body, treating as success"
                );
                Ok(())
            } else {
                // Non-2xx with non-JSON body (e.g. proxy error page)
                // Log only body length — do not log the body content to avoid
                // leaking proxy HTML, WAF block pages, or internal server details.
                tracing::warn!(
                    status = %status,
                    body_len = body_text.len(),
                    "Logout returned non-2xx with non-JSON body"
                );
                Err(RestError::InvalidSnowflakeResponse {
                    source: SnowflakeResponseError::ResponseStatus {
                        status,
                        message: "Unexpected server response during logout".to_string(),
                        location: snafu::Location::default(),
                    },
                    location: snafu::Location::default(),
                })
            }
        }
    }
}

/// Handle a parsed JSON logout response from Snowflake.
fn handle_logout_response(response: LogoutResponse) -> Result<(), RestError> {
    if response.success {
        tracing::info!("Session logout completed successfully");
        return Ok(());
    }

    let message = response
        .message
        .unwrap_or_else(|| "Unknown error".to_string());
    let code = response
        .code
        .as_deref()
        .and_then(|c| c.parse::<i32>().ok())
        .unwrap_or(-1);

    // SESSION_GONE (390111) means session already terminated — this is success
    if code == SESSION_GONE {
        tracing::info!(
            code = SESSION_GONE,
            "Session already gone (390111) - treating as successful logout"
        );
        return Ok(());
    }

    // SESSION_TOKEN_EXPIRED (390112) — signal RefreshContext to refresh token and retry
    if code == SESSION_TOKEN_EXPIRED {
        tracing::info!(
            code = SESSION_TOKEN_EXPIRED,
            "Session token expired (390112) - signaling token refresh"
        );
        return Err(RestError::InvalidSnowflakeResponse {
            source: SnowflakeResponseError::SessionExpired {
                location: snafu::Location::default(),
            },
            location: snafu::Location::default(),
        });
    }

    // Other Snowflake errors
    tracing::warn!(code, %message, "Logout failed with error");
    LogoutSnafu { message, code }.fail()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_session_gone_error_code() {
        assert_eq!(SESSION_GONE, 390111);
    }

    #[test]
    fn test_session_token_expired_error_code() {
        assert_eq!(SESSION_TOKEN_EXPIRED, 390112);
    }

    #[test]
    fn test_handle_success_response() {
        let response = LogoutResponse {
            success: true,
            message: None,
            code: None,
        };
        assert!(handle_logout_response(response).is_ok());
    }

    #[test]
    fn test_handle_session_gone_390111() {
        let response = LogoutResponse {
            success: false,
            message: Some("Session gone".to_string()),
            code: Some("390111".to_string()),
        };
        assert!(handle_logout_response(response).is_ok());
    }

    #[test]
    fn test_handle_session_token_expired_390112() {
        let response = LogoutResponse {
            success: false,
            message: Some("Token expired".to_string()),
            code: Some("390112".to_string()),
        };
        let err = handle_logout_response(response).unwrap_err();
        // Should be SessionExpired wrapped in InvalidSnowflakeResponse
        assert!(
            matches!(
                err,
                RestError::InvalidSnowflakeResponse {
                    source: SnowflakeResponseError::SessionExpired { .. },
                    ..
                }
            ),
            "Expected SessionExpired, got: {:?}",
            err
        );
    }

    #[test]
    fn test_handle_other_error_code() {
        let response = LogoutResponse {
            success: false,
            message: Some("Bad request".to_string()),
            code: Some("400".to_string()),
        };
        let err = handle_logout_response(response).unwrap_err();
        assert!(
            matches!(err, RestError::Logout { code: 400, .. }),
            "Expected Logout with code 400, got: {:?}",
            err
        );
    }
}
