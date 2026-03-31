use std::time::Duration;

use snafu::ResultExt;
use url::Url;

use crate::config::rest_parameters::ClientInfo;
use crate::rest::snowflake::{
    HeartbeatSnafu, InvalidSnowflakeResponseSnafu, RestError, UrlJoinSnafu, apply_query_headers,
    read_response_json,
};

const HEARTBEAT_PATH: &str = "/session/heartbeat";
const HEARTBEAT_TIMEOUT: Duration = Duration::from_secs(300);

#[derive(Debug, serde::Deserialize)]
struct HeartbeatResponse {
    success: bool,
    message: Option<String>,
    code: Option<String>,
}

/// Send a heartbeat POST to keep the session alive.
///
/// Returns `Ok(())` on success, `RestError::InvalidSnowflakeResponse` with
/// `SnowflakeResponseError::SessionExpired` on HTTP 401, or
/// `RestError::Heartbeat` when the server explicitly reports failure.
#[tracing::instrument(skip(client, client_info, session_token))]
pub async fn send_heartbeat(
    client: &reqwest::Client,
    server_url: &Url,
    client_info: &ClientInfo,
    session_token: &str,
) -> Result<(), RestError> {
    let url = server_url.join(HEARTBEAT_PATH).context(UrlJoinSnafu {
        path: HEARTBEAT_PATH,
    })?;

    let request = apply_query_headers(client.post(url), client_info, session_token)
        .timeout(HEARTBEAT_TIMEOUT)
        .build()
        .context(crate::rest::snowflake::RequestConstructionSnafu {
            request: "heartbeat",
        })?;

    let response =
        client
            .execute(request)
            .await
            .context(crate::rest::snowflake::CommunicationSnafu {
                context: "Failed to execute heartbeat request",
            })?;

    let parsed: HeartbeatResponse = read_response_json(response)
        .await
        .context(InvalidSnowflakeResponseSnafu)?;

    if !parsed.success {
        let message = parsed
            .message
            .unwrap_or_else(|| "Unknown error".to_string());
        let code = parsed
            .code
            .as_deref()
            .and_then(|c| c.parse::<i32>().ok())
            .unwrap_or(-1);
        return HeartbeatSnafu { message, code }.fail();
    }

    tracing::debug!("Heartbeat sent successfully");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use wiremock::matchers::{header, header_regex, method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    fn test_client_info() -> ClientInfo {
        use crate::crl::config::CrlConfig;
        use crate::tls::config::TlsConfig;
        ClientInfo {
            application: "TestApp".to_string(),
            version: "1.0.0".to_string(),
            os: "Linux".to_string(),
            os_version: "5.15".to_string(),
            ocsp_mode: None,
            crl_config: CrlConfig::default(),
            tls_config: TlsConfig::default(),
        }
    }

    #[tokio::test]
    async fn heartbeat_request_url_and_headers() {
        let server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/session/heartbeat"))
            .and(header_regex("Authorization", r#"^Snowflake Token=".+"$"#))
            .and(header("Accept", "application/json"))
            .and(header_regex(
                "User-Agent",
                r#"^.+/\S+ \(\S+\) CPython/3\.11\.6$"#,
            ))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "success": true
            })))
            .expect(1)
            .mount(&server)
            .await;

        let client = reqwest::Client::new();
        let server_url = Url::parse(&server.uri()).unwrap();
        let result = send_heartbeat(
            &client,
            &server_url,
            &test_client_info(),
            "test_session_token",
        )
        .await;

        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn heartbeat_success_response() {
        let server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/session/heartbeat"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "success": true,
                "message": null,
                "code": null
            })))
            .mount(&server)
            .await;

        let client = reqwest::Client::new();
        let server_url = Url::parse(&server.uri()).unwrap();
        let result = send_heartbeat(
            &client,
            &server_url,
            &test_client_info(),
            "test_session_token",
        )
        .await;

        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn heartbeat_session_expired_response() {
        let server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/session/heartbeat"))
            .respond_with(ResponseTemplate::new(401))
            .mount(&server)
            .await;

        let client = reqwest::Client::new();
        let server_url = Url::parse(&server.uri()).unwrap();
        let result = send_heartbeat(
            &client,
            &server_url,
            &test_client_info(),
            "test_session_token",
        )
        .await;

        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(
            matches!(
                &err,
                RestError::InvalidSnowflakeResponse {
                    source: crate::rest::snowflake::SnowflakeResponseError::SessionExpired { .. },
                    ..
                }
            ),
            "Expected SessionExpired, got: {err:?}"
        );
    }

    #[tokio::test]
    async fn heartbeat_failure_response() {
        let server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/session/heartbeat"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "success": false,
                "message": "Session gone",
                "code": "390112"
            })))
            .mount(&server)
            .await;

        let client = reqwest::Client::new();
        let server_url = Url::parse(&server.uri()).unwrap();
        let result = send_heartbeat(
            &client,
            &server_url,
            &test_client_info(),
            "test_session_token",
        )
        .await;

        assert!(result.is_err());
        assert!(
            matches!(result.unwrap_err(), RestError::Heartbeat { .. }),
            "Expected Heartbeat variant"
        );
    }
}
