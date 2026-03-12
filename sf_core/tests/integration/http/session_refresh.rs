//! Integration tests for session token refresh functionality.

use sf_core::config::rest_parameters::ClientInfo;
use sf_core::crl::config::CrlConfig;
use sf_core::rest::snowflake::{SessionTokens, refresh_session};
use sf_core::sensitive::SensitiveString;
use sf_core::tls::config::TlsConfig;
use std::net::SocketAddr;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;

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

fn test_tokens() -> SessionTokens {
    SessionTokens {
        session_token: SensitiveString::from("old-session-token"),
        master_token: SensitiveString::from("valid-master-token"),
        session_id: 12345,
        session_expires_at: None,
        master_expires_at: None,
    }
}

#[tokio::test]
async fn should_refresh_session_successfully() {
    // Given a server that accepts token refresh requests
    let (addr, attempts, server) = spawn_refresh_server(|_| async move {
        // Successful refresh response
        let body = r#"{"success":true,"data":{"sessionToken":"new-session-token","masterToken":"new-master-token","sessionId":67890}}"#;
        format!(
            "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
            body.len(),
            body
        ).into_bytes()
    }).await;

    let client = reqwest::Client::new();
    let server_url = format!("http://{}", addr);

    // When we refresh the session
    let result = refresh_session(&client, &server_url, &test_client_info(), &test_tokens()).await;

    // Then we should get new tokens
    let new_tokens = result.expect("refresh should succeed");
    assert_eq!(new_tokens.session_token.reveal(), "new-session-token");
    assert_eq!(new_tokens.master_token.reveal(), "new-master-token");
    assert_eq!(new_tokens.session_id, 67890);
    assert_eq!(attempts.load(Ordering::SeqCst), 1);
    server.await.unwrap();
}

#[tokio::test]
async fn should_fail_when_master_token_expired() {
    // Given a server that returns 401 for expired master token
    let (addr, attempts, server) = spawn_refresh_server(|_| async move {
        b"HTTP/1.1 401 Unauthorized\r\nContent-Length: 21\r\nConnection: close\r\n\r\nMaster token expired".to_vec()
    }).await;

    let client = reqwest::Client::new();
    let server_url = format!("http://{}", addr);

    // When we try to refresh the session
    let result = refresh_session(&client, &server_url, &test_client_info(), &test_tokens()).await;

    // Then it should fail with session refresh error
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(err.to_string().contains("Session refresh"));
    assert_eq!(attempts.load(Ordering::SeqCst), 1);
    server.await.unwrap();
}

#[tokio::test]
async fn should_fail_when_refresh_returns_error() {
    // Given a server that returns a Snowflake error response
    let (addr, attempts, server) = spawn_refresh_server(|_| async move {
        let body = r#"{"success":false,"code":"390114","message":"Session token has expired"}"#;
        format!(
            "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
            body.len(),
            body
        ).into_bytes()
    }).await;

    let client = reqwest::Client::new();
    let server_url = format!("http://{}", addr);

    // When we try to refresh the session
    let result = refresh_session(&client, &server_url, &test_client_info(), &test_tokens()).await;

    // Then it should fail with the Snowflake error
    assert!(result.is_err());
    let err = result.unwrap_err();
    // The error message includes both the wrapper and the inner message
    let err_str = err.to_string();
    assert!(
        err_str.contains("Session refresh failed") || err_str.contains("390114"),
        "Unexpected error message: {}",
        err_str
    );
    assert_eq!(attempts.load(Ordering::SeqCst), 1);
    server.await.unwrap();
}

#[tokio::test]
async fn should_retry_refresh_on_503_then_succeed() {
    // Given a server that returns 503 on first attempt, then succeeds
    let (addr, attempts, server) =
        spawn_refresh_server_with_max_connections(2, |attempt| async move {
            if attempt == 1 {
                b"HTTP/1.1 503 Service Unavailable\r\nContent-Length: 19\r\nConnection: close\r\n\r\nService Unavailable"
                    .to_vec()
            } else {
                let body = r#"{"success":true,"data":{"sessionToken":"retried-token","masterToken":"retried-master","sessionId":99999}}"#;
                format!(
                    "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                    body.len(),
                    body
                )
                .into_bytes()
            }
        })
        .await;

    let client = reqwest::Client::new();
    let server_url = format!("http://{}", addr);

    // When we refresh the session
    let result = refresh_session(&client, &server_url, &test_client_info(), &test_tokens()).await;

    // Then it should succeed after retrying
    let new_tokens = result.expect("refresh should succeed after 503 retry");
    assert_eq!(new_tokens.session_token.reveal(), "retried-token");
    assert_eq!(new_tokens.master_token.reveal(), "retried-master");
    assert_eq!(new_tokens.session_id, 99999);
    assert_eq!(
        attempts.load(Ordering::SeqCst),
        2,
        "Should have made exactly 2 attempts"
    );
    server.await.unwrap();
}

#[tokio::test]
async fn should_retry_refresh_on_429_then_succeed() {
    // Given a server that returns 429 Too Many Requests on first attempt, then succeeds
    let (addr, attempts, server) =
        spawn_refresh_server_with_max_connections(2, |attempt| async move {
            if attempt == 1 {
                b"HTTP/1.1 429 Too Many Requests\r\nContent-Length: 17\r\nConnection: close\r\n\r\nToo Many Requests"
                    .to_vec()
            } else {
                let body = r#"{"success":true,"data":{"sessionToken":"after-429","masterToken":"after-429-master","sessionId":42900}}"#;
                format!(
                    "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                    body.len(),
                    body
                )
                .into_bytes()
            }
        })
        .await;

    let client = reqwest::Client::new();
    let server_url = format!("http://{}", addr);

    // When we refresh the session
    let result = refresh_session(&client, &server_url, &test_client_info(), &test_tokens()).await;

    // Then it should succeed after retrying
    let new_tokens = result.expect("refresh should succeed after 429 retry");
    assert_eq!(new_tokens.session_token.reveal(), "after-429");
    assert_eq!(
        attempts.load(Ordering::SeqCst),
        2,
        "Should have made exactly 2 attempts"
    );
    server.await.unwrap();
}

#[tokio::test]
async fn should_not_retry_refresh_on_401() {
    // Given a server that returns 401 (should not be retried)
    let (addr, attempts, server) = spawn_refresh_server_with_max_connections(2, |_| async move {
        b"HTTP/1.1 401 Unauthorized\r\nContent-Length: 12\r\nConnection: close\r\n\r\nUnauthorized"
            .to_vec()
    })
    .await;

    let client = reqwest::Client::new();
    let server_url = format!("http://{}", addr);

    // When we try to refresh the session
    let result = refresh_session(&client, &server_url, &test_client_info(), &test_tokens()).await;

    // Then it should fail immediately without retrying
    assert!(result.is_err());
    assert_eq!(
        attempts.load(Ordering::SeqCst),
        1,
        "Should NOT retry on 401"
    );
    server.abort();
}

async fn spawn_refresh_server<F, Fut>(
    responder: F,
) -> (SocketAddr, Arc<AtomicUsize>, tokio::task::JoinHandle<()>)
where
    F: Fn(usize) -> Fut + Send + Sync + 'static,
    Fut: std::future::Future<Output = Vec<u8>> + Send + 'static,
{
    spawn_refresh_server_with_max_connections(1, responder).await
}

async fn spawn_refresh_server_with_max_connections<F, Fut>(
    max_connections: usize,
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
        for _ in 0..max_connections {
            let (mut stream, _) = listener.accept().await.unwrap();
            let attempt = attempts_clone.fetch_add(1, Ordering::SeqCst) + 1;

            let mut buf = [0u8; 4096];
            let _ = stream.read(&mut buf).await;

            let response = responder(attempt).await;
            stream.write_all(&response).await.unwrap();
            let _ = stream.shutdown().await;
        }
    });

    (addr, attempts, handle)
}
