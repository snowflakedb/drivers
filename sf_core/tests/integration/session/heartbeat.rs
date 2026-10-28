//! Integration tests for the per-connection heartbeat background task.

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::time::Duration;

use serde_json::json;
use sf_core::apis::database_driver_v1::heartbeat::spawn_heartbeat_task;
use sf_core::config::rest_parameters::test_fixtures::test_client_info;
use sf_core::protobuf::apis::database_driver_v1::{DriverProviders, WrapperPresets};
use sf_core::rest::snowflake::{SessionTokens, TokenExpiry};
use sf_core::sensitive::SensitiveString;
use tokio::sync::RwLock as AsyncRwLock;
use wiremock::matchers::{body_partial_json, method, path, path_regex};
use wiremock::{Mock, MockServer, ResponseTemplate};

use crate::common::mocks::auth::mount_jwt_login_with_keep_alive;
use crate::common::private_key_helper;
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
        Arc::new(AtomicBool::new(false)),
    );
    // Win32 CI can miss the first tick in 100ms (interval 50ms + HTTP). Wait
    // until the mock has seen at least one request, with a hard deadline.
    let deadline = tokio::time::Instant::now() + Duration::from_secs(2);
    while heartbeat_count.load(Ordering::SeqCst) < 1 && tokio::time::Instant::now() < deadline {
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
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
async fn heartbeat_exits_when_the_session_is_gone() {
    let server = MockServer::start().await;
    let heartbeat_count = Arc::new(AtomicUsize::new(0));
    let heartbeat_count_clone = heartbeat_count.clone();

    Mock::given(method("POST"))
        .and(path("/session/heartbeat"))
        .respond_with(move |_: &wiremock::Request| {
            heartbeat_count_clone.fetch_add(1, Ordering::SeqCst);
            ResponseTemplate::new(401)
        })
        .named("heartbeat")
        .mount(&server)
        .await;

    Mock::given(method("POST"))
        .and(path("/session/token-request"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "success": false,
            "code": "390111",
            "message": "Session no longer exists."
        })))
        .named("refresh_refused")
        .mount(&server)
        .await;

    let session_terminated = Arc::new(AtomicBool::new(false));
    let mut handle = spawn_heartbeat_task(
        Arc::new(AsyncRwLock::new(Some(test_tokens("tok1")))),
        reqwest::Client::new(),
        server.uri(),
        test_client_info(),
        Duration::from_millis(50),
        Arc::new(AtomicBool::new(false)),
        session_terminated.clone(),
    );

    let deadline = tokio::time::Instant::now() + Duration::from_secs(2);
    while !session_terminated.load(Ordering::SeqCst) && tokio::time::Instant::now() < deadline {
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
    assert!(
        session_terminated.load(Ordering::SeqCst),
        "the refused renewal must mark the session"
    );

    let count_when_gone = heartbeat_count.load(Ordering::SeqCst);
    assert_eq!(
        count_when_gone, 1,
        "the task must not keep beating a session the server has taken away"
    );
    // Same window as `heartbeat_stops_on_cancellation`: absence of further ticks is
    // the behavior under test once the session is already marked gone.
    tokio::time::sleep(Duration::from_millis(200)).await;
    assert_eq!(
        heartbeat_count.load(Ordering::SeqCst),
        count_when_gone,
        "the task must not keep beating a session the server has taken away"
    );

    handle.cancel_and_wait().await;
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

#[tokio::test]
async fn should_clamp_login_heartbeat_frequency_to_minimum() {
    assert_login_heartbeat_frequency_is_clamped(100, 900).await;
}

#[tokio::test]
async fn should_clamp_login_heartbeat_frequency_to_maximum() {
    assert_login_heartbeat_frequency_is_clamped(9_000, 3_600).await;
}

async fn assert_login_heartbeat_frequency_is_clamped(requested: i64, expected: i64) {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path_regex(r"/session/v1/login-request.*"))
        .and(body_partial_json(json!({
            "data": {
                "AUTHENTICATOR": "SNOWFLAKE_JWT",
                "SESSION_PARAMETERS": {
                    "CLIENT_SESSION_KEEP_ALIVE_HEARTBEAT_FREQUENCY": expected.to_string()
                }
            }
        })))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "success": true,
            "data": {
                "token": "mock_token",
                "masterToken": "mock_master_token",
                "sessionId": 12345
            }
        })))
        .expect(1)
        .mount(&server)
        .await;

    let server_uri = server.uri();
    let result = tokio::task::spawn_blocking(move || {
        let mut client = SnowflakeTestClient::with_int_tests_params(Some(&server_uri));
        client.set_connection_option("authenticator", "SNOWFLAKE_JWT");
        let temp_key_file = private_key_helper::get_test_private_key_file()
            .expect("failed to create test private key file");
        client.set_connection_option("private_key_file", temp_key_file.path().to_str().unwrap());
        client
            .set_connection_option_int("CLIENT_SESSION_KEEP_ALIVE_HEARTBEAT_FREQUENCY", requested);
        let result = client.connection_init_blocking();
        client.set_temp_key_file(temp_key_file);
        result
    })
    .await
    .expect("connection task panicked");

    assert!(
        result.is_ok(),
        "connection should succeed with heartbeat frequency {requested} clamped to {expected}: {result:?}"
    );
}

// ---------------------------------------------------------------------------
//                     Adopted Session (session_token + master_token)
// ---------------------------------------------------------------------------

/// Eight heartbeat intervals: with a 1 s master validity and no requested
/// frequency the interval is 1/16 s. Asserting that no heartbeat is sent
/// needs a fixed window; there is no event to poll for.
const NO_HEARTBEAT_WINDOW: Duration = Duration::from_millis(500);

#[tokio::test]
async fn should_send_heartbeats_on_an_adopted_session_when_keep_alive_is_true() {
    // Given a mock server that expects at least 2 heartbeats
    let server = MockServer::start().await;
    let heartbeats = server
        .register_as_scoped(heartbeat_mock().expect(2..))
        .await;
    mount_logout_success(&server).await;

    // When a connection adopts a session with keep-alive enabled
    let client = adopt_session(&server, KeepAlive::Option(true)).await;
    let started = tokio::time::timeout(Duration::from_secs(10), heartbeats.wait_until_satisfied())
        .await
        .is_ok();
    close_connection(client).await;

    // Then heartbeats are sent for the adopted session
    assert!(started, "expected at least 2 heartbeats within 10 s");
}

#[tokio::test]
async fn should_not_send_heartbeats_on_an_adopted_session_when_keep_alive_is_omitted() {
    // Given a mock server that expects no heartbeat
    let server = MockServer::start().await;
    heartbeat_mock().expect(0).mount(&server).await;
    mount_logout_success(&server).await;

    // When a connection adopts a session without the keep-alive option
    let client = adopt_session(&server, KeepAlive::Omitted).await;
    tokio::time::sleep(NO_HEARTBEAT_WINDOW).await;
    close_connection(client).await;

    // Then no heartbeat is sent: `expect(0)` is verified when the server drops
}

#[tokio::test]
async fn should_start_heartbeats_on_an_adopted_session_after_a_query_reports_keep_alive() {
    // Given a mock server whose query response reports keep-alive on for the session
    let server = MockServer::start().await;
    let heartbeats = server
        .register_as_scoped(heartbeat_mock().expect(2..))
        .await;
    query_mock(Some(true)).mount(&server).await;
    mount_logout_success(&server).await;

    // When a connection adopts the session without the keep-alive option and runs a query
    let client = adopt_session(&server, KeepAlive::Omitted).await;
    let client = run_query(client).await;
    let started = tokio::time::timeout(Duration::from_secs(10), heartbeats.wait_until_satisfied())
        .await
        .is_ok();
    close_connection(client).await;

    // Then heartbeats are sent for the adopted session
    assert!(started, "expected at least 2 heartbeats within 10 s");
}

#[tokio::test]
async fn should_not_send_heartbeats_on_an_adopted_session_when_a_query_omits_keep_alive() {
    // Given a mock server whose query response does not mention keep-alive
    let server = MockServer::start().await;
    heartbeat_mock().expect(0).mount(&server).await;
    query_mock(None).expect(1).mount(&server).await;
    mount_logout_success(&server).await;

    // When a connection adopts the session without the keep-alive option and runs a query
    let client = adopt_session(&server, KeepAlive::Omitted).await;
    let client = run_query(client).await;
    tokio::time::sleep(NO_HEARTBEAT_WINDOW).await;
    close_connection(client).await;

    // Then no heartbeat is sent: `expect(0)` is verified when the server drops
}

#[tokio::test]
async fn should_not_send_heartbeats_on_an_adopted_session_when_a_query_reports_keep_alive_off() {
    // Given a mock server whose query response reports keep-alive off for the session
    let server = MockServer::start().await;
    heartbeat_mock().expect(0).mount(&server).await;
    query_mock(Some(false)).expect(1).mount(&server).await;
    mount_logout_success(&server).await;

    // When a connection adopts the session without the keep-alive option and runs a query
    let client = adopt_session(&server, KeepAlive::Omitted).await;
    let client = run_query(client).await;
    tokio::time::sleep(NO_HEARTBEAT_WINDOW).await;
    close_connection(client).await;

    // Then no heartbeat is sent: `expect(0)` is verified when the server drops
}

#[tokio::test]
async fn should_not_send_heartbeats_on_an_adopted_session_when_keep_alive_is_false_and_a_query_reports_it_on()
 {
    // Given a mock server whose query response reports keep-alive on for the session
    let server = MockServer::start().await;
    heartbeat_mock().expect(0).mount(&server).await;
    query_mock(Some(true)).mount(&server).await;
    mount_logout_success(&server).await;

    // When a connection adopts the session with keep-alive disabled and runs a query
    let client = adopt_session(&server, KeepAlive::Option(false)).await;
    let client = run_query(client).await;
    tokio::time::sleep(NO_HEARTBEAT_WINDOW).await;
    close_connection(client).await;

    // Then no heartbeat is sent: `expect(0)` is verified when the server drops
}

#[tokio::test]
async fn should_not_send_heartbeats_on_an_adopted_session_when_a_session_parameter_disables_keep_alive_and_a_query_reports_it_on()
 {
    // Given a mock server whose query response reports keep-alive on for the session
    let server = MockServer::start().await;
    heartbeat_mock().expect(0).mount(&server).await;
    query_mock(Some(true)).mount(&server).await;
    mount_logout_success(&server).await;

    // When a connection adopts the session with keep-alive disabled through session parameters and runs a query
    let client = adopt_session(&server, KeepAlive::SessionParameter(false)).await;
    let client = run_query(client).await;
    tokio::time::sleep(NO_HEARTBEAT_WINDOW).await;
    close_connection(client).await;

    // Then no heartbeat is sent: `expect(0)` is verified when the server drops
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

fn heartbeat_mock() -> Mock {
    Mock::given(method("POST"))
        .and(path("/session/heartbeat"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({"success": true})))
        .named("heartbeat")
}

fn query_mock(session_keep_alive: Option<bool>) -> Mock {
    let parameters: Vec<_> = session_keep_alive
        .map(|value| json!({"name": "CLIENT_SESSION_KEEP_ALIVE", "value": value}))
        .into_iter()
        .collect();
    Mock::given(method("POST"))
        .and(path_regex(r"/queries/v1/query-request.*"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "success": true,
            "data": {
                "queryId": "keep-alive-query",
                "queryResultFormat": "json",
                "rowset": [],
                "rowtype": [],
                "parameters": parameters,
            }
        })))
        .named("query")
}

enum KeepAlive {
    Omitted,
    Option(bool),
    SessionParameter(bool),
}

async fn adopt_session(server: &MockServer, keep_alive: KeepAlive) -> SnowflakeTestClient {
    let server_uri = server.uri();
    tokio::task::spawn_blocking(move || {
        let providers = DriverProviders {
            wrapper_presets: WrapperPresets {
                validate_session_token: false,
                ..WrapperPresets::default()
            },
            ..Default::default()
        };
        let client = SnowflakeTestClient::with_int_tests_params_using(Some(&server_uri), providers);
        client.set_connection_option("session_token", "adopted-session-token");
        client.set_connection_option("master_token", "adopted-master-token");
        client.set_connection_option_int("master_validity_in_seconds", 1);
        match keep_alive {
            KeepAlive::Omitted => {}
            KeepAlive::Option(value) => {
                client.set_connection_option_bool("CLIENT_SESSION_KEEP_ALIVE", value);
            }
            KeepAlive::SessionParameter(value) => {
                client.set_session_parameter("CLIENT_SESSION_KEEP_ALIVE", &value.to_string());
            }
        }
        client
            .connect()
            .expect("adopting the session should succeed");
        client
    })
    .await
    .expect("adopt task panicked")
}

async fn run_query(client: SnowflakeTestClient) -> SnowflakeTestClient {
    tokio::task::spawn_blocking(move || {
        client.execute_sql("SELECT 1");
        client
    })
    .await
    .expect("query task panicked")
}

async fn close_connection(client: SnowflakeTestClient) {
    let result = tokio::task::spawn_blocking(move || client.connection_close_blocking())
        .await
        .expect("close task panicked");
    assert!(result.is_ok(), "close should succeed: {result:?}");
}

fn test_tokens(session_token: &str) -> SessionTokens {
    SessionTokens {
        session_token: SensitiveString::from(session_token),
        master_token: SensitiveString::from("master_tok"),
        session_id: Some(1),
        session_expires_at: None,
        master_expires_at: Some(TokenExpiry::from_duration(Duration::from_secs(14400))),
        master_validity: Some(Duration::from_secs(14400)),
    }
}
