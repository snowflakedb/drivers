use std::time::Duration;

use snafu::ResultExt;
use url::Url;

use crate::config::rest_parameters::ClientInfo;
use crate::rest::snowflake::{
    CommunicationSnafu, InvalidSnowflakeResponseSnafu, RequestConstructionSnafu, RestError,
    UrlJoinSnafu, apply_json_content_type, apply_query_headers, read_response_json,
};

const TELEMETRY_SEND_PATH: &str = "/telemetry/send";
const TELEMETRY_TIMEOUT: Duration = Duration::from_secs(30);

#[derive(Debug, serde::Deserialize)]
struct TelemetryResponse {
    success: bool,
    message: Option<String>,
    code: Option<String>,
}

/// POST a batch of telemetry events to the Snowflake in-band endpoint.
///
/// The `payload` must conform to Snowflake's expected format:
/// `{"logs": [{"message": {...}, "timestamp": "..."}]}`
///
/// Telemetry is best-effort: callers should handle errors gracefully.
#[tracing::instrument(skip(client, client_info, session_token, payload))]
pub async fn send_telemetry(
    client: &reqwest::Client,
    server_url: &Url,
    client_info: &ClientInfo,
    session_token: &str,
    payload: &serde_json::Value,
) -> Result<(), RestError> {
    let url = server_url.join(TELEMETRY_SEND_PATH).context(UrlJoinSnafu {
        path: TELEMETRY_SEND_PATH,
    })?;

    let request = apply_json_content_type(apply_query_headers(
        client.post(url),
        client_info,
        session_token,
    ))
    .json(payload)
    .timeout(TELEMETRY_TIMEOUT)
    .build()
    .context(RequestConstructionSnafu {
        request: "telemetry",
    })?;

    let response = client.execute(request).await.context(CommunicationSnafu {
        context: "Failed to execute telemetry request",
    })?;

    let parsed: TelemetryResponse = read_response_json(response)
        .await
        .context(InvalidSnowflakeResponseSnafu)?;

    if !parsed.success {
        tracing::warn!(
            code = parsed.code.as_deref().unwrap_or("none"),
            message = parsed.message.as_deref().unwrap_or("none"),
            "Telemetry endpoint returned success=false",
        );
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use wiremock::matchers::{body_json, header_regex, method, path};
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
    async fn send_telemetry_success() {
        let server = MockServer::start().await;

        let payload = json!({
            "logs": [{
                "message": {"type": "test"},
                "timestamp": "1234567890123"
            }]
        });

        Mock::given(method("POST"))
            .and(path("/telemetry/send"))
            .and(header_regex("Authorization", r#"^Snowflake Token=".+"$"#))
            .and(body_json(&payload))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "success": true
            })))
            .expect(1)
            .mount(&server)
            .await;

        let client = reqwest::Client::new();
        let server_url = Url::parse(&server.uri()).unwrap();
        let result = send_telemetry(
            &client,
            &server_url,
            &test_client_info(),
            "test_session_token",
            &payload,
        )
        .await;

        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn send_telemetry_server_error() {
        let server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/telemetry/send"))
            .respond_with(ResponseTemplate::new(500).set_body_string("Internal Server Error"))
            .mount(&server)
            .await;

        let client = reqwest::Client::new();
        let server_url = Url::parse(&server.uri()).unwrap();
        let result = send_telemetry(
            &client,
            &server_url,
            &test_client_info(),
            "test_session_token",
            &json!({"logs": []}),
        )
        .await;

        assert!(result.is_err());
    }

    #[tokio::test]
    async fn send_telemetry_session_expired() {
        let server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/telemetry/send"))
            .respond_with(ResponseTemplate::new(401))
            .mount(&server)
            .await;

        let client = reqwest::Client::new();
        let server_url = Url::parse(&server.uri()).unwrap();
        let result = send_telemetry(
            &client,
            &server_url,
            &test_client_info(),
            "test_session_token",
            &json!({"logs": []}),
        )
        .await;

        assert!(result.is_err());
    }
}
