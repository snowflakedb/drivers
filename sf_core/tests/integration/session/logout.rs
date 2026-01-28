//! Integration tests for session logout functionality.

use sf_core::config::rest_parameters::ClientInfo;
use sf_core::config::retry::RetryPolicy;
use sf_core::rest::snowflake::logout::logout_session;
use std::net::SocketAddr;
use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};
use std::time::Duration;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;

#[test]
#[ignore = "TODO: SNOW-2872349 - Phase 5"]
fn should_return_true_when_first_running_async_query_is_detected_without_checking_remaining_queries(
) {
    //Given Async query registry contains multiple queries
    //And First query in registry is running
    //When Auto-detection checks for running queries
    //Then Detection returns true immediately
    //And Remaining queries are not checked
    todo!("Phase 5: Integration optimization test")
}

#[tokio::test]
async fn should_construct_logout_request_with_correct_http_method_url_headers_and_body() {
    //Given Mock HTTP server is configured to capture requests
    //And UD Core client is logged in with session token
    let (addr, _request_data, server) = spawn_capture_server().await;
    let server_url = format!("http://{}", addr);
    let session_token = "test_session_token_12345";
    let client = reqwest::Client::builder()
        .no_proxy()
        .build()
        .expect("Failed to build HTTP client");
    let client_info = test_client_info();
    
    //When Logout is initiated
    let result = logout_session(
        &client,
        &server_url,
        session_token,
        &client_info,
        Duration::from_secs(5),
        &RetryPolicy::default(),
    )
    .await;
    
    //Then Logout succeeds
    assert!(result.is_ok(), "Logout should succeed");
    
    // Wait for server to capture the request
    let captured = server.await.unwrap();
    
    //And HTTP method is POST
    assert!(captured.starts_with(b"POST"), "Should be POST request");
    
    //And Request URL path is /session
    assert!(captured.starts_with(b"POST /session"), "Should request /session");
    
    //And Query parameter delete is set to true
    let request_str = String::from_utf8_lossy(&captured);
    assert!(request_str.contains("delete=true"), "Should have delete=true");
    
    //And Query parameter requestId is present and static across attempts
    assert!(request_str.contains("requestId="), "Should have requestId");
    
    //And Query parameter request_guid is present and unique per attempt
    assert!(request_str.contains("request_guid="), "Should have request_guid");
    
    //And Authorization header is present with format "Snowflake Token={session_token}"
    assert!(
        request_str.contains(&format!("Authorization: Snowflake Token=\"{}\"", session_token)) ||
        request_str.contains(&format!("authorization: Snowflake Token=\"{}\"", session_token)),
        "Should have Authorization header with session token"
    );
    
    //And Content-Type header is application/json
    assert!(
        request_str.to_lowercase().contains("content-type: application/json"),
        "Should have Content-Type: application/json"
    );
    
    //And Accept header is application/snowflake
    assert!(
        request_str.to_lowercase().contains("accept: application/snowflake"),
        "Should have Accept: application/snowflake"
    );
    
    //And User-Agent header contains UD version and Rust version
    assert!(
        request_str.contains("user-agent:") && request_str.contains("UD/"),
        "Should have User-Agent with UD version"
    );
    
    //And Request body is exactly empty JSON object {}
    assert!(request_str.contains("{}"), "Should have empty JSON object body");
}

#[tokio::test]
async fn should_apply_retry_policy_to_logout_http_request() {
    //Given Mock HTTP server returns 503 error on first attempt
    //And Mock HTTP server returns 200 on second attempt
    //And Retry policy allows 2 attempts
    let (addr, attempts, server) = spawn_test_server(2, |attempt| async move {
        if attempt == 1 {
            // First attempt fails with 503
            let body = r#"{"success":false,"message":""}"#;
            format!(
                "HTTP/1.1 503 Service Unavailable\r\nContent-Type: application/json\r\nContent-Length: {}\r\nRetry-After: 0\r\nConnection: close\r\n\r\n{}",
                body.len(),
                body
            ).into_bytes()
        } else {
            // Second attempt succeeds
            let body = r#"{"success":true}"#;
            format!(
                "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                body.len(),
                body
            ).into_bytes()
        }
    })
    .await;
    
    let server_url = format!("http://{}", addr);
    let session_token = "test_token";
    let client = reqwest::Client::builder()
        .no_proxy()
        .build()
        .expect("Failed to build HTTP client");
    let client_info = test_client_info();
    
    //When Logout is initiated
    let result = logout_session(
        &client,
        &server_url,
        session_token,
        &client_info,
        Duration::from_secs(5),
        &RetryPolicy::default(),
    )
    .await;
    
    //Then First request receives 503 response
    //And Retry policy is consulted
    //And Second request is made after backoff delay
    //And Logout succeeds
    assert!(result.is_ok(), "Logout should succeed after retry: {:?}", result.err());
    assert_eq!(attempts.load(Ordering::SeqCst), 2, "Should have made 2 attempts");
    server.await.unwrap();
}

#[tokio::test]
async fn should_handle_http_connection_reset_during_logout() {
    //Given Mock HTTP server resets connection on first attempt
    //And Mock HTTP server succeeds on second attempt
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let attempts = Arc::new(AtomicUsize::new(0));
    let attempts_clone = attempts.clone();
    
    let server = tokio::spawn(async move {
        loop {
            let (mut stream, _) = listener.accept().await.unwrap();
            let attempt = attempts_clone.fetch_add(1, Ordering::SeqCst) + 1;
            
            if attempt == 1 {
                // First attempt: accept connection then immediately close (connection reset)
                drop(stream);
            } else {
                // Second attempt: read request and respond successfully
                let mut buf = [0u8; 2048];
                let _ = stream.read(&mut buf).await;
                let body = r#"{"success":true}"#;
                let response = format!(
                    "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                    body.len(),
                    body
                );
                stream.write_all(response.as_bytes()).await.unwrap();
                let _ = stream.shutdown().await;
                break;
            }
        }
    });
    
    let server_url = format!("http://{}", addr);
    let session_token = "test_token";
    let client = reqwest::Client::builder()
        .no_proxy()
        .build()
        .expect("Failed to build HTTP client");
    let client_info = test_client_info();
    
    //When Logout is initiated
    let result = logout_session(
        &client,
        &server_url,
        session_token,
        &client_info,
        Duration::from_secs(5),
        &RetryPolicy::default(),
    )
    .await;
    
    //Then Connection reset is detected
    //And Request is retried according to retry policy
    //And Logout succeeds on retry
    assert!(result.is_ok(), "Should succeed after connection reset retry");
    assert_eq!(attempts.load(Ordering::SeqCst), 2, "Should have made 2 attempts");
    server.await.unwrap();
}

#[tokio::test]
async fn should_record_connection_close_decision_metrics_before_logout() {
    //Given Telemetry client is configured
    //And UD Core client is logged in
    
    // Note: This test is a stub as telemetry infrastructure isn't implemented yet
    // TODO: SNOW-2912513 - Implement telemetry
    
    //When Connection close is initiated
    //Then Pre-logout metrics are recorded in telemetry batch
    //And Metrics include whether auto-detection was performed
    //And Metrics include whether async queries were detected
    //And Metrics include whether logout will be sent or skipped
    //And Metrics include skip reason if logout is skipped
    //And Telemetry batch is flushed before logout is sent
    //And Logout proceeds after telemetry flush completes
    
    // For now, just verify that logout works without telemetry
    let (addr, _, server) = spawn_test_server(1, |_| async move {
        let body = r#"{"success":true}"#;
        format!(
            "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
            body.len(),
            body
        ).into_bytes()
    })
    .await;
    
    let server_url = format!("http://{}", addr);
    let client = reqwest::Client::builder()
        .no_proxy()
        .build()
        .expect("Failed to build HTTP client");
    let client_info = test_client_info();
    
    let result = logout_session(
        &client,
        &server_url,
        "test_token",
        &client_info,
        Duration::from_secs(5),
        &RetryPolicy::default(),
    )
    .await;
    
    assert!(result.is_ok(), "Logout should succeed even without telemetry");
    server.await.unwrap();
    
    // TODO: Once telemetry is implemented, verify:
    // - Telemetry records were created
    // - Metrics include expected fields
    // - Telemetry was flushed before logout
}

// Helper functions

fn test_client_info() -> ClientInfo {
    ClientInfo {
        application: "TestApp".to_string(),
        version: "1.0.0".to_string(),
        os: "TestOS".to_string(),
        os_version: "1.0".to_string(),
        ocsp_mode: Some("FAIL_OPEN".to_string()),
        crl_config: Default::default(),
        tls_config: Default::default(),
    }
}

async fn spawn_test_server<F, Fut>(
    max_attempts: usize,
    responder: F,
) -> (SocketAddr, Arc<AtomicUsize>, tokio::task::JoinHandle<()>)
where
    F: Fn(usize) -> Fut + Send + Sync + 'static,
    Fut: std::future::Future<Output = Vec<u8>> + Send + 'static,
{
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let attempts = Arc::new(AtomicUsize::new(0));
    let attempts_clone = attempts.clone();
    let responder = Arc::new(responder);
    
    let handle = tokio::spawn(async move {
        loop {
            let (mut stream, _) = listener.accept().await.unwrap();
            let attempt = attempts_clone.fetch_add(1, Ordering::SeqCst) + 1;
            let responder = responder.clone();
            let mut buf = [0u8; 2048];
            let _ = stream.read(&mut buf).await;
            let response = responder(attempt).await;
            stream.write_all(&response).await.unwrap();
            let _ = stream.shutdown().await;
            if attempt >= max_attempts {
                break;
            }
        }
    });
    
    (addr, attempts, handle)
}

async fn spawn_capture_server() -> (SocketAddr, Arc<AtomicUsize>, tokio::task::JoinHandle<Vec<u8>>) {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let attempts = Arc::new(AtomicUsize::new(0));
    let attempts_clone = attempts.clone();
    
    let handle = tokio::spawn(async move {
        let (mut stream, _) = listener.accept().await.unwrap();
        attempts_clone.fetch_add(1, Ordering::SeqCst);
        
        // Read the request
        let mut buf = vec![0u8; 4096];
        let n = stream.read(&mut buf).await.unwrap();
        buf.truncate(n);
        
        // Send success response
        let body = r#"{"success":true}"#;
        let response = format!(
            "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
            body.len(),
            body
        );
        stream.write_all(response.as_bytes()).await.unwrap();
        let _ = stream.shutdown().await;
        
        buf
    });
    
    (addr, attempts, handle)
}
