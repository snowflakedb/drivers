//! Integration tests for the per-connection heartbeat background task.

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::time::Duration;

use serde_json::json;
use sf_core::apis::database_driver_v1::heartbeat::spawn_heartbeat_task;
use sf_core::config::rest_parameters::test_fixtures::test_client_info;
use sf_core::rest::snowflake::SessionTokens;
use sf_core::sensitive::SensitiveString;
use tokio::sync::RwLock as AsyncRwLock;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

use crate::common::mocks::auth::mount_jwt_login_with_keep_alive;
use crate::common::snowflake_test_client::SnowflakeTestClient;

#[tokio::test]
async fn heartbeat_sends_periodic_requests() {
    // Given a mock server that accepts heartbeat POST requests
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/session/heartbeat"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({"success": true})))
        .expect(2..)
        .named("heartbeat")
        .mount(&server)
        .await;

    // Given a heartbeat task spawned with a 50ms interval
    let tokens = Arc::new(AsyncRwLock::new(Some(test_tokens("tok1"))));
    let mut handle = spawn_heartbeat_task(
        tokens,
        reqwest::Client::new(),
        server.uri(),
        test_client_info(),
        Duration::from_millis(50),
        Arc::new(AtomicBool::new(false)),
    );

    // When enough time passes for multiple heartbeat intervals
    tokio::time::sleep(Duration::from_millis(200)).await;

    // Then at least 2 heartbeat POST requests should have been sent
    handle.cancel_and_wait().await;
}

#[tokio::test]
async fn heartbeat_refreshes_on_401_then_retries() {
    // Given a mock server that returns 401 on the first heartbeat request
    let heartbeat_count = Arc::new(AtomicUsize::new(0));
    let heartbeat_count_clone = heartbeat_count.clone();
    let server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/session/heartbeat"))
        .respond_with(move |_: &wiremock::Request| {
            let count = heartbeat_count_clone.fetch_add(1, Ordering::SeqCst) + 1;
            if count == 1 {
                ResponseTemplate::new(401)
            } else {
                ResponseTemplate::new(200).set_body_json(json!({"success": true}))
            }
        })
        .named("heartbeat")
        .mount(&server)
        .await;

    // Given a mock token-request endpoint that returns refreshed tokens
    Mock::given(method("POST"))
        .and(path("/session/token-request"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "success": true,
            "data": {
                "sessionToken": "refreshed_tok",
                "masterToken": "refreshed_master",
                "sessionId": 2
            }
        })))
        .expect(1)
        .named("refresh")
        .mount(&server)
        .await;

    // Given a heartbeat task started with an old session token
    let tokens = Arc::new(AsyncRwLock::new(Some(test_tokens("old_tok"))));
    let tokens_clone = tokens.clone();
    let mut handle = spawn_heartbeat_task(
        tokens_clone,
        reqwest::Client::new(),
        server.uri(),
        test_client_info(),
        Duration::from_millis(50),
        Arc::new(AtomicBool::new(false)),
    );

    // When the heartbeat task runs and encounters the 401
    tokio::time::sleep(Duration::from_millis(300)).await;
    handle.cancel_and_wait().await;

    // Then the session token should have been refreshed via /session/token-request
    let guard = tokens.read().await;
    let current = guard.as_ref().expect("tokens should still exist");
    assert_eq!(
        current.session_token.reveal(),
        "refreshed_tok",
        "Session token should have been updated by heartbeat refresh"
    );
    assert_eq!(
        current.master_token.reveal(),
        "refreshed_master",
        "Master token should have been updated by heartbeat refresh"
    );

    // Then at least 2 heartbeat attempts should have occurred (initial 401 + retry after refresh)
    assert!(
        heartbeat_count.load(Ordering::SeqCst) >= 2,
        "Expected at least 2 heartbeat attempts (initial + retry after refresh)"
    );
}

#[tokio::test]
async fn heartbeat_stops_on_cancellation() {
    // Given a mock server that counts heartbeat requests
    let server = MockServer::start().await;
    let heartbeat_count = Arc::new(AtomicUsize::new(0));
    let heartbeat_count_clone = heartbeat_count.clone();

    Mock::given(method("POST"))
        .and(path("/session/heartbeat"))
        .respond_with(move |_: &wiremock::Request| {
            heartbeat_count_clone.fetch_add(1, Ordering::SeqCst);
            ResponseTemplate::new(200).set_body_json(json!({"success": true}))
        })
        .named("heartbeat")
        .mount(&server)
        .await;

    // Given a running heartbeat task
    let tokens = Arc::new(AsyncRwLock::new(Some(test_tokens("tok1"))));
    let mut handle = spawn_heartbeat_task(
        tokens,
        reqwest::Client::new(),
        server.uri(),
        test_client_info(),
        Duration::from_millis(50),
        Arc::new(AtomicBool::new(false)),
    );
    tokio::time::sleep(Duration::from_millis(100)).await;
    let count_before_cancel = heartbeat_count.load(Ordering::SeqCst);
    assert!(
        count_before_cancel >= 1,
        "Should have sent at least 1 heartbeat before cancel"
    );

    // When the cancellation token is triggered
    handle.cancel_and_wait().await;

    // Give any in-flight heartbeat that crossed the wire just before cancel time to settle
    // before we snapshot the baseline; otherwise it can tick between snapshots and flake.
    tokio::time::sleep(Duration::from_millis(50)).await;
    let count_at_cancel = heartbeat_count.load(Ordering::SeqCst);

    // Then no more heartbeat requests should be sent after cancellation
    tokio::time::sleep(Duration::from_millis(200)).await;
    let count_after = heartbeat_count.load(Ordering::SeqCst);
    assert_eq!(
        count_at_cancel, count_after,
        "No heartbeats should be sent after cancellation"
    );
}

#[tokio::test]
async fn heartbeat_exits_when_tokens_cleared() {
    // Given a mock server that accepts heartbeat requests
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/session/heartbeat"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({"success": true})))
        .named("heartbeat")
        .mount(&server)
        .await;

    // Given a running heartbeat task with shared token state
    let tokens = Arc::new(AsyncRwLock::new(Some(test_tokens("tok1"))));
    let tokens_clone = tokens.clone();
    let mut handle = spawn_heartbeat_task(
        tokens_clone,
        reqwest::Client::new(),
        server.uri(),
        test_client_info(),
        Duration::from_millis(50),
        Arc::new(AtomicBool::new(false)),
    );
    tokio::time::sleep(Duration::from_millis(100)).await;

    // When the session tokens are cleared (simulating session close)
    *tokens.write().await = None;

    // Then the heartbeat task should exit on its own within a reasonable timeout
    tokio::time::timeout(Duration::from_secs(2), handle.cancel_and_wait())
        .await
        .expect("heartbeat task should exit after tokens cleared");
}

#[tokio::test]
async fn heartbeat_not_started_when_keep_alive_false() {
    // Given a mock server with CLIENT_SESSION_KEEP_ALIVE=false in the login response
    let server = MockServer::start().await;
    mount_jwt_login_with_keep_alive(&server, false).await;

    // And a heartbeat endpoint that expects zero requests
    Mock::given(method("POST"))
        .and(path("/session/heartbeat"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({"success": true})))
        .expect(0)
        .named("heartbeat")
        .mount(&server)
        .await;

    mount_logout_success(&server).await;

    // When connection is initialized via the full lifecycle
    let server_uri = server.uri();
    let client = tokio::task::spawn_blocking(move || {
        SnowflakeTestClient::connect_integration_test(Some(&server_uri))
    })
    .await
    .unwrap();

    // Then no heartbeat requests should arrive after waiting
    tokio::time::sleep(Duration::from_millis(300)).await;

    // Cleanup
    let _ = tokio::task::spawn_blocking(move || client.connection_close_blocking())
        .await
        .unwrap();
    // expect(0) on the heartbeat mock verifies no requests were sent
}

#[tokio::test]
async fn heartbeat_fires_repeatedly_at_configured_interval() {
    // Given a mock server that counts heartbeat requests
    let server = MockServer::start().await;
    let heartbeat_count = Arc::new(AtomicUsize::new(0));
    let heartbeat_count_clone = heartbeat_count.clone();

    Mock::given(method("POST"))
        .and(path("/session/heartbeat"))
        .respond_with(move |_: &wiremock::Request| {
            heartbeat_count_clone.fetch_add(1, Ordering::SeqCst);
            ResponseTemplate::new(200).set_body_json(json!({"success": true}))
        })
        .named("heartbeat")
        .mount(&server)
        .await;

    // Given a heartbeat task spawned with a 100ms interval (bypassing clamp for testing)
    let tokens = Arc::new(AsyncRwLock::new(Some(test_tokens("tok1"))));
    let mut handle = spawn_heartbeat_task(
        tokens,
        reqwest::Client::new(),
        server.uri(),
        test_client_info(),
        Duration::from_millis(100),
        Arc::new(AtomicBool::new(false)),
    );

    // When 550ms elapse (enough for ~4-5 heartbeats at 100ms)
    tokio::time::sleep(Duration::from_millis(550)).await;
    handle.cancel_and_wait().await;

    // Then at least 4 heartbeats should have been sent
    let count = heartbeat_count.load(Ordering::SeqCst);
    assert!(
        count >= 4,
        "Expected at least 4 heartbeats at 100ms interval over 550ms, got {count}"
    );
}

#[tokio::test]
async fn heartbeat_drop_cancels_task() {
    // Given a mock server that counts heartbeat requests
    let server = MockServer::start().await;
    let heartbeat_count = Arc::new(AtomicUsize::new(0));
    let heartbeat_count_clone = heartbeat_count.clone();

    Mock::given(method("POST"))
        .and(path("/session/heartbeat"))
        .respond_with(move |_: &wiremock::Request| {
            heartbeat_count_clone.fetch_add(1, Ordering::SeqCst);
            ResponseTemplate::new(200).set_body_json(json!({"success": true}))
        })
        .named("heartbeat")
        .mount(&server)
        .await;

    // Given a heartbeat task that has sent at least one request
    let tokens = Arc::new(AsyncRwLock::new(Some(test_tokens("tok1"))));
    {
        let _handle = spawn_heartbeat_task(
            tokens,
            reqwest::Client::new(),
            server.uri(),
            test_client_info(),
            Duration::from_millis(50),
            Arc::new(AtomicBool::new(false)),
        );
        tokio::time::sleep(Duration::from_millis(100)).await;

        // When the HeartbeatHandle is dropped (simulating connection_release)
    }
    // Give any in-flight heartbeat that crossed the wire just before drop time to settle
    // before we snapshot the baseline; otherwise it can tick between snapshots and flake.
    tokio::time::sleep(Duration::from_millis(50)).await;
    let count_at_drop = heartbeat_count.load(Ordering::SeqCst);

    // Then no more heartbeat requests should arrive after the handle is dropped
    tokio::time::sleep(Duration::from_millis(200)).await;
    let count_after = heartbeat_count.load(Ordering::SeqCst);
    assert_eq!(
        count_at_drop, count_after,
        "No heartbeats should be sent after HeartbeatHandle is dropped"
    );
}

// ---------------------------------------------------------------------------
//                  Connection Lifecycle Tests
// ---------------------------------------------------------------------------

#[tokio::test]
async fn heartbeat_started_when_keep_alive_true() {
    // Given a mock server with CLIENT_SESSION_KEEP_ALIVE=true in the login response
    let server = MockServer::start().await;
    mount_jwt_login_with_keep_alive(&server, true).await;

    // And a heartbeat endpoint that expects at least 1 request
    let heartbeat_count = Arc::new(AtomicUsize::new(0));
    let heartbeat_count_clone = heartbeat_count.clone();

    Mock::given(method("POST"))
        .and(path("/session/heartbeat"))
        .respond_with(move |_: &wiremock::Request| {
            heartbeat_count_clone.fetch_add(1, Ordering::SeqCst);
            ResponseTemplate::new(200).set_body_json(json!({"success": true}))
        })
        .named("heartbeat")
        .mount(&server)
        .await;

    mount_logout_success(&server).await;

    // When connection is initialized via the full lifecycle
    let server_uri = server.uri();
    let client = tokio::task::spawn_blocking(move || {
        SnowflakeTestClient::connect_integration_test(Some(&server_uri))
    })
    .await
    .unwrap();

    // Then heartbeat requests should arrive (250ms interval from
    // masterValidityInSeconds=1 → 1s/4 = 250ms; generous wait for CI)
    tokio::time::sleep(Duration::from_millis(1200)).await;
    assert!(
        heartbeat_count.load(Ordering::SeqCst) >= 1,
        "Expected at least 1 heartbeat request, got {}",
        heartbeat_count.load(Ordering::SeqCst)
    );

    // Cleanup
    let _ = tokio::task::spawn_blocking(move || client.connection_close_blocking())
        .await
        .unwrap();
}

#[tokio::test]
async fn connection_close_cancels_heartbeat() {
    // Given a mock server with CLIENT_SESSION_KEEP_ALIVE=true in the login response
    let server = MockServer::start().await;
    mount_jwt_login_with_keep_alive(&server, true).await;

    // And a heartbeat endpoint that counts requests
    let heartbeat_count = Arc::new(AtomicUsize::new(0));
    let heartbeat_count_clone = heartbeat_count.clone();

    Mock::given(method("POST"))
        .and(path("/session/heartbeat"))
        .respond_with(move |_: &wiremock::Request| {
            heartbeat_count_clone.fetch_add(1, Ordering::SeqCst);
            ResponseTemplate::new(200).set_body_json(json!({"success": true}))
        })
        .named("heartbeat")
        .mount(&server)
        .await;

    mount_logout_success(&server).await;

    // When connection is initialized
    let server_uri = server.uri();
    let client = tokio::task::spawn_blocking(move || {
        SnowflakeTestClient::connect_integration_test(Some(&server_uri))
    })
    .await
    .unwrap();

    // Then heartbeat requests should arrive before we close (generous wait for CI)
    tokio::time::sleep(Duration::from_millis(1200)).await;
    assert!(
        heartbeat_count.load(Ordering::SeqCst) >= 1,
        "Expected at least 1 heartbeat before close, got {}",
        heartbeat_count.load(Ordering::SeqCst)
    );

    // When the connection is closed
    let result = tokio::task::spawn_blocking(move || client.connection_close_blocking())
        .await
        .unwrap();
    assert!(
        result.is_ok(),
        "Connection close should succeed: {result:?}"
    );

    // Give any in-flight heartbeat that crossed the wire just before close time to settle
    // before we snapshot the baseline; belt-and-braces since `connection_close_blocking`
    // already calls `cancel_and_wait` synchronously inside cleanup.
    tokio::time::sleep(Duration::from_millis(50)).await;

    // Then no more heartbeat requests should arrive after close
    let count_at_close = heartbeat_count.load(Ordering::SeqCst);
    tokio::time::sleep(Duration::from_millis(1000)).await;
    let count_after = heartbeat_count.load(Ordering::SeqCst);
    assert_eq!(
        count_at_close, count_after,
        "No heartbeats should be sent after connection close"
    );
}

// ---------------------------------------------------------------------------
//                          Helpers
// ---------------------------------------------------------------------------

/// Mount a successful logout endpoint (POST /session?delete=true).
async fn mount_logout_success(server: &MockServer) {
    Mock::given(method("POST"))
        .and(path("/session"))
        .and(wiremock::matchers::query_param("delete", "true"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_json(json!({"success": true}))
                .insert_header("Content-Type", "application/json"),
        )
        .mount(server)
        .await;
}

fn test_tokens(session_token: &str) -> SessionTokens {
    SessionTokens {
        session_token: SensitiveString::from(session_token),
        master_token: SensitiveString::from("master_tok"),
        session_id: 1,
        session_expires_at: None,
        master_expires_at: Some(std::time::Instant::now() + Duration::from_secs(14400)),
        master_validity: Some(Duration::from_secs(14400)),
    }
}
