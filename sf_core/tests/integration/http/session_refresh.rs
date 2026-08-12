//! Integration tests for session token refresh functionality.

use sf_core::config::rest_parameters::test_fixtures::test_client_info;
use sf_core::rest::snowflake::{RestError, SessionTokens, SnowflakeResponseError, refresh_session};
use sf_core::sensitive::SensitiveString;
use std::net::SocketAddr;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;

fn test_tokens() -> SessionTokens {
    SessionTokens {
        session_token: SensitiveString::from("old-session-token"),
        master_token: SensitiveString::from("valid-master-token"),
        session_id: 12345,
        session_expires_at: None,
        master_expires_at: None,
        master_validity: None,
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
    // Given a server that returns a generic Snowflake refresh error (not a
    // token-lifecycle code, which has dedicated handling)
    let (addr, attempts, server) = spawn_refresh_server(|_| async move {
        let body = r#"{"success":false,"code":"390195","message":"Session refresh was rejected"}"#;
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

    // Then it should fail with the generic session-refresh error
    assert!(result.is_err());
    let err = result.unwrap_err();
    let err_str = err.to_string();
    assert!(
        err_str.contains("Session refresh failed"),
        "Unexpected error message: {}",
        err_str
    );
    assert_eq!(attempts.load(Ordering::SeqCst), 1);
    server.await.unwrap();
}

/// GS 390113/390114/390115 on the refresh endpoint must all surface the
/// discriminable MasterTokenExpired variant (not the generic
/// SessionRefreshFailed), carrying the real GS code forward, so callers can
/// mark the connection expired. Unit-level proof of the refresh_session mapping.
/// Before this fix, only 390114 was special-cased here; 390113/390115 fell
/// through to the generic SessionRefreshFailed error.
#[tokio::test]
async fn should_map_master_token_terminal_codes_to_master_token_expired() {
    for code in [390113, 390114, 390115] {
        let (addr, attempts, server) = spawn_refresh_server(move |_| {
            let body = format!(
                r#"{{"success":false,"code":"{code}","message":"Master token is no longer valid"}}"#
            );
            async move {
                format!(
                    "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                    body.len(),
                    body
                ).into_bytes()
            }
        }).await;

        let client = reqwest::Client::new();
        let server_url = format!("http://{}", addr);

        let result =
            refresh_session(&client, &server_url, &test_client_info(), &test_tokens()).await;

        let err = result.expect_err(&format!("{code} refresh must fail"));
        match err {
            RestError::InvalidSnowflakeResponse {
                source: SnowflakeResponseError::MasterTokenExpired { code: got, .. },
                ..
            } => assert_eq!(got, code, "expected the real GS code to be preserved"),
            other => {
                panic!("expected InvalidSnowflakeResponse{{MasterTokenExpired}}, got {other:?}")
            }
        }
        assert_eq!(attempts.load(Ordering::SeqCst), 1);
        server.await.unwrap();
    }
}

async fn spawn_refresh_server<F, Fut>(
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
        let (mut stream, _) = listener.accept().await.unwrap();
        let attempt = attempts_clone.fetch_add(1, Ordering::SeqCst) + 1;

        // Read the request
        let mut buf = [0u8; 4096];
        let _ = stream.read(&mut buf).await;

        // Send the response
        let response = responder(attempt).await;
        stream.write_all(&response).await.unwrap();
        let _ = stream.shutdown().await;
    });

    (addr, attempts, handle)
}
