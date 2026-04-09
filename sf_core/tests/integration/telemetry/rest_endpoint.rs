use std::io::Read;

use serde_json::json;
use sf_core::config::rest_parameters::{ClientInfo, QueryParameters};
use sf_core::crl::config::CrlConfig;
use sf_core::rest::snowflake::telemetry::send_telemetry;
use sf_core::tls::config::TlsConfig;
use wiremock::matchers::{header, header_regex, method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

fn test_query_parameters(server_url: &str) -> QueryParameters {
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
    }
}

/// Decompress a gzip-encoded body and parse as JSON.
fn decompress_gzip_json(body: &[u8]) -> serde_json::Value {
    let mut decoder = flate2::read::GzDecoder::new(body);
    let mut decompressed = String::new();
    decoder
        .read_to_string(&mut decompressed)
        .expect("Failed to decompress gzip body");
    serde_json::from_str(&decompressed).expect("Failed to parse decompressed JSON")
}

#[tokio::test]
async fn telemetry_post_includes_auth_and_content_type_headers() {
    let server = MockServer::start().await;

    let payload = json!({
        "logs": [{
            "message": {"type": "session_init", "service.name": "snowflake-python"},
            "timestamp": "1700000000000"
        }]
    });

    Mock::given(method("POST"))
        .and(path("/telemetry/send"))
        .and(header_regex("Authorization", r#"^Snowflake Token=".+"$"#))
        .and(header("Content-Type", "application/json"))
        .and(header("Accept", "application/json"))
        .and(header("Content-Encoding", "gzip"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({"success": true})))
        .expect(1)
        .mount(&server)
        .await;

    let client = reqwest::Client::new();
    let query_params = test_query_parameters(&server.uri());
    let result = send_telemetry(&client, &query_params, "my_session_token", &payload).await;

    assert!(result.is_ok());

    // Verify the gzip body decompresses to the original payload
    let requests = server.received_requests().await.unwrap();
    assert_eq!(requests.len(), 1);
    let parsed = decompress_gzip_json(&requests[0].body);
    assert_eq!(parsed, payload);
}

#[tokio::test]
async fn telemetry_post_sends_multiple_log_entries() {
    let server = MockServer::start().await;

    let payload = json!({
        "logs": [
            {
                "message": {"type": "session_init"},
                "timestamp": "1700000000000"
            },
            {
                "message": {"type": "driver_exception", "exception.type": "ValueError"},
                "timestamp": "1700000001000"
            }
        ]
    });

    Mock::given(method("POST"))
        .and(path("/telemetry/send"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({"success": true})))
        .expect(1)
        .mount(&server)
        .await;

    let client = reqwest::Client::new();
    let query_params = test_query_parameters(&server.uri());
    let result = send_telemetry(&client, &query_params, "token", &payload).await;

    assert!(result.is_ok());

    // Verify the gzip body decompresses to the original payload
    let requests = server.received_requests().await.unwrap();
    assert_eq!(requests.len(), 1);
    let parsed = decompress_gzip_json(&requests[0].body);
    assert_eq!(parsed, payload);
}

#[tokio::test]
async fn telemetry_post_handles_401_as_error() {
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
        "expired_token",
        &json!({"logs": []}),
    )
    .await;

    assert!(result.is_err());
}

#[tokio::test]
async fn telemetry_post_handles_connection_refused() {
    let client = reqwest::Client::new();
    // Port 1 is almost certainly not listening
    let query_params = test_query_parameters("http://127.0.0.1:1");
    let result = send_telemetry(&client, &query_params, "token", &json!({"logs": []})).await;

    assert!(result.is_err());
}
