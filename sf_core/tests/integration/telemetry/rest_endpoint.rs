use serde_json::json;
use sf_core::config::rest_parameters::ClientInfo;
use sf_core::crl::config::CrlConfig;
use sf_core::rest::snowflake::telemetry::send_telemetry;
use sf_core::tls::config::TlsConfig;
use url::Url;
use wiremock::matchers::{body_json, header, header_regex, method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

fn test_client_info() -> ClientInfo {
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
        .and(body_json(&payload))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({"success": true})))
        .expect(1)
        .mount(&server)
        .await;

    let client = reqwest::Client::new();
    let server_url = Url::parse(&server.uri()).unwrap();
    let result = send_telemetry(
        &client,
        &server_url,
        &test_client_info(),
        "my_session_token",
        &payload,
    )
    .await;

    assert!(result.is_ok());
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
        .and(body_json(&payload))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({"success": true})))
        .expect(1)
        .mount(&server)
        .await;

    let client = reqwest::Client::new();
    let server_url = Url::parse(&server.uri()).unwrap();
    let result = send_telemetry(&client, &server_url, &test_client_info(), "token", &payload).await;

    assert!(result.is_ok());
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
    let server_url = Url::parse(&server.uri()).unwrap();
    let result = send_telemetry(
        &client,
        &server_url,
        &test_client_info(),
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
    let server_url = Url::parse("http://127.0.0.1:1").unwrap();
    let result = send_telemetry(
        &client,
        &server_url,
        &test_client_info(),
        "token",
        &json!({"logs": []}),
    )
    .await;

    assert!(result.is_err());
}
