//! Integration tests for session logout functionality.
//!
//! These tests use mock HTTP servers (wiremock, spawn_test_server) to verify
//! logout behavior without connecting to real Snowflake.

use crate::common::mocks::auth::mount_jwt_login_success;
use crate::common::mocks::session::is_logout_request;
use crate::common::private_key_helper;
use crate::common::snowflake_test_client::SnowflakeTestClient;
use crate::common::test_server::{
    json_error_response, json_response, service_unavailable_response, spawn_test_server,
};
use serde_json::json;
use sf_core::config::logout::{ErrorStrategy, LogoutConfig};
use sf_core::config::rest_parameters::ClientInfo;
use sf_core::config::retry::RetryPolicy;
use sf_core::protobuf::generated::database_driver_v1::*;
use sf_core::rest::snowflake::logout::logout_session;
use sf_core::sensitive::SensitiveString;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::{Duration, Instant};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;
use tokio::time::sleep;
use wiremock::matchers::{body_partial_json, header, method, path, path_regex, query_param};
use wiremock::{Mock, MockServer, ResponseTemplate};

/// Create a thread-local capturing subscriber for log assertions.
///
/// Returns a (guard, buffer) pair. The guard must be held alive for the duration
/// of the test. Logs are captured into the buffer. Use `get_captured_logs(&buf)`
/// to read them.
///
/// This avoids the global subscriber conflict that #[traced_test] causes when
/// other tests in the same binary call setup_logging() (which uses try_init).
fn capturing_subscriber() -> (
    tracing::subscriber::DefaultGuard,
    &'static std::sync::Mutex<Vec<u8>>,
) {
    let buf: &'static std::sync::Mutex<Vec<u8>> =
        Box::leak(Box::new(std::sync::Mutex::new(Vec::new())));
    let mock_writer = tracing_test::internal::MockWriter::new(buf);
    let dispatch = tracing_test::internal::get_subscriber(mock_writer, "trace");
    let guard = tracing::dispatcher::set_default(&dispatch);
    (guard, buf)
}

/// Read captured logs as a string.
fn get_captured_logs(buf: &std::sync::Mutex<Vec<u8>>) -> String {
    String::from_utf8(buf.lock().unwrap().clone()).unwrap_or_default()
}

// ===========================================================================
//                      HTTP Request Construction
// ===========================================================================

#[tokio::test]
async fn should_construct_logout_request_with_correct_http_method_url_headers_and_body() {
    //Given Mock HTTP server is configured to capture requests
    let server = MockServer::start().await;
    mount_jwt_login_success(&server).await;

    Mock::given(method("POST"))
        .and(path("/session"))
        .and(query_param("delete", "true"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_json(json!({ "success": true }))
                .insert_header("Content-Type", "application/json"),
        )
        .mount(&server)
        .await;

    //And UD Core connection is logged in
    let server_uri = server.uri();
    let client = tokio::task::spawn_blocking(move || {
        SnowflakeTestClient::connect_integration_test(Some(&server_uri))
    })
    .await
    .unwrap();

    //When Logout is initiated
    let result = tokio::task::spawn_blocking(move || client.connection_close_blocking())
        .await
        .unwrap();

    //Then Logout succeeds
    assert!(result.is_ok(), "Close should succeed: {:?}", result.err());

    let received_requests = server.received_requests().await.unwrap();
    let logout_request = received_requests
        .iter()
        .find(|r| is_logout_request(r))
        .expect("Should have made a logout request");

    //Then HTTP method is POST
    assert_eq!(
        logout_request.method.as_str(),
        "POST",
        "Should be POST request"
    );

    //And Request URL path is /session
    assert_eq!(
        logout_request.url.path(),
        "/session",
        "Should request /session"
    );

    let url_str = logout_request.url.to_string();

    //And Query parameter delete is set to true
    assert!(url_str.contains("delete=true"), "Should have delete=true");

    //And Query parameter requestId is present and static across attempts
    assert!(url_str.contains("requestId="), "Should have requestId");

    //And Query parameter request_guid is present and unique per attempt
    assert!(
        url_str.contains("request_guid="),
        "Should have request_guid"
    );

    //And Authorization header is present with format "Snowflake Token={session_token}"
    let auth_header = logout_request
        .headers
        .get("authorization")
        .expect("Should have Authorization header");
    let auth_value = auth_header.to_str().unwrap();
    assert!(
        auth_value.starts_with("Snowflake Token=\"") && auth_value.ends_with('"'),
        "Authorization should have format Snowflake Token=\"...\", got: {}",
        auth_value
    );

    //And Content-Type header is application/json
    let content_type = logout_request
        .headers
        .get("content-type")
        .expect("Should have Content-Type header");
    assert_eq!(
        content_type.to_str().unwrap(),
        "application/json",
        "Should have Content-Type: application/json"
    );

    //And Accept header is application/snowflake
    let accept = logout_request
        .headers
        .get("accept")
        .expect("Should have Accept header");
    assert_eq!(
        accept.to_str().unwrap(),
        "application/snowflake",
        "Should have Accept: application/snowflake"
    );

    //And User-Agent header contains UD version and Rust version
    let user_agent = logout_request
        .headers
        .get("user-agent")
        .expect("Should have User-Agent header");
    assert!(
        user_agent.to_str().unwrap().contains("/"),
        "User-Agent should contain app/version, got: {}",
        user_agent.to_str().unwrap()
    );

    //And Request body is exactly empty JSON object {}
    let body_str = String::from_utf8_lossy(&logout_request.body);
    assert_eq!(
        body_str.trim(),
        "{}",
        "Should have empty JSON object body, got: {}",
        body_str
    );
}

#[test]
fn should_not_send_logout_when_connection_was_never_established() {
    use sf_core::protobuf::apis::database_driver_v1::{
        DatabaseDriverClientBlockingExt, database_driver_client,
    };

    //Given Connection handle created but never initialized
    let client = database_driver_client();
    let db_handle = client
        .database_new_blocking(DatabaseNewRequest {})
        .unwrap()
        .db_handle
        .unwrap();
    client
        .database_init_blocking(DatabaseInitRequest {
            db_handle: Some(db_handle),
        })
        .unwrap();
    let conn_handle = client
        .connection_new_blocking(ConnectionNewRequest {})
        .unwrap()
        .conn_handle
        .unwrap();
    // Note: connection_init() NOT called - connection remains uninitialized

    //When Connection close is attempted
    let result = client.connection_close_blocking(ConnectionCloseRequest {
        conn_handle: Some(conn_handle),
    });

    //Then Close succeeds without sending HTTP request
    assert!(
        result.is_ok(),
        "Connection close should succeed for uninitialized connection"
    );

    //And Connection is marked as closed
    assert!(
        client
            .connection_is_closed_blocking(ConnectionIsClosedRequest {
                conn_handle: Some(conn_handle),
            })
            .unwrap()
            .is_closed,
        "Connection should be marked closed"
    );

    // Cleanup
    client
        .connection_release_blocking(ConnectionReleaseRequest {
            conn_handle: Some(conn_handle),
        })
        .unwrap();
    client
        .database_release_blocking(DatabaseReleaseRequest {
            db_handle: Some(db_handle),
        })
        .unwrap();
}

// ===========================================================================
//                      Parameter-Based Logout Control
// ===========================================================================

#[tokio::test]
async fn should_not_send_logout_when_server_session_keep_alive_is_explicitly_true() {
    //Given Mock HTTP server is configured
    let server = MockServer::start().await;
    mount_jwt_login_success(&server).await;

    //And UD Core connection is logged in with server_session_keep_alive set to true
    let server_uri = server.uri();
    let client = tokio::task::spawn_blocking(move || {
        let mut client = SnowflakeTestClient::with_int_tests_params(Some(&server_uri));

        // Configure JWT authentication
        client.set_connection_option("authenticator", "SNOWFLAKE_JWT");
        let temp_key_file = private_key_helper::get_test_private_key_file()
            .expect("Failed to create test private key file");
        client.set_connection_option("private_key_file", temp_key_file.path().to_str().unwrap());

        // Configure logout behavior BEFORE connection_init
        client.set_connection_option_bool("server_session_keep_alive", true);

        // Initialize connection
        client.connection_init_blocking().unwrap();

        client.set_temp_key_file(temp_key_file);
        client
    })
    .await
    .unwrap();

    //When Connection is closed
    let result = tokio::task::spawn_blocking(move || client.connection_close_blocking())
        .await
        .unwrap();

    //Then No logout HTTP request is sent to server
    assert!(result.is_ok(), "Close should succeed");

    // Verify no logout request was made by checking server received requests
    let received_requests = server.received_requests().await.unwrap();
    assert_eq!(
        received_requests.len(),
        1,
        "Should have received exactly 1 request (login only)"
    );

    // Verify the single request was a login request, not a logout request
    let request = &received_requests[0];
    let url = request.url.to_string();
    assert!(
        url.contains("/session/v1/login-request"),
        "Request should be to login endpoint, not logout. URL: {}",
        url
    );
    assert!(
        !url.contains("delete=true"),
        "Request should not have delete=true query parameter (logout). URL: {}",
        url
    );
}

#[tokio::test]
async fn should_send_logout_when_server_session_keep_alive_is_explicitly_false() {
    //Given Mock HTTP server is configured
    let server = MockServer::start().await;
    mount_jwt_login_success(&server).await;

    // Mount logout endpoint mock
    Mock::given(method("POST"))
        .and(path("/session"))
        .and(query_param("delete", "true"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_json(json!({ "success": true }))
                .insert_header("Content-Type", "application/json"),
        )
        .mount(&server)
        .await;

    //And UD Core connection is logged in with server_session_keep_alive set to false
    let server_uri = server.uri();
    let client = tokio::task::spawn_blocking(move || {
        let mut client = SnowflakeTestClient::with_int_tests_params(Some(&server_uri));

        // Configure JWT authentication
        client.set_connection_option("authenticator", "SNOWFLAKE_JWT");
        let temp_key_file = private_key_helper::get_test_private_key_file()
            .expect("Failed to create test private key file");
        client.set_connection_option("private_key_file", temp_key_file.path().to_str().unwrap());

        // Configure logout behavior BEFORE connection_init
        client.set_connection_option_bool("server_session_keep_alive", false);

        // Initialize connection
        client.connection_init_blocking().unwrap();

        client.set_temp_key_file(temp_key_file);
        client
    })
    .await
    .unwrap();

    //When Connection is closed
    let result = tokio::task::spawn_blocking(move || client.connection_close_blocking())
        .await
        .unwrap();

    //Then Logout HTTP request is sent to server
    assert!(result.is_ok(), "Close should succeed");
    let received_requests = server.received_requests().await.unwrap();
    // Verify there was a logout request
    let logout_count = received_requests
        .iter()
        .filter(|r| is_logout_request(r))
        .count();
    assert_eq!(
        logout_count, 1,
        "Should have received exactly 1 logout request"
    );
}
// ===========================================================================
//                      Default Configuration
// ===========================================================================

#[tokio::test]
async fn should_timeout_after_15_seconds_by_default_when_server_does_not_respond() {
    //Given Mock HTTP server holds connection open for 20 seconds without responding
    let server = MockServer::start().await;
    mount_jwt_login_success(&server).await;

    // Logout endpoint delays 20s — longer than Core's default 15s total timeout.
    // The retry loop enforces max_elapsed as a hard bound on in-flight
    // requests (remaining budget applied as per-request timeout when
    // per_request_timeout is None).
    Mock::given(method("POST"))
        .and(path("/session"))
        .and(query_param("delete", "true"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_json(json!({ "success": true }))
                .insert_header("Content-Type", "application/json")
                .set_delay(Duration::from_secs(20)),
        )
        .mount(&server)
        .await;

    //And UD Core connection is logged in with no timeout override
    let server_uri = server.uri();
    let client = tokio::task::spawn_blocking(move || {
        SnowflakeTestClient::connect_integration_test(Some(&server_uri))
    })
    .await
    .unwrap();

    //When Logout is initiated
    let start = Instant::now();
    let result = tokio::task::spawn_blocking(move || client.connection_close_blocking())
        .await
        .unwrap();
    let elapsed = start.elapsed();

    //Then Close throws timeout error
    assert!(result.is_err(), "Should timeout with default 15s budget");

    //And Total elapsed time is between 14 and 17 seconds
    assert!(
        elapsed >= Duration::from_secs(14) && elapsed < Duration::from_secs(18),
        "Should timeout after ~15 seconds, took {:?}",
        elapsed
    );
}

#[tokio::test]
async fn should_cancel_individual_request_when_per_request_socket_timeout_exceeded() {
    //Given Mock HTTP server holds connection open for 8 seconds on first attempt then succeeds immediately
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let attempts = Arc::new(AtomicUsize::new(0));
    let attempts_clone = attempts.clone();

    let server = tokio::spawn(async move {
        // Handle connections concurrently to avoid blocking retries
        for _ in 0..2 {
            let (mut stream, _) = listener.accept().await.unwrap();
            let attempts_ref = attempts_clone.clone();

            tokio::spawn(async move {
                let attempt = attempts_ref.fetch_add(1, Ordering::SeqCst) + 1;
                let mut buf = vec![0u8; 4096];
                let _ = stream.read(&mut buf).await;

                if attempt == 1 {
                    // First attempt: hold for 8 seconds (longer than 2s socket timeout)
                    sleep(Duration::from_secs(8)).await;
                    // Client will have given up by now - don't send response
                } else {
                    // Second attempt: respond immediately
                    let response = json_response(r#"{"success":true}"#);
                    let _ = stream.write_all(&response).await;
                    let _ = stream.shutdown().await;
                }
            });
        }
    });

    let server_url = format!("http://{}", addr);
    let client = reqwest::Client::builder().no_proxy().build().unwrap();
    let client_info = test_client_info();

    //And UD Core connection is logged in
    //And Per-request socket timeout is set to 2 seconds
    let per_request_timeout = Duration::from_secs(2);

    //And Total retry budget timeout is set to 10 seconds
    let total_timeout = Duration::from_secs(10);

    // Build retry policy with total budget matching connection_close behavior
    let retry_policy = RetryPolicy {
        max_elapsed: Some(total_timeout),
        per_request_timeout: Some(per_request_timeout),
        ..Default::default()
    };

    //When Logout is initiated
    let start = Instant::now();
    let result = logout_session(
        &client,
        &server_url,
        &SensitiveString::from("test_token"),
        &client_info,
        &retry_policy,
        tokio_util::sync::CancellationToken::new(),
    )
    .await;
    let elapsed = start.elapsed();

    //Then First request is cancelled after 2 seconds due to socket timeout
    //And Retry proceeds because total budget still has time remaining
    //And Second request succeeds immediately
    //And Close succeeds
    assert!(
        result.is_ok(),
        "Should succeed after retry: {:?}",
        result.err()
    );
    assert_eq!(
        attempts.load(Ordering::SeqCst),
        2,
        "Should have made 2 attempts"
    );

    // Total time should be ~2s (first timeout) + backoff + ~0s (second immediate)
    assert!(
        elapsed < Duration::from_secs(6),
        "Should complete in reasonable time, took {:?}",
        elapsed
    );

    server.await.unwrap();
}

#[tokio::test]
async fn should_respect_total_retry_budget_timeout_across_all_attempts() {
    //Given Mock HTTP server responds with 503 after 2 second delay on each attempt
    let (addr, attempts, server) = spawn_test_server(10, |_| async move {
        sleep(Duration::from_secs(2)).await;
        service_unavailable_response(r#"{"success":false}"#, 0)
    })
    .await;

    //And UD Core connection is logged in
    let server_url = format!("http://{}", addr);
    let client = reqwest::Client::builder().no_proxy().build().unwrap();
    let client_info = test_client_info();

    //And Total retry budget timeout is set to 5 seconds
    let total_timeout = Duration::from_secs(5);

    //And Retry policy allows 10 attempts
    let retry_policy = RetryPolicy {
        max_attempts: 10,
        max_elapsed: Some(total_timeout),
        per_request_timeout: Some(total_timeout),
        ..Default::default()
    };

    //When Logout is initiated
    let start = Instant::now();
    let result = logout_session(
        &client,
        &server_url,
        &SensitiveString::from("test_token"),
        &client_info,
        &retry_policy,
        tokio_util::sync::CancellationToken::new(),
    )
    .await;
    let elapsed = start.elapsed();

    //Then Fewer than 4 attempts are made
    let attempt_count = attempts.load(Ordering::SeqCst);
    assert!(
        attempt_count < 4,
        "Should make fewer than 4 attempts, made {}",
        attempt_count
    );

    //And The last attempt times out because remaining budget is less than server response time
    assert!(result.is_err(), "Should fail due to timeout/exhaustion");

    //And Total wall-clock time does not exceed 7 seconds for closing the connection
    assert!(
        elapsed < Duration::from_secs(7),
        "Should not exceed 7 seconds, took {:?}",
        elapsed
    );

    server.abort(); // Server expects 10 requests but budget limits to ~3
}

// ===========================================================================
//                  Close vs Active Query Execution
// ===========================================================================
// TODO: SNOW-2923705 - Tests removed until query execution is implemented
// These tests had Gherkin comments but no real implementation, which could
// trick the Gherkin validator. See tests/definitions/core/session/logout.feature
// for the scenarios that need implementation once query execution is ready.

// ===========================================================================
//                  Close vs Token Refresh
// ===========================================================================
// TODO: SNOW-2923705 - Tests removed until token refresh coordination is implemented
// These tests had Gherkin comments but no real implementation.
// See tests/definitions/core/session/logout.feature for scenarios.

// ===========================================================================
//                  Error Strategy Behavior (Injected Strategy Testing)
// ===========================================================================

#[tokio::test]
async fn should_ignore_session_gone_390111_for_each_strategy_type() {
    // Scenario Outline with Examples: strict, best-effort
    for (strategy_type, error_strategy) in [
        ("strict", ErrorStrategy::Strict),
        ("best-effort", ErrorStrategy::BestEffort),
    ] {
        //Given Core logout function called with <strategy_type> strategy
        let (addr, _, server) = spawn_test_server(1, |_| async move {
            json_error_response(
                410,
                "Gone",
                r#"{"success":false,"message":"Session gone","code":"390111"}"#,
            )
        })
        .await;

        let server_url = format!("http://{}", addr);
        let client = reqwest::Client::builder().no_proxy().build().unwrap();
        let client_info = test_client_info();

        //And Mock HTTP server returns SESSION_GONE 390111
        let _config = LogoutConfig {
            error_strategy,
            ..Default::default()
        };

        //When Logout is executed
        let result = logout_session(
            &client,
            &server_url,
            &SensitiveString::from("test_token"),
            &client_info,
            &RetryPolicy::default(),
            tokio_util::sync::CancellationToken::new(),
        )
        .await;

        //Then Close succeeds
        assert!(
            result.is_ok(),
            "SESSION_GONE should be treated as success for {}",
            strategy_type
        );

        //And Error is ignored
        let server_result = server.await;
        assert!(
            server_result.is_ok(),
            "Mock server should complete cleanly for {} — SESSION_GONE was absorbed",
            strategy_type
        );
    }
}

#[tokio::test]
async fn should_retry_logout_on_retryable_error_type_for_each_strategy_type() {
    // Scenario Outline: Examples (error_type, strategy_type)
    // 503 Service Unavailable × (strict, best-effort)
    // 429 Too Many Requests × (strict, best-effort)
    // connection reset × (strict, best-effort)

    for (strategy_name, error_strategy) in [
        ("strict", ErrorStrategy::Strict),
        ("best-effort", ErrorStrategy::BestEffort),
    ] {
        // Test HTTP error codes (503, 429)
        for (error_type, error_response_fn) in [
            (
                "503 Service Unavailable",
                (|| service_unavailable_response(r#"{"success":false}"#, 0)) as fn() -> Vec<u8>,
            ),
            (
                "429 Too Many Requests",
                (|| {
                    json_error_response(
                        429,
                        "Too Many Requests",
                        r#"{"success":false,"message":"Rate limited"}"#,
                    )
                }) as fn() -> Vec<u8>,
            ),
        ] {
            //Given Core logout function called with <strategy_type> strategy
            let _config = LogoutConfig {
                error_strategy,
                ..Default::default()
            };

            //And Mock HTTP server returns <error_type> on attempt 1
            let attempt_1_response_fn = error_response_fn;

            //And Mock HTTP server returns 200 on attempt 2
            let (addr, attempts, server) = spawn_test_server(2, move |attempt| {
                let error_fn = attempt_1_response_fn;
                async move {
                    if attempt == 1 {
                        error_fn()
                    } else {
                        json_response(r#"{"success":true}"#)
                    }
                }
            })
            .await;

            let server_url = format!("http://{}", addr);
            let client = reqwest::Client::builder().no_proxy().build().unwrap();
            let client_info = test_client_info();

            //When Logout is executed
            let result = logout_session(
                &client,
                &server_url,
                &SensitiveString::from("test_token"),
                &client_info,
                &RetryPolicy::default(),
                tokio_util::sync::CancellationToken::new(),
            )
            .await;

            //Then Logout is retried
            assert_eq!(
                attempts.load(Ordering::SeqCst),
                2,
                "Should retry on {} for {}",
                error_type,
                strategy_name
            );

            //And Close succeeds
            assert!(
                result.is_ok(),
                "Should succeed after retry on {} for {}",
                error_type,
                strategy_name
            );

            server.await.unwrap();
        }

        // Test connection reset (requires different server setup)
        {
            let error_type = "connection reset";
            //Given Core logout function called with <strategy_type> strategy
            let _config = LogoutConfig {
                error_strategy,
                ..Default::default()
            };

            //And Mock HTTP server resets connection on first attempt
            let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
            let addr = listener.local_addr().unwrap();
            let attempts = Arc::new(AtomicUsize::new(0));
            let attempts_clone = attempts.clone();

            //And Mock HTTP server succeeds on second attempt
            let server = tokio::spawn(async move {
                loop {
                    let (mut stream, _) = listener.accept().await.unwrap();
                    let attempt = attempts_clone.fetch_add(1, Ordering::SeqCst) + 1;

                    if attempt == 1 {
                        drop(stream); // Reset connection on first attempt
                    } else {
                        // Second attempt: succeed
                        let mut buf = vec![0u8; 4096];
                        let _ = stream.read(&mut buf).await;
                        let response = json_response(r#"{"success":true}"#);
                        stream.write_all(&response).await.unwrap();
                        let _ = stream.shutdown().await;
                        break;
                    }
                }
            });

            let server_url = format!("http://{}", addr);
            let client = reqwest::Client::builder().no_proxy().build().unwrap();
            let client_info = test_client_info();

            //When Logout is executed
            let result = logout_session(
                &client,
                &server_url,
                &SensitiveString::from("test_token"),
                &client_info,
                &RetryPolicy::default(),
                tokio_util::sync::CancellationToken::new(),
            )
            .await;

            //Then Logout is retried
            assert_eq!(
                attempts.load(Ordering::SeqCst),
                2,
                "Should retry on {} for {}",
                error_type,
                strategy_name
            );

            //And Close succeeds
            assert!(
                result.is_ok(),
                "Should succeed after retry on {} for {}",
                error_type,
                strategy_name
            );

            server.await.unwrap();
        }
    }
}

#[tokio::test]
async fn should_attempt_token_refresh_on_390112_when_retries_allowed_for_each_strategy_type() {
    // Scenario Outline: Examples (strategy_type)
    // strict, best-effort
    for (strategy_name, error_strategy) in [
        ("strict", ErrorStrategy::Strict),
        ("best-effort", ErrorStrategy::BestEffort),
    ] {
        //Given Core logout function called with <strategy_type> strategy
        let server = MockServer::start().await;

        // Login mock: initial tokens
        Mock::given(method("POST"))
            .and(path_regex(r"/session/v1/login-request.*"))
            .and(body_partial_json(json!({
                "data": { "AUTHENTICATOR": "SNOWFLAKE_JWT" }
            })))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_json(json!({
                        "success": true,
                        "data": {
                            "token": "initial-session-token",
                            "masterToken": "valid-master-token",
                            "sessionId": 12345
                        }
                    }))
                    .insert_header("Content-Type", "application/json"),
            )
            .mount(&server)
            .await;

        //And Mock HTTP server returns SESSION_TOKEN_EXPIRED 390112 on first attempt
        Mock::given(method("POST"))
            .and(path("/session"))
            .and(query_param("delete", "true"))
            .and(header(
                "Authorization",
                "Snowflake Token=\"initial-session-token\"",
            ))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_json(json!({
                        "success": false,
                        "code": "390112",
                        "message": "Session token expired"
                    }))
                    .insert_header("Content-Type", "application/json"),
            )
            .up_to_n_times(1)
            .mount(&server)
            .await;

        //And Mock HTTP server returns 200 after token refresh
        Mock::given(method("POST"))
            .and(path_regex(r"/session/token-request.*"))
            .and(header(
                "Authorization",
                "Snowflake Token=\"valid-master-token\"",
            ))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_json(json!({
                        "success": true,
                        "data": {
                            "sessionToken": "refreshed-session-token",
                            "masterToken": "valid-master-token",
                            "sessionId": 12345
                        }
                    }))
                    .insert_header("Content-Type", "application/json"),
            )
            .mount(&server)
            .await;

        // Second logout attempt with refreshed token: succeeds.
        // .expect(1) proves the retry used the new token, not the old one.
        Mock::given(method("POST"))
            .and(path("/session"))
            .and(query_param("delete", "true"))
            .and(header(
                "Authorization",
                "Snowflake Token=\"refreshed-session-token\"",
            ))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_json(json!({ "success": true }))
                    .insert_header("Content-Type", "application/json"),
            )
            .expect(1)
            .mount(&server)
            .await;

        //And Retry policy allows 1 retry
        let server_uri = server.uri();
        let client = tokio::task::spawn_blocking(move || {
            let mut client = SnowflakeTestClient::with_int_tests_params(Some(&server_uri));

            client.set_connection_option("authenticator", "SNOWFLAKE_JWT");
            let temp_key_file = private_key_helper::get_test_private_key_file()
                .expect("Failed to create test private key file");
            client
                .set_connection_option("private_key_file", temp_key_file.path().to_str().unwrap());

            client.set_connection_option_bool("server_session_keep_alive", false);
            client.set_logout_error_strategy(error_strategy);
            client.set_connection_option_int("logout_total_timeout_seconds", 30);
            client.set_connection_option_int("logout_max_attempts", 1); // Token refresh retry is "free"

            client.connection_init_blocking().unwrap();

            client.set_temp_key_file(temp_key_file);
            client
        })
        .await
        .unwrap();

        //When Logout is executed
        let result = tokio::task::spawn_blocking(move || client.connection_close_blocking())
            .await
            .unwrap();

        // Verify requests: login + logout(390112) + token-refresh + logout(success)
        let received_requests = server.received_requests().await.unwrap();

        //Then Token refresh request is sent to server
        assert!(
            received_requests
                .iter()
                .any(|r| r.url.path().contains("token-request")),
            "Should have made token refresh request for {}",
            strategy_name,
        );

        //And Logout is retried with new session token
        let logout_count = received_requests
            .iter()
            .filter(|r| is_logout_request(r))
            .count();
        assert!(
            logout_count >= 2,
            "Should have made at least 2 logout requests for {}, got {}",
            strategy_name,
            logout_count,
        );

        //And Close succeeds
        assert!(
            result.is_ok(),
            "Close should succeed after token refresh for {}: {:?}",
            strategy_name,
            result.err()
        );
    }
}

#[tokio::test]
async fn should_fail_gracefully_when_token_refresh_fails_on_390112_for_each_strategy_type() {
    // Scenario Outline: Examples (strategy_type)
    // Tests that when 390112 triggers a refresh but the master token is also expired,
    // Strict raises the error and BestEffort suppresses it.
    for (strategy_name, error_strategy, should_succeed) in [
        ("strict", ErrorStrategy::Strict, false),
        ("best-effort", ErrorStrategy::BestEffort, true),
    ] {
        //Given Mock HTTP server configured with login, 390112 logout, and failed token refresh
        let server = MockServer::start().await;

        // Login: returns initial tokens with master token that will fail refresh
        Mock::given(method("POST"))
            .and(path_regex(r"/session/v1/login-request.*"))
            .and(body_partial_json(json!({
                "data": { "AUTHENTICATOR": "SNOWFLAKE_JWT" }
            })))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_json(json!({
                        "success": true,
                        "data": {
                            "token": "initial-session-token",
                            "masterToken": "expired-master-token",
                            "sessionId": 12345
                        }
                    }))
                    .insert_header("Content-Type", "application/json"),
            )
            .mount(&server)
            .await;

        // Logout attempt: returns 390112 SESSION_TOKEN_EXPIRED
        Mock::given(method("POST"))
            .and(path("/session"))
            .and(query_param("delete", "true"))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_json(json!({
                        "success": false,
                        "code": "390112",
                        "message": "Session token expired"
                    }))
                    .insert_header("Content-Type", "application/json"),
            )
            .mount(&server)
            .await;

        // Token refresh: fails with 401 (master token expired)
        Mock::given(method("POST"))
            .and(path_regex(r"/session/token-request.*"))
            .respond_with(ResponseTemplate::new(401).set_body_string("Master token expired"))
            .mount(&server)
            .await;

        //And UD Core connection is configured and logged in with <strategy_type> strategy
        let server_uri = server.uri();
        let client = tokio::task::spawn_blocking(move || {
            let mut client = SnowflakeTestClient::with_int_tests_params(Some(&server_uri));

            // Configure JWT authentication
            client.set_connection_option("authenticator", "SNOWFLAKE_JWT");
            let temp_key_file = private_key_helper::get_test_private_key_file()
                .expect("Failed to create test private key file");
            client
                .set_connection_option("private_key_file", temp_key_file.path().to_str().unwrap());

            // Configure logout behavior BEFORE connection_init
            client.set_connection_option_bool("server_session_keep_alive", false);
            client.set_logout_error_strategy(error_strategy);
            client.set_connection_option_int("logout_total_timeout_seconds", 30);
            client.set_connection_option_int("logout_max_attempts", 1); // 1 attempt (0 retries)

            // Initialize connection
            client.connection_init_blocking().unwrap();

            client.set_temp_key_file(temp_key_file);
            client
        })
        .await
        .unwrap();

        //When Connection close is initiated
        let result = tokio::task::spawn_blocking(move || client.connection_close_blocking())
            .await
            .unwrap();

        if should_succeed {
            //Then BestEffort: Close succeeds (error suppressed)
            assert!(
                result.is_ok(),
                "BestEffort should suppress refresh failure for {}: {:?}",
                strategy_name,
                result.err()
            );
        } else {
            //Then Strict: Close fails with error
            assert!(
                result.is_err(),
                "Strict should raise refresh failure for {}",
                strategy_name,
            );
        }
    }
}

// ===========================================================================
//                  Retry and Timeout Configuration
// ===========================================================================

#[tokio::test]
async fn should_honor_provided_retry_config_and_succeed_for_each_strategy_type() {
    // Scenario Outline: Examples (strategy_type, max_attempts, failures)
    // strict + 1, best-effort + 3
    for (strategy_name, error_strategy, _max_attempts, num_failures) in [
        ("strict", ErrorStrategy::Strict, 1, 0),
        ("best-effort", ErrorStrategy::BestEffort, 3, 1),
    ] {
        //Given Core logout function called with <strategy_type> strategy
        let _config = LogoutConfig {
            error_strategy,
            ..Default::default()
        };

        //And Retry policy configured with <max_attempts> max attempts
        let expected_attempts = num_failures + 1;
        let retry_policy = RetryPolicy {
            max_attempts: expected_attempts as u32,
            ..Default::default()
        };

        //And Mock HTTP server fails <failures> times then returns 200
        let (addr, attempts, server) =
            spawn_test_server(expected_attempts, move |attempt| async move {
                if attempt <= num_failures {
                    service_unavailable_response(r#"{"success":false}"#, 0)
                } else {
                    json_response(r#"{"success":true}"#)
                }
            })
            .await;

        let server_url = format!("http://{}", addr);
        let client = reqwest::Client::builder().no_proxy().build().unwrap();
        let client_info = test_client_info();

        //When Logout is executed
        let result = logout_session(
            &client,
            &server_url,
            &SensitiveString::from("test_token"),
            &client_info,
            &retry_policy,
            tokio_util::sync::CancellationToken::new(),
        )
        .await;

        //Then Exactly <expected_attempts> attempts are made
        assert_eq!(
            attempts.load(Ordering::SeqCst),
            expected_attempts,
            "Expected {} attempts for {}",
            expected_attempts,
            strategy_name
        );

        //And Close succeeds
        assert!(result.is_ok(), "Should succeed for {}", strategy_name);

        server.await.unwrap();
    }
}

#[tokio::test]
async fn should_honor_provided_timeout_config_and_succeed_for_each_strategy_type() {
    // Scenario Outline: Examples (strategy_type, timeout_seconds, delay_seconds)
    for (strategy_name, error_strategy, timeout_seconds, delay_seconds) in [
        ("strict", ErrorStrategy::Strict, 5, 3),
        ("best-effort", ErrorStrategy::BestEffort, 5, 3),
        ("strict", ErrorStrategy::Strict, 10, 8),
        ("best-effort", ErrorStrategy::BestEffort, 10, 8),
        ("strict", ErrorStrategy::Strict, 15, 13),
        ("best-effort", ErrorStrategy::BestEffort, 15, 13),
        ("strict", ErrorStrategy::Strict, 300, 50),
        ("best-effort", ErrorStrategy::BestEffort, 300, 50),
    ] {
        //Given Core logout function called with <strategy_type> strategy
        let _config = LogoutConfig {
            error_strategy,
            ..Default::default()
        };

        //And Timeout configured to <timeout_seconds> seconds
        let timeout = Duration::from_secs(timeout_seconds);

        //And Retry policy allows the default attempt number
        let retry_policy = RetryPolicy {
            max_elapsed: Some(timeout),
            ..Default::default()
        };

        //And Mock HTTP server delays response by <delay_seconds> seconds then returns 200
        let (addr, _, server) = spawn_test_server(1, move |_| {
            let delay = delay_seconds;
            async move {
                sleep(Duration::from_secs(delay)).await;
                json_response(r#"{"success":true}"#)
            }
        })
        .await;

        let server_url = format!("http://{}", addr);
        let client = reqwest::Client::builder().no_proxy().build().unwrap();
        let client_info = test_client_info();

        //When Logout is executed
        let start = Instant::now();
        let result = logout_session(
            &client,
            &server_url,
            &SensitiveString::from("test_token"),
            &client_info,
            &retry_policy,
            tokio_util::sync::CancellationToken::new(),
        )
        .await;
        let elapsed = start.elapsed();

        //Then Request completes within <timeout_seconds> seconds
        assert!(
            elapsed < Duration::from_secs(timeout_seconds + 2), // +2 buffer
            "Should complete within timeout for {}",
            strategy_name
        );

        //Then Close succeeds
        assert!(result.is_ok(), "Should succeed for {}", strategy_name);

        server.await.unwrap();
    }
}

// ===========================================================================
//                    Error Strategy Tests
// ===========================================================================
//
// TODO: Implement error strategy tests once logout configuration architecture is fixed
//
// The following scenarios require calling the connection layer (connection_close) which
// implements error strategy handling, not the HTTP layer (logout_session) which only
// performs HTTP requests.

// ===========================================================================
//                Connection Layer Error Strategy Tests
// ===========================================================================
// These tests verify error strategy behavior at the connection layer,
// testing connection_close() with different ErrorStrategy configurations.

#[tokio::test]
async fn should_throw_after_exhausted_retries_with_strict_strategy() {
    // Thread-local capturing subscriber — no global conflict with setup_logging()
    let (_guard, log_buf) = capturing_subscriber();

    // Scenario Outline with Examples: max_attempts = 2, 3
    for max_attempts in [2u64, 3] {
        log_buf.lock().unwrap().clear();

        //Given Core logout function called with strict strategy
        let error_strategy = ErrorStrategy::Strict;

        //And Retry policy configured with <max_attempts> max attempts
        let configured_max_attempts = max_attempts as i64;

        //And Mock HTTP server returns 503 on all attempts
        let server = MockServer::start().await;
        mount_jwt_login_success(&server).await;

        Mock::given(wiremock::matchers::method("POST"))
            .and(wiremock::matchers::path("/session"))
            .and(wiremock::matchers::query_param("delete", "true"))
            .respond_with(ResponseTemplate::new(503).set_body_string("Service Unavailable"))
            .expect(max_attempts)
            .mount(&server)
            .await;

        let server_uri = server.uri();
        let client = tokio::task::spawn_blocking(move || {
            let mut client = SnowflakeTestClient::with_int_tests_params(Some(&server_uri));
            client.set_connection_option("authenticator", "SNOWFLAKE_JWT");
            let temp_key_file = private_key_helper::get_test_private_key_file()
                .expect("Failed to create test private key file");
            client
                .set_connection_option("private_key_file", temp_key_file.path().to_str().unwrap());

            client.set_connection_option_bool("server_session_keep_alive", false);
            client.set_logout_error_strategy(error_strategy);
            client.set_connection_option_int("logout_total_timeout_seconds", 30);
            client.set_connection_option_int("logout_max_attempts", configured_max_attempts);

            client.connection_init_blocking().unwrap();

            client.set_temp_key_file(temp_key_file);
            client
        })
        .await
        .unwrap();

        // Propagate capturing subscriber to spawn_blocking thread
        let dispatch = tracing::dispatcher::get_default(|d| d.clone());

        //When Logout is executed
        let result = tokio::task::spawn_blocking(move || {
            let _guard = tracing::dispatcher::set_default(&dispatch);
            client.connection_close_blocking()
        })
        .await
        .unwrap();

        let received_requests = server.received_requests().await.unwrap();
        let logout_count = received_requests
            .iter()
            .filter(|r| is_logout_request(r))
            .count();

        //Then Exactly <max_attempts> attempts are made
        assert_eq!(
            logout_count, max_attempts as usize,
            "Should have made exactly {} logout attempts",
            max_attempts
        );

        //And No further retries after max reached
        assert!(
            logout_count <= max_attempts as usize,
            "Must not exceed max_attempts={}, got {}",
            max_attempts,
            logout_count
        );

        //And Error log is emitted
        let logs = get_captured_logs(log_buf);
        assert!(
            logs.contains("ERROR") && logs.contains("Logout failed"),
            "Expected ERROR log with 'Logout failed' (max_attempts={}).\nCaptured:\n{}",
            max_attempts,
            logs,
        );

        //And Close throws error
        assert!(
            result.is_err(),
            "Close should fail with strict strategy after exhausted retries (max_attempts={})",
            max_attempts
        );
    }
}

#[tokio::test]
async fn should_log_warn_and_succeed_after_exhausted_retries_with_best_effort_strategy() {
    let (_guard, log_buf) = capturing_subscriber();

    // Scenario Outline with Examples: max_attempts = 2, 3
    for max_attempts in [2u64, 3] {
        log_buf.lock().unwrap().clear();

        //Given Core logout function called with best-effort strategy
        let error_strategy = ErrorStrategy::BestEffort;

        //And Retry policy configured with <max_attempts> max attempts
        let configured_max_attempts = max_attempts as i64;

        //And Mock HTTP server returns 503 on all attempts
        let server = MockServer::start().await;
        mount_jwt_login_success(&server).await;

        Mock::given(wiremock::matchers::method("POST"))
            .and(wiremock::matchers::path("/session"))
            .and(wiremock::matchers::query_param("delete", "true"))
            .respond_with(ResponseTemplate::new(503).set_body_string("Service Unavailable"))
            .expect(max_attempts)
            .mount(&server)
            .await;

        let server_uri = server.uri();
        let client = tokio::task::spawn_blocking(move || {
            let mut client = SnowflakeTestClient::with_int_tests_params(Some(&server_uri));
            client.set_connection_option("authenticator", "SNOWFLAKE_JWT");
            let temp_key_file = private_key_helper::get_test_private_key_file()
                .expect("Failed to create test private key file");
            client
                .set_connection_option("private_key_file", temp_key_file.path().to_str().unwrap());

            client.set_connection_option_bool("server_session_keep_alive", false);
            client.set_logout_error_strategy(error_strategy);
            client.set_connection_option_int("logout_total_timeout_seconds", 30);
            client.set_connection_option_int("logout_max_attempts", configured_max_attempts);

            client.connection_init_blocking().unwrap();

            client.set_temp_key_file(temp_key_file);
            client
        })
        .await
        .unwrap();

        // Propagate capturing subscriber to spawn_blocking thread
        let dispatch = tracing::dispatcher::get_default(|d| d.clone());

        //When Logout is executed
        let result = tokio::task::spawn_blocking(move || {
            let _guard = tracing::dispatcher::set_default(&dispatch);
            client.connection_close_blocking()
        })
        .await
        .unwrap();

        let received_requests = server.received_requests().await.unwrap();
        let logout_count = received_requests
            .iter()
            .filter(|r| is_logout_request(r))
            .count();

        //Then Exactly <max_attempts> attempts are made
        assert_eq!(
            logout_count, max_attempts as usize,
            "Should have made exactly {} logout attempts",
            max_attempts
        );

        //And No further retries after max reached
        assert!(
            logout_count <= max_attempts as usize,
            "Must not exceed max_attempts={}, got {}",
            max_attempts,
            logout_count
        );

        //And WARN log is emitted
        let logs = get_captured_logs(log_buf);
        assert!(
            logs.contains("WARN") && logs.contains("Logout failed"),
            "Expected WARN log with 'Logout failed' (max_attempts={}).\nCaptured:\n{}",
            max_attempts,
            logs,
        );

        //And Close succeeds
        assert!(
            result.is_ok(),
            "Close should succeed with best-effort strategy despite failures (max_attempts={}): {:?}",
            max_attempts,
            result.err()
        );
    }
}

#[tokio::test]
async fn should_throw_on_non_retryable_error_code_in_strict_strategy() {
    // Scenario Outline with Examples: error_code = 400, 403, 404, 390114

    let error_cases: Vec<(&str, ResponseTemplate)> = vec![
        //And Mock HTTP server returns <error_code> error
        (
            "400 Bad Request",
            ResponseTemplate::new(400).set_body_string("Bad Request"),
        ),
        (
            "403 Forbidden",
            ResponseTemplate::new(403).set_body_string("Forbidden"),
        ),
        (
            "404 Not Found",
            ResponseTemplate::new(404).set_body_string("Not Found"),
        ),
        (
            "390114 MASTER_TOKEN_EXPIRED",
            ResponseTemplate::new(401)
                .set_body_json(json!({
                    "success": false,
                    "code": "390114",
                    "message": "Master token expired"
                }))
                .insert_header("Content-Type", "application/json"),
        ),
    ];

    for (error_code, response_template) in error_cases {
        //Given Core logout function called with strict strategy
        let server = MockServer::start().await;
        mount_jwt_login_success(&server).await;

        Mock::given(wiremock::matchers::method("POST"))
            .and(wiremock::matchers::path("/session"))
            .and(wiremock::matchers::query_param("delete", "true"))
            .respond_with(response_template)
            .expect(1)
            .mount(&server)
            .await;

        //And UD Core connection is configured and logged in
        let server_uri = server.uri();
        let client = tokio::task::spawn_blocking(move || {
            let mut client = SnowflakeTestClient::with_int_tests_params(Some(&server_uri));
            client.set_connection_option("authenticator", "SNOWFLAKE_JWT");
            let temp_key_file = private_key_helper::get_test_private_key_file()
                .expect("Failed to create test private key file");
            client
                .set_connection_option("private_key_file", temp_key_file.path().to_str().unwrap());

            client.set_connection_option_bool("server_session_keep_alive", false);
            client.set_logout_error_strategy(ErrorStrategy::Strict);
            client.set_connection_option_int("logout_total_timeout_seconds", 30);
            client.set_connection_option_int("logout_max_attempts", 3);

            client.connection_init_blocking().unwrap();

            client.set_temp_key_file(temp_key_file);
            client
        })
        .await
        .unwrap();

        //When Logout is executed
        let result = tokio::task::spawn_blocking(move || client.connection_close_blocking())
            .await
            .unwrap();

        //Then Close throws error immediately
        assert!(
            result.is_err(),
            "Close should fail with strict strategy for non-retryable error {}: {:?}",
            error_code,
            result.ok()
        );

        let received_requests = server.received_requests().await.unwrap();
        let logout_count = received_requests
            .iter()
            .filter(|r| is_logout_request(r))
            .count();

        //And Error is surfaced to caller
        let error = result.unwrap_err();
        assert!(
            !format!("{error:?}").is_empty(),
            "Error should be surfaced to caller for non-retryable error {}",
            error_code
        );

        //And No retries are attempted
        assert_eq!(
            logout_count, 1,
            "Should have made exactly 1 logout attempt (no retries for non-retryable error {})",
            error_code
        );
    }
}

#[tokio::test]
async fn should_log_and_suppress_non_retryable_error_code_in_best_effort_strategy() {
    let (_guard, log_buf) = capturing_subscriber();

    // Scenario Outline with Examples: error_code = 400, 403, 404, 390114

    let error_cases: Vec<(&str, ResponseTemplate)> = vec![
        //And Mock HTTP server returns <error_code> error
        (
            "400 Bad Request",
            ResponseTemplate::new(400).set_body_string("Bad Request"),
        ),
        (
            "403 Forbidden",
            ResponseTemplate::new(403).set_body_string("Forbidden"),
        ),
        (
            "404 Not Found",
            ResponseTemplate::new(404).set_body_string("Not Found"),
        ),
        (
            "390114 MASTER_TOKEN_EXPIRED",
            ResponseTemplate::new(401)
                .set_body_json(json!({
                    "success": false,
                    "code": "390114",
                    "message": "Master token expired"
                }))
                .insert_header("Content-Type", "application/json"),
        ),
    ];

    for (error_code, response_template) in error_cases {
        log_buf.lock().unwrap().clear();

        //Given Core logout function called with best-effort strategy
        let server = MockServer::start().await;
        mount_jwt_login_success(&server).await;

        Mock::given(wiremock::matchers::method("POST"))
            .and(wiremock::matchers::path("/session"))
            .and(wiremock::matchers::query_param("delete", "true"))
            .respond_with(response_template)
            .expect(1)
            .mount(&server)
            .await;

        //And UD Core connection is configured and logged in
        let server_uri = server.uri();
        let client = tokio::task::spawn_blocking(move || {
            let mut client = SnowflakeTestClient::with_int_tests_params(Some(&server_uri));
            client.set_connection_option("authenticator", "SNOWFLAKE_JWT");
            let temp_key_file = private_key_helper::get_test_private_key_file()
                .expect("Failed to create test private key file");
            client
                .set_connection_option("private_key_file", temp_key_file.path().to_str().unwrap());

            client.set_connection_option_bool("server_session_keep_alive", false);
            client.set_logout_error_strategy(ErrorStrategy::BestEffort);
            client.set_connection_option_int("logout_total_timeout_seconds", 30);
            client.set_connection_option_int("logout_max_attempts", 3);

            client.connection_init_blocking().unwrap();

            client.set_temp_key_file(temp_key_file);
            client
        })
        .await
        .unwrap();

        // Propagate capturing subscriber to spawn_blocking thread
        let dispatch = tracing::dispatcher::get_default(|d| d.clone());

        //When Logout is executed
        let result = tokio::task::spawn_blocking(move || {
            let _guard = tracing::dispatcher::set_default(&dispatch);
            client.connection_close_blocking()
        })
        .await
        .unwrap();

        let received_requests = server.received_requests().await.unwrap();
        let logout_count = received_requests
            .iter()
            .filter(|r| is_logout_request(r))
            .count();

        //Then Error is logged as WARN
        let logs = get_captured_logs(log_buf);
        assert!(
            logs.contains("WARN") && logs.contains("Logout failed"),
            "Expected WARN log with 'Logout failed' for non-retryable error {}.\nCaptured:\n{}",
            error_code,
            logs,
        );

        //And Close succeeds without throwing
        assert!(
            result.is_ok(),
            "Close should succeed with best-effort strategy despite non-retryable error {}: {:?}",
            error_code,
            result.err()
        );

        //And No retries are attempted
        assert_eq!(
            logout_count, 1,
            "Should have made exactly 1 logout attempt (no retries for non-retryable error {})",
            error_code
        );
    }
}

// ===========================================================================
//                      Timeout Failure Scenarios
// ===========================================================================

#[tokio::test]
async fn should_throw_on_timeout_with_strict_strategy() {
    // Scenario Outline: Examples (timeout_seconds=3, delay_seconds=5)
    //Given Core logout function called with strict strategy
    let _error_strategy = ErrorStrategy::Strict;

    //And Timeout configured to <timeout_seconds> seconds
    let timeout = Duration::from_secs(3);

    //And Mock HTTP server delays response by <delay_seconds> seconds
    let delay = Duration::from_secs(5);

    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();

    let server = tokio::spawn(async move {
        let (mut stream, _) = listener.accept().await.unwrap();
        let mut buf = vec![0u8; 4096];
        let _ = stream.read(&mut buf).await;
        // Delay longer than timeout
        sleep(delay).await;
        // Client will have timed out - don't send response
    });

    let server_url = format!("http://{}", addr);
    let client = reqwest::Client::builder().no_proxy().build().unwrap();
    let client_info = test_client_info();

    let _config = LogoutConfig {
        error_strategy: ErrorStrategy::Strict,
        logout_total_timeout: timeout,
        ..Default::default()
    };

    let retry_policy = RetryPolicy {
        max_attempts: 1,
        max_elapsed: Some(timeout),
        per_request_timeout: Some(timeout),
        ..Default::default()
    };

    //When Logout is executed
    let start = Instant::now();
    let result = logout_session(
        &client,
        &server_url,
        &SensitiveString::from("test_token"),
        &client_info,
        &retry_policy,
        tokio_util::sync::CancellationToken::new(),
    )
    .await;
    let elapsed = start.elapsed();

    //Then Request times out after <timeout_seconds> seconds
    assert!(
        elapsed >= timeout && elapsed < timeout + Duration::from_secs(2),
        "Should timeout after ~{:?}, took {:?}",
        timeout,
        elapsed
    );

    //Then Close throws timeout error
    assert!(result.is_err(), "Strict strategy should fail on timeout");

    let error_msg = format!("{:?}", result.unwrap_err());
    assert!(
        error_msg.contains("TimedOut")
            || error_msg.contains("timeout")
            || error_msg.contains("timed out")
            || error_msg.contains("Timeout")
            || error_msg.contains("deadline"),
        "Error should be timeout-related, got: {}",
        error_msg
    );

    server.abort();
}

#[tokio::test]
async fn should_log_warn_and_succeed_on_timeout_with_best_effort_strategy() {
    // Scenario Outline: Examples (timeout_seconds=3, delay_seconds=5)
    //Given Core logout function called with best-effort strategy
    let config = LogoutConfig {
        error_strategy: ErrorStrategy::BestEffort,
        ..Default::default()
    };

    //And Timeout configured to <timeout_seconds> seconds
    let timeout = Duration::from_secs(3);

    //And Mock HTTP server delays response by <delay_seconds> seconds
    let delay = Duration::from_secs(5);

    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();

    let server = tokio::spawn(async move {
        let (mut stream, _) = listener.accept().await.unwrap();
        let mut buf = vec![0u8; 4096];
        let _ = stream.read(&mut buf).await;
        // Delay longer than timeout
        sleep(delay).await;
        // Client will have timed out
    });

    let server_url = format!("http://{}", addr);
    let client = reqwest::Client::builder().no_proxy().build().unwrap();
    let client_info = test_client_info();

    let retry_policy = RetryPolicy {
        max_attempts: 1,
        max_elapsed: Some(timeout),
        per_request_timeout: Some(timeout),
        ..Default::default()
    };

    //When Logout is executed
    let start = Instant::now();
    let result = logout_session(
        &client,
        &server_url,
        &SensitiveString::from("test_token"),
        &client_info,
        &retry_policy,
        tokio_util::sync::CancellationToken::new(),
    )
    .await;
    let elapsed = start.elapsed();

    //Then Request times out after <timeout_seconds> seconds
    assert!(
        elapsed >= timeout && elapsed < timeout + Duration::from_secs(2),
        "Should timeout after ~{:?}, took {:?}",
        timeout,
        elapsed
    );

    //And Timeout is logged as WARN
    let api_result = result.map_err(|e| sf_core::apis::database_driver_v1::ApiError::Logout {
        message: format!("{e}"),
        location: snafu::Location::default(),
    });
    let handled_result = config.error_strategy.handle_failed_logout(api_result);

    //Then Close succeeds
    assert!(
        handled_result.is_ok(),
        "BestEffort should succeed despite timeout, raw result: {:?}",
        handled_result
    );

    server.abort();
}

// ===========================================================================
//                    Post-Logout Session Invalidation
// ===========================================================================

#[tokio::test]
async fn should_reject_queries_client_side_after_connection_is_closed() {
    //Given Mock HTTP server is configured
    let server = MockServer::start().await;
    mount_jwt_login_success(&server).await;

    // Mount logout endpoint mock
    Mock::given(method("POST"))
        .and(path("/session"))
        .and(query_param("delete", "true"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_json(json!({ "success": true }))
                .insert_header("Content-Type", "application/json"),
        )
        .mount(&server)
        .await;

    //And UD Core connection is logged in
    let server_uri = server.uri();
    let client = tokio::task::spawn_blocking(move || {
        SnowflakeTestClient::connect_integration_test(Some(&server_uri))
    })
    .await
    .unwrap();

    //When Connection is closed
    let (close_result, client) = tokio::task::spawn_blocking(move || {
        let result = client.connection_close_blocking();
        (result, client)
    })
    .await
    .unwrap();
    assert!(close_result.is_ok(), "Connection close should succeed");

    //And Query is attempted on closed connection
    let result_after =
        tokio::task::spawn_blocking(move || client.execute_query_no_unwrap("SELECT 1"))
            .await
            .unwrap();

    //Then Query fails with connection closed error
    let error_msg = result_after.expect_err("Query should fail after connection is closed");
    assert!(
        error_msg.contains("closed"),
        "Error must mention connection is closed, got: {error_msg}",
    );
}

// Helper functions

fn test_client_info() -> ClientInfo {
    ClientInfo {
        client_app_id: "TestApp".to_string(),
        application: "TestApp".to_string(),
        version: "1.0.0".to_string(),
        os: "TestOS".to_string(),
        os_version: "1.0".to_string(),
        ocsp_mode: Some("FAIL_OPEN".to_string()),
        crl_config: Default::default(),
        tls_config: Default::default(),
        proxy_config: Default::default(),
        platforms: vec![],
        os_details: None,
        compiler: None,
        runtime_name: None,
        runtime_version: None,
    }
}
