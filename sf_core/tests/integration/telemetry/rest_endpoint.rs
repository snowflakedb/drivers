use serde_json::json;
use sf_core::rest::snowflake::telemetry::send_telemetry;
use wiremock::matchers::{header, header_regex, method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

use super::common::{decompress_gzip_json, test_query_parameters};

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
