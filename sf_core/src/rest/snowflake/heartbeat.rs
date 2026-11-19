use std::time::Duration;

use snafu::ResultExt;
use url::Url;

use crate::config::rest_parameters::ClientInfo;
use crate::http::dpop::{self, DPoPBinding, DPoPSessionKey};
use crate::rest::snowflake::{
    HeartbeatSnafu, RestError, UrlJoinSnafu, apply_query_headers, parse_external_session_header,
    parse_gs_code_or_unavailable, read_response_json,
};

const HEARTBEAT_PATH: &str = "/session/heartbeat";
const DEFAULT_HEARTBEAT_TIMEOUT: Duration = Duration::from_secs(300);

type HeartbeatResponse = crate::rest::snowflake::SnowflakeResponse<serde_json::Value>;

/// Send a heartbeat POST to keep the session alive using the default timeout.
///
/// Returns `Ok(())` on success, `RestError::SessionExpired` on HTTP 401, or
/// `RestError::Heartbeat` when the server explicitly reports failure.
pub async fn send_heartbeat(
    client: &reqwest::Client,
    server_url: &Url,
    client_info: &ClientInfo,
    session_token: &str,
    dpop_key: Option<&DPoPSessionKey>,
) -> Result<(), RestError> {
    send_heartbeat_with_timeout(
        client,
        server_url,
        client_info,
        session_token,
        None,
        None,
        dpop_key,
    )
    .await
}

/// Like [`send_heartbeat`] but with an optional timeout override.
///
/// When `timeout` is `None`, the default 300 s HTTP request timeout is used.
#[tracing::instrument(skip(client, client_info, session_token, external_session_id, dpop_key))]
pub async fn send_heartbeat_with_timeout(
    client: &reqwest::Client,
    server_url: &Url,
    client_info: &ClientInfo,
    session_token: &str,
    timeout: Option<Duration>,
    external_session_id: Option<&str>,
    dpop_key: Option<&DPoPSessionKey>,
) -> Result<(), RestError> {
    let url = server_url.join(HEARTBEAT_PATH).context(UrlJoinSnafu {
        path: HEARTBEAT_PATH,
    })?;

    let external_session_header = parse_external_session_header(external_session_id)?;
    let request = apply_query_headers(
        client.post(url),
        client_info,
        session_token,
        external_session_header.as_ref(),
        None,
    )
    .timeout(timeout.unwrap_or(DEFAULT_HEARTBEAT_TIMEOUT))
    .build()
    .context(crate::rest::snowflake::RequestConstructionSnafu {
        request: "heartbeat",
    })?;

    let binding = dpop_key.map(|key| DPoPBinding::new(key, session_token));
    let response = dpop::execute(client, request, binding).await.context(
        crate::rest::snowflake::CommunicationSnafu {
            context: "Failed to execute heartbeat request",
        },
    )?;

    let parsed: HeartbeatResponse = read_response_json::<serde_json::Value>(response).await?;

    if !parsed.success {
        let message = parsed
            .message
            .unwrap_or_else(|| "Unknown error".to_string());
        let code = parse_gs_code_or_unavailable(parsed.code.as_deref());
        return HeartbeatSnafu { message, code }.fail();
    }

    tracing::debug!("Heartbeat sent successfully");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::rest_parameters::test_fixtures::test_client_info;
    use serde_json::json;
    use wiremock::matchers::{header, header_regex, method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    #[tokio::test]
    async fn heartbeat_request_url_and_headers() {
        let server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/session/heartbeat"))
            .and(header_regex("Authorization", r#"^Snowflake Token=".+"$"#))
            .and(header("Accept", "application/json"))
            .and(header_regex("User-Agent", r#"^.+/\S+ \(\S+\)"#))
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
            None,
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
            None,
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
            None,
        )
        .await;

        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(
            matches!(&err, RestError::SessionExpired { .. }),
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
                "message": "Some heartbeat failure",
                "code": "390100"
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
            None,
        )
        .await;

        assert!(result.is_err());
        assert!(
            matches!(result.unwrap_err(), RestError::Heartbeat { .. }),
            "Expected Heartbeat variant"
        );
    }

    #[tokio::test]
    async fn heartbeat_on_a_bound_session_carries_a_proof_for_the_session_token() {
        use crate::crypto::dpop::test_fixtures::{decode_jwt, token_hash};

        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/session/heartbeat"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({ "success": true })))
            .mount(&server)
            .await;
        let key = dpop::test_fixtures::session_key();

        send_heartbeat(
            &reqwest::Client::new(),
            &Url::parse(&server.uri()).unwrap(),
            &test_client_info(),
            "session-token",
            Some(&key),
        )
        .await
        .expect("heartbeat");

        let requests = server.received_requests().await.unwrap();
        let (_, claims) = decode_jwt(requests[0].headers["DPoP"].to_str().unwrap());
        assert_eq!(claims["htu"], format!("{}/session/heartbeat", server.uri()));
        assert_eq!(claims["ath"], token_hash("session-token"));
    }
}
