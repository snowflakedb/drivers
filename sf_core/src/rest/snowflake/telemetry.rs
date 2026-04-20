use std::io::Write;
use std::time::Duration;

use flate2::Compression;
use flate2::write::GzEncoder;
use reqwest::header;
use snafu::ResultExt;
use url::Url;

use crate::config::rest_parameters::QueryParameters;
use crate::rest::snowflake::{
    CommunicationSnafu, InvalidSnowflakeResponseSnafu, PayloadEncodingSnafu,
    RequestConstructionSnafu, RestError, UrlJoinSnafu, apply_json_content_type,
    apply_query_headers, read_response_json,
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
#[tracing::instrument(skip(client, query_parameters, session_token, payload))]
pub async fn send_telemetry(
    client: &reqwest::Client,
    query_parameters: &QueryParameters,
    session_token: &str,
    payload: &serde_json::Value,
) -> Result<(), RestError> {
    let server_url = Url::parse(&query_parameters.server_url).context(UrlJoinSnafu {
        path: TELEMETRY_SEND_PATH,
    })?;
    let url = server_url.join(TELEMETRY_SEND_PATH).context(UrlJoinSnafu {
        path: TELEMETRY_SEND_PATH,
    })?;

    let json_bytes = serde_json::to_vec(payload).map_err(|e| {
        PayloadEncodingSnafu {
            reason: format!("JSON serialization failed: {e}"),
        }
        .build()
    })?;
    let mut encoder = GzEncoder::new(Vec::new(), Compression::default());
    encoder.write_all(&json_bytes).map_err(|e| {
        PayloadEncodingSnafu {
            reason: format!("gzip compression failed: {e}"),
        }
        .build()
    })?;
    let compressed = encoder.finish().map_err(|e| {
        PayloadEncodingSnafu {
            reason: format!("gzip finalization failed: {e}"),
        }
        .build()
    })?;

    let request = apply_json_content_type(apply_query_headers(
        client.post(url),
        &query_parameters.client_info,
        session_token,
    ))
    .header(header::CONTENT_ENCODING, "gzip")
    .header(header::ACCEPT_ENCODING, "gzip, deflate")
    .header(header::CONNECTION, "keep-alive")
    .body(compressed)
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
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    fn test_query_parameters(server_url: &str) -> QueryParameters {
        use crate::config::rest_parameters::ClientInfo;
        use crate::crl::config::CrlConfig;
        use crate::tls::config::TlsConfig;
        QueryParameters {
            server_url: server_url.to_string(),
            client_info: ClientInfo {
                application: "TestApp".to_string(),
                version: "1.0.0".to_string(),
                os: "Linux".to_string(),
                os_version: "5.15".to_string(),
                ocsp_mode: None,
                crl_config: CrlConfig::default(),
                tls_config: TlsConfig::default(),
            },
            log_max_query_length: 80,
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
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "success": true
            })))
            .expect(1)
            .mount(&server)
            .await;

        let client = reqwest::Client::new();
        let query_params = test_query_parameters(&server.uri());
        let result = send_telemetry(&client, &query_params, "test_session_token", &payload).await;

        assert!(result.is_ok(), "send_telemetry failed: {:?}", result.err());

        // Verify headers and gzip body
        let requests = server.received_requests().await.unwrap();
        assert_eq!(requests.len(), 1);
        let req = &requests[0];

        // Check all expected headers
        let has_header = |name: &str, value: &str| {
            req.headers
                .get(name)
                .map(|v| v.to_str().unwrap_or("") == value)
                .unwrap_or(false)
        };
        assert!(has_header("content-encoding", "gzip"));
        assert!(has_header("accept-encoding", "gzip, deflate"));
        assert!(has_header("connection", "keep-alive"));
        assert!(has_header("content-type", "application/json"));
        assert!(has_header("accept", "application/json"));
        assert!(req.headers.get("authorization").is_some());

        // Verify the gzip body decompresses to the original payload
        let body = &req.body;
        let mut decoder = flate2::read::GzDecoder::new(&body[..]);
        let mut decompressed = String::new();
        std::io::Read::read_to_string(&mut decoder, &mut decompressed).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&decompressed).unwrap();
        assert_eq!(parsed, payload);
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
        let query_params = test_query_parameters(&server.uri());
        let result = send_telemetry(
            &client,
            &query_params,
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
        let query_params = test_query_parameters(&server.uri());
        let result = send_telemetry(
            &client,
            &query_params,
            "test_session_token",
            &json!({"logs": []}),
        )
        .await;

        assert!(result.is_err());
    }
}
