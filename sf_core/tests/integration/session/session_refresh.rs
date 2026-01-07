//! Integration tests for session refresh with concurrent requests.

use sf_core::apis::database_driver_v1::{Connection, with_valid_session};
use sf_core::config::rest_parameters::ClientInfo;
use sf_core::config::retry::RetryPolicy;
use sf_core::crl::config::CrlConfig;
use sf_core::rest::snowflake::SessionTokens;
use sf_core::tls::config::TlsConfig;
use std::collections::HashMap;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;
use tokio::sync::RwLock as AsyncRwLock;

fn test_client_info() -> ClientInfo {
    ClientInfo {
        application: "test".to_string(),
        version: "1.0".to_string(),
        os: "test-os".to_string(),
        os_version: "1.0".to_string(),
        ocsp_mode: None,
        crl_config: CrlConfig::default(),
        tls_config: TlsConfig::insecure(),
    }
}

#[tokio::test]
async fn should_only_refresh_once_with_concurrent_401_errors() {
    // Given a connection with an expired session token
    let query_attempts = Arc::new(AtomicUsize::new(0));
    let refresh_attempts = Arc::new(AtomicUsize::new(0));
    let query_attempts_clone = query_attempts.clone();
    let refresh_attempts_clone = refresh_attempts.clone();

    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();

    // Server handles both query and refresh requests
    let server = tokio::spawn(async move {
        // We expect: 3 initial queries (all get 401), 1 refresh, 3 retry queries (all succeed)
        for _ in 0..7 {
            let (mut stream, _) = listener.accept().await.unwrap();
            let query_attempts = query_attempts_clone.clone();
            let refresh_attempts = refresh_attempts_clone.clone();

            tokio::spawn(async move {
                let mut buf = [0u8; 4096];
                let n = stream.read(&mut buf).await.unwrap();
                let request = String::from_utf8_lossy(&buf[..n]);

                let response = if request.contains("/session/token-request") {
                    // Refresh request
                    refresh_attempts.fetch_add(1, Ordering::SeqCst);
                    let body = r#"{"success":true,"data":{"sessionToken":"new-session-token","masterToken":"new-master-token","sessionId":67890}}"#;
                    format!(
                        "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                        body.len(),
                        body
                    )
                } else if request.contains("/queries") {
                    // Query request
                    let attempt = query_attempts.fetch_add(1, Ordering::SeqCst) + 1;
                    if attempt <= 3 {
                        // First 3 queries get 401 (simulating concurrent requests hitting expired session)
                        "HTTP/1.1 401 Unauthorized\r\nContent-Length: 15\r\nConnection: close\r\n\r\nSession expired".to_string()
                    } else {
                        // After refresh, queries succeed
                        let body = r#"{"success":true,"data":{"queryId":"test-query"}}"#;
                        format!(
                            "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                            body.len(),
                            body
                        )
                    }
                } else {
                    "HTTP/1.1 404 Not Found\r\nContent-Length: 0\r\nConnection: close\r\n\r\n"
                        .to_string()
                };

                stream.write_all(response.as_bytes()).await.unwrap();
                let _ = stream.shutdown().await;
            });
        }
    });

    // Create a connection with initial tokens
    let tokens = SessionTokens {
        session_token: "old-session-token".to_string(),
        master_token: "valid-master-token".to_string(),
        session_id: 12345,
        session_expires_at: None,
        master_expires_at: None,
    };

    let conn = Arc::new(Mutex::new(Connection {
        settings: HashMap::new(),
        tokens: Arc::new(AsyncRwLock::new(Some(tokens))),
        http_client: Some(reqwest::Client::new()),
        retry_policy: RetryPolicy::default(),
        server_url: Some(format!("http://{}", addr)),
        client_info: Some(test_client_info()),
    }));

    // When multiple concurrent requests receive 401 errors
    let mut handles = vec![];
    for i in 0..3 {
        let conn = conn.clone();
        let server_addr = addr;
        handles.push(tokio::spawn(async move {
            with_valid_session(&conn, |token| {
                let query_addr = server_addr;
                async move {
                    // Make a query request
                    let client = reqwest::Client::new();
                    let resp = client
                        .post(format!("http://{}/queries/v1/query-request", query_addr))
                        .header("Authorization", format!("Snowflake Token=\"{}\"", token))
                        .send()
                        .await
                        .map_err(|e| sf_core::rest::snowflake::RestError::Communication {
                            context: "query".to_string(),
                            source: e,
                            location: snafu::Location::default(),
                        })?;

                    if resp.status() == reqwest::StatusCode::UNAUTHORIZED {
                        return Err(sf_core::rest::snowflake::RestError::InvalidSnowflakeResponse {
                            source: sf_core::rest::snowflake::SnowflakeResponseError::SessionExpired {
                                location: snafu::Location::default(),
                            },
                            location: snafu::Location::default(),
                        });
                    }

                    Ok(format!("request {} succeeded", i))
                }
            })
            .await
        }));
    }

    // Then only one refresh attempt should be made
    // And all requests should succeed after the refresh
    let mut success_count = 0;
    for (i, handle) in handles.into_iter().enumerate() {
        let result = handle.await.expect("task panicked");
        assert!(result.is_ok(), "Request {} failed: {:?}", i, result);
        success_count += 1;
    }
    assert_eq!(success_count, 3, "Expected all 3 requests to succeed");

    assert_eq!(
        refresh_attempts.load(Ordering::SeqCst),
        1,
        "Expected exactly 1 refresh attempt, but got {}",
        refresh_attempts.load(Ordering::SeqCst)
    );

    // Verify the token was updated
    let tokens_lock = conn.lock().unwrap().tokens.clone();
    let final_token = tokens_lock
        .read()
        .await
        .as_ref()
        .unwrap()
        .session_token
        .clone();
    assert_eq!(final_token, "new-session-token");

    server.abort();
}
