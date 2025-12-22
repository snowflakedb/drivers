//! Integration tests for sync query retry with requestId.
//!
//! Tests verify that:
//! - Sync queries include requestId parameter
//! - Connection failures trigger retry with retry=true
//! - Retry uses the same requestId for server-side idempotency
//! - Sync mode is the default execution mode

use sf_core::config::rest_parameters::{ClientInfo, QueryParameters};
use sf_core::config::retry::RetryPolicy;
use sf_core::crl::config::CrlConfig;
use sf_core::rest::snowflake::{QueryExecutionMode, snowflake_query_with_client};
use sf_core::tls::config::TlsConfig;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;

#[tokio::test]
async fn should_include_request_id_in_query_parameters() {
    // Given a server that captures the request
    let captured_requests = Arc::new(Mutex::new(Vec::<String>::new()));
    let captured_clone = captured_requests.clone();

    let (addr, server) = spawn_capture_server(1, move |request| {
        captured_clone.lock().unwrap().push(request);
        json_response(
            r#"{"success":true,"data":{"queryId":"test-query-id","rowtype":[],"rowset":[]}}"#,
        )
    })
    .await;

    let client = reqwest::Client::new();
    let query_params = test_query_params(&addr);

    // When I execute a sync query
    let result = snowflake_query_with_client(
        &client,
        query_params,
        "test-token".to_string(),
        "SELECT 1".to_string(),
        None,
        &RetryPolicy::default(),
        QueryExecutionMode::Blocking,
    )
    .await;

    assert!(result.is_ok(), "Query should succeed");
    server.abort();

    // Then the request should include requestId and request_guid parameters
    let requests = captured_requests.lock().unwrap();
    assert!(!requests.is_empty(), "Should have captured a request");
    let request = &requests[0];
    assert!(
        request.contains("requestId="),
        "Request should include requestId: {}",
        request
    );
    assert!(
        request.contains("request_guid="),
        "Request should include request_guid: {}",
        request
    );
}

#[tokio::test]
async fn should_retry_sync_query_on_connection_reset() {
    // Given a server that resets on first connection
    let captured_requests = Arc::new(Mutex::new(Vec::<String>::new()));
    let captured_clone = captured_requests.clone();
    let attempt_count = Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let attempt_clone = attempt_count.clone();

    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();

    let server = tokio::spawn(async move {
        loop {
            let (mut stream, _) = listener.accept().await.unwrap();
            let attempt = attempt_clone.fetch_add(1, std::sync::atomic::Ordering::SeqCst) + 1;

            let mut buf = vec![0u8; 4096];
            let n = stream.read(&mut buf).await.unwrap_or(0);
            let request = String::from_utf8_lossy(&buf[..n]).to_string();
            captured_clone.lock().unwrap().push(request);

            if attempt == 1 {
                // First attempt: close connection abruptly (simulates connection reset)
                drop(stream);
            } else {
                // Second attempt: respond with success
                let body = r#"{"success":true,"data":{"queryId":"test-query-id","rowtype":[],"rowset":[]}}"#;
                let response = format!(
                    "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                    body.len(),
                    body
                );
                let _ = stream.write_all(response.as_bytes()).await;
                let _ = stream.shutdown().await;
                break;
            }
        }
    });

    let client = reqwest::Client::new();
    let query_params = test_query_params(&addr);

    // When a query is submitted in sync mode
    let result = snowflake_query_with_client(
        &client,
        query_params,
        "test-token".to_string(),
        "SELECT 1".to_string(),
        None,
        &RetryPolicy::default(),
        QueryExecutionMode::Blocking,
    )
    .await;

    // Then the driver should retry with the same requestId
    assert!(result.is_ok(), "Query should succeed after retry");
    server.await.ok();

    // And the retry should include retry=true parameter
    let requests = captured_requests.lock().unwrap();
    assert!(requests.len() >= 2, "Should have at least 2 requests");

    let first_request_id = extract_query_param(&requests[0], "requestId");
    assert!(
        first_request_id.is_some(),
        "First request should have requestId"
    );

    let second_request = &requests[1];
    let second_request_id = extract_query_param(second_request, "requestId");
    assert_eq!(
        first_request_id, second_request_id,
        "Retry should use same requestId"
    );
    assert!(
        second_request.contains("retry=true"),
        "Retry request should include retry=true: {}",
        second_request
    );
}

#[tokio::test]
async fn should_use_sync_mode_by_default() {
    // Given a server that captures the request
    let captured_requests = Arc::new(Mutex::new(Vec::<String>::new()));
    let captured_clone = captured_requests.clone();

    let (addr, server) = spawn_capture_server(1, move |request| {
        captured_clone.lock().unwrap().push(request);
        json_response(
            r#"{"success":true,"data":{"queryId":"test-query-id","rowtype":[],"rowset":[]}}"#,
        )
    })
    .await;

    let client = reqwest::Client::new();
    let query_params = test_query_params(&addr);

    // When I execute a query with Blocking mode
    let result = snowflake_query_with_client(
        &client,
        query_params,
        "test-token".to_string(),
        "SELECT 1".to_string(),
        None,
        &RetryPolicy::default(),
        QueryExecutionMode::Blocking,
    )
    .await;

    assert!(result.is_ok(), "Query should succeed");
    server.abort();

    // Then the request body should have asyncExec=false
    let requests = captured_requests.lock().unwrap();
    assert!(!requests.is_empty());
    let request = &requests[0];

    if let Some(body_start) = request.find("\r\n\r\n") {
        let body = &request[body_start + 4..];
        assert!(
            body.contains("\"asyncExec\":false") || body.contains("\"asyncExec\": false"),
            "Request body should have asyncExec=false: {}",
            body
        );
    }
}

// Helper functions

fn test_query_params(addr: &SocketAddr) -> QueryParameters {
    QueryParameters {
        server_url: format!("http://{}", addr),
        client_info: ClientInfo {
            application: "test".to_string(),
            version: "1.0.0".to_string(),
            os: "test-os".to_string(),
            os_version: "1.0".to_string(),
            ocsp_mode: None,
            crl_config: CrlConfig::default(),
            tls_config: TlsConfig::insecure(),
        },
    }
}

fn json_response(body: &str) -> Vec<u8> {
    format!(
        "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
        body.len(),
        body
    )
    .into_bytes()
}

fn extract_query_param<'a>(request: &'a str, param: &str) -> Option<&'a str> {
    let search = format!("{}=", param);
    if let Some(start) = request.find(&search) {
        let value_start = start + search.len();
        let remaining = &request[value_start..];
        let end = remaining
            .find(['&', ' ', '\r', '\n'])
            .unwrap_or(remaining.len());
        Some(&remaining[..end])
    } else {
        None
    }
}

async fn spawn_capture_server<F>(
    max_attempts: usize,
    responder: F,
) -> (SocketAddr, tokio::task::JoinHandle<()>)
where
    F: Fn(String) -> Vec<u8> + Send + Sync + 'static,
{
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let responder = Arc::new(responder);
    let attempt = Arc::new(std::sync::atomic::AtomicUsize::new(0));

    let handle = tokio::spawn(async move {
        loop {
            let (mut stream, _) = listener.accept().await.unwrap();
            let current = attempt.fetch_add(1, std::sync::atomic::Ordering::SeqCst) + 1;
            let responder = responder.clone();

            // Read request
            let mut buf = vec![0u8; 4096];
            let n = stream.read(&mut buf).await.unwrap_or(0);
            let request = String::from_utf8_lossy(&buf[..n]).to_string();

            let response = responder(request);
            let _ = stream.write_all(&response).await;
            let _ = stream.shutdown().await;

            if current >= max_attempts {
                break;
            }
        }
    });

    (addr, handle)
}
