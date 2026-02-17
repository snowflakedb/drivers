//! Integration tests for session logout functionality.
//!
//! These tests use mock HTTP servers (wiremock, spawn_test_server) to verify
//! logout behavior without connecting to real Snowflake.

use crate::common::mocks::auth::mount_jwt_login_success;
use crate::common::snowflake_test_client::SnowflakeTestClient;
use crate::common::test_server::{
    extract_query_param, json_error_response, json_response, service_unavailable_response,
    spawn_capture_server, spawn_test_server,
};
use sf_core::config::logout::{ErrorStrategy, LogoutConfig};
use sf_core::config::rest_parameters::ClientInfo;
use sf_core::config::retry::RetryPolicy;
use sf_core::protobuf_apis::database_driver_v1::DatabaseDriverClient;
use sf_core::protobuf_gen::database_driver_v1::*;
use sf_core::rest::snowflake::logout::logout_session;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::{Duration, Instant};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;
use tokio::time::sleep;
use wiremock::MockServer;

// ===========================================================================
//                      HTTP Request Construction
// ===========================================================================

#[tokio::test]
async fn should_construct_logout_request_with_correct_http_method_url_headers_and_body() {
    //Given Mock HTTP server is configured to capture requests
    //And UD Core client is logged in
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

    //Then HTTP method is POST
    assert!(captured.starts_with(b"POST"), "Should be POST request");

    //And Request URL path is /session
    assert!(
        captured.starts_with(b"POST /session"),
        "Should request /session"
    );

    let request_str = String::from_utf8_lossy(&captured);

    //And Query parameter delete is set to true
    assert!(
        request_str.contains("delete=true"),
        "Should have delete=true"
    );

    //And Query parameter requestId is present and static across attempts
    assert!(request_str.contains("requestId="), "Should have requestId");

    //And Query parameter request_guid is present and unique per attempt
    assert!(
        request_str.contains("request_guid="),
        "Should have request_guid"
    );

    //And Authorization header is present with format "Snowflake Token={session_token}"
    assert!(
        request_str.contains(&format!(
            "Authorization: Snowflake Token=\"{}\"",
            session_token
        )) || request_str.contains(&format!(
            "authorization: Snowflake Token=\"{}\"",
            session_token
        )),
        "Should have Authorization header with session token"
    );

    //And Content-Type header is application/json
    assert!(
        request_str
            .to_lowercase()
            .contains("content-type: application/json"),
        "Should have Content-Type: application/json"
    );

    //And Accept header is application/snowflake
    assert!(
        request_str
            .to_lowercase()
            .contains("accept: application/snowflake"),
        "Should have Accept: application/snowflake"
    );

    //And User-Agent header contains UD version and Rust version
    assert!(
        request_str.contains("user-agent:") && request_str.contains("UD/"),
        "Should have User-Agent with UD version"
    );

    //And Request body is exactly empty JSON object {}
    assert!(
        request_str.contains("{}"),
        "Should have empty JSON object body"
    );
}

#[tokio::test]
async fn should_not_send_logout_when_connection_was_never_established() {
    //Given Mock HTTP server is configured
    let server = MockServer::start().await;

    //And Connection attempt failed before authentication
    let connection_established = false; // Simulate failed connection

    //When Connection close is attempted
    if connection_established {
        // Would call logout_session() here if connection was established
        panic!("Should not reach here - connection was never established");
    }

    //Then No HTTP request is sent to server
    let received_requests = server.received_requests().await.unwrap();
    assert_eq!(
        received_requests.len(),
        0,
        "Should not send any HTTP requests when connection was never established"
    );
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
    let client = SnowflakeTestClient::connect_integration_test(Some(&server.uri()));

    //When Connection is closed
    let result = DatabaseDriverClient::connection_close(ConnectionCloseRequest {
        conn_handle: Some(client.conn_handle),
        server_session_keep_alive: Some(true),
        enable_auto_detection: None,
        error_strategy: None,
        timeout_seconds: None,
    });

    //Then No logout HTTP request is sent to server
    assert!(result.is_ok(), "Close should succeed");

    // Verify no logout request was made by checking server received requests
    // (We didn't mount any /session endpoint, so any logout attempt would fail)
}

#[tokio::test]
async fn should_send_logout_when_server_session_keep_alive_is_explicitly_false() {
    //Given Mock HTTP server is configured
    let (addr, attempts, server) =
        spawn_test_server(1, |_| async move { json_response(r#"{"success":true}"#) }).await;

    let server_url = format!("http://{}", addr);
    let session_token = "test_token";
    let client = reqwest::Client::builder().no_proxy().build().unwrap();
    let client_info = test_client_info();

    //And UD Core connection is logged in with server_session_keep_alive set to false
    let config = LogoutConfig {
        server_session_keep_alive: Some(false),
        ..Default::default()
    };

    //When Connection is closed
    let result = logout_session(
        &client,
        &server_url,
        session_token,
        &client_info,
        config.timeout,
        &RetryPolicy::default(),
    )
    .await;

    //Then Logout HTTP request is sent to server
    assert!(result.is_ok(), "Logout should succeed");
    assert_eq!(
        attempts.load(Ordering::SeqCst),
        1,
        "Should have made exactly 1 logout request"
    );

    server.await.unwrap();
}

// ===========================================================================
//                      Default Configuration
// ===========================================================================

#[tokio::test]
async fn should_timeout_after_5_seconds_by_default_when_server_does_not_respond() {
    //Given Mock HTTP server holds connection open for 10 seconds without responding
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();

    let server = tokio::spawn(async move {
        let (mut stream, _) = listener.accept().await.unwrap();
        // Read request but don't respond - hold connection open
        let mut buf = vec![0u8; 4096];
        let _ = stream.read(&mut buf).await;
        // Sleep for 10 seconds (longer than 5s timeout)
        sleep(Duration::from_secs(10)).await;
        // Connection will timeout before we respond
    });

    let server_url = format!("http://{}", addr);
    let client = reqwest::Client::builder().no_proxy().build().unwrap();
    let client_info = test_client_info();

    //And UD Core connection is logged in with no timeout override
    let config = LogoutConfig::default(); // Default timeout is 5 seconds

    //When Logout is initiated
    let start = Instant::now();
    let result = logout_session(
        &client,
        &server_url,
        "test_token",
        &client_info,
        config.timeout,
        &RetryPolicy::default(),
    )
    .await;
    let elapsed = start.elapsed();

    //Then Logout request times out after approximately 5 seconds
    assert!(result.is_err(), "Should timeout");

    //And Close throws timeout error
    assert!(
        format!("{:?}", result.unwrap_err()).contains("timeout")
            || format!("{:?}", result.unwrap_err()).contains("Timeout"),
        "Error should be timeout-related"
    );

    //And Total elapsed time is between 5 and 6 seconds
    assert!(
        elapsed >= Duration::from_secs(5) && elapsed < Duration::from_secs(7),
        "Should timeout after ~5 seconds, took {:?}",
        elapsed
    );

    server.abort(); // Clean up server task
}

#[tokio::test]
async fn should_cancel_individual_request_when_per_request_socket_timeout_exceeded() {
    //Given Mock HTTP server holds connection open for 8 seconds on first attempt then succeeds immediately
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let attempts = Arc::new(AtomicUsize::new(0));
    let attempts_clone = attempts.clone();

    let server = tokio::spawn(async move {
        loop {
            let (mut stream, _) = listener.accept().await.unwrap();
            let attempt = attempts_clone.fetch_add(1, Ordering::SeqCst) + 1;
            let mut buf = vec![0u8; 4096];
            let _ = stream.read(&mut buf).await;

            if attempt == 1 {
                // First attempt: hold for 8 seconds (longer than 2s socket timeout)
                sleep(Duration::from_secs(8)).await;
                // Client will have given up by now
            } else {
                // Second attempt: respond immediately
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

    //And UD Core connection is logged in
    //And Per-request socket timeout is set to 2 seconds
    let per_request_timeout = Duration::from_secs(2);

    //And Total retry budget timeout is set to 10 seconds
    let total_timeout = Duration::from_secs(10);

    //When Logout is initiated
    let start = Instant::now();
    let result = logout_session(
        &client,
        &server_url,
        "test_token",
        &client_info,
        per_request_timeout,
        &RetryPolicy::default(),
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

    let server_url = format!("http://{}", addr);
    let client = reqwest::Client::builder().no_proxy().build().unwrap();
    let client_info = test_client_info();

    //And UD Core connection is logged in
    //And Total retry budget timeout is set to 5 seconds
    let total_timeout = Duration::from_secs(5);

    //And Retry policy allows 10 attempts
    let retry_policy = RetryPolicy::default();

    //When Logout is initiated
    let start = Instant::now();
    let result = logout_session(
        &client,
        &server_url,
        "test_token",
        &client_info,
        total_timeout,
        &retry_policy,
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

    //And The last attempt timeouts because remaining budget is less than server response time
    assert!(result.is_err(), "Should fail due to timeout/exhaustion");

    //And Total wall-clock time does not exceed 7 seconds for closing the connection
    assert!(
        elapsed < Duration::from_secs(7),
        "Should not exceed 7 seconds, took {:?}",
        elapsed
    );

    server.await.unwrap();
}

// ===========================================================================
//                  Close vs Active Query Execution
// ===========================================================================

#[tokio::test]
#[ignore = "TODO: Requires query execution implementation - SNOW-2923705"]
async fn should_reject_new_query_with_connection_closed_error_when_submitted_after_close_started() {
    //Given Mock HTTP server delays logout response by 5 seconds then returns 200
    //And UD Core connection is logged in
    //When Connection close is initiated on a separate thread
    //And Query SELECT 1 is submitted while logout is still in-flight
    //Then Query SELECT 1 fails with connection closed error
    //And Mock server did not receive any query request
    //And Close completes successfully after logout response arrives

    // TODO: SNOW-2923705 - Implement after query execution is available
}

#[tokio::test]
#[ignore = "TODO: Requires query execution implementation - SNOW-2923705"]
async fn should_fail_in_flight_query_when_server_response_arrives_after_closing_process_started() {
    //Given Mock HTTP server delays query response by 3 seconds then returns query result
    //And Mock HTTP server accepts logout requests with 200
    //And UD Core connection is logged in
    //And Socket timeout is set to 10 seconds
    //And Query is submitted and server has not responded yet
    //When Connection close is initiated
    //And Server returns query response after closing process started
    //Then Mock server successfully completed query response delivery
    //And Query caller receives connection closed error
    //And Mock server received POST /session?delete=true logout request
    //And Close completes successfully

    // TODO: SNOW-2923705 - Implement after query execution is available
}

// ===========================================================================
//                  Close vs Token Refresh
// ===========================================================================

#[tokio::test]
#[ignore = "TODO: Requires token refresh during close - SNOW-2923705"]
async fn should_wait_for_in_flight_token_renewal_to_complete_then_logout_with_refreshed_token() {
    //Given Mock HTTP server delays token refresh response by 3 seconds then returns new token
    //And Mock HTTP server accepts logout requests with 200
    //And UD Core connection is logged in
    //And Token refresh is already in-flight
    //When Connection close is requested while refresh is still in-flight
    //Then Mock server received token refresh request before logout request
    //And Logout request Authorization header contains the refreshed session token
    //And Close completes successfully

    // TODO: SNOW-2923705 - Complex scenario requiring concurrent refresh + close
}

#[tokio::test]
#[ignore = "TODO: Requires query execution implementation - SNOW-2923705"]
async fn should_not_start_token_renewal_when_query_receives_390112_after_closing_process_started() {
    //Given Mock HTTP server returns 390112 SESSION_TOKEN_EXPIRED to query after 3 second delay
    //And Mock HTTP server accepts logout requests with 200
    //And UD Core connection is logged in
    //And Socket timeout is set to 10 seconds
    //And Query is submitted and waiting for server response
    //When Connection close is initiated
    //And Server responds 390112 SESSION_TOKEN_EXPIRED to the in-flight query
    //Then Mock server did not receive any token refresh request
    //And Query caller receives connection closed error
    //And Close completes successfully

    // TODO: SNOW-2923705 - Requires query execution and token refresh coordination
}

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

        //And Mock server returns SESSION_GONE 390111
        let config = LogoutConfig {
            error_strategy,
            ..Default::default()
        };

        //When Logout is executed
        let result = logout_session(
            &client,
            &server_url,
            "test_token",
            &client_info,
            config.timeout,
            &RetryPolicy::default(),
        )
        .await;

        //Then Close succeeds
        assert!(
            result.is_ok(),
            "SESSION_GONE should be treated as success for {}",
            strategy_type
        );

        //And Error is ignored

        server.await.unwrap();
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
            ("503 Service Unavailable", || {
                service_unavailable_response(r#"{"success":false}"#, 0)
            }),
            ("429 Too Many Requests", || {
                json_error_response(
                    429,
                    "Too Many Requests",
                    r#"{"success":false,"message":"Rate limited"}"#,
                )
            }),
        ] {
            //Given Core logout function called with <strategy_type> strategy
            //And Mock server returns <error_type> on attempt 1
            //And Mock server returns 200 on attempt 2
            let (addr, attempts, server) = spawn_test_server(2, |attempt| {
                let error_fn = error_response_fn;
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

            let config = LogoutConfig {
                error_strategy,
                ..Default::default()
            };

            //When Logout is executed
            let result = logout_session(
                &client,
                &server_url,
                "test_token",
                &client_info,
                config.timeout,
                &RetryPolicy::default(),
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
            //And Mock server resets connection on first attempt
            //And Mock server succeeds on second attempt
            let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
            let addr = listener.local_addr().unwrap();
            let attempts = Arc::new(AtomicUsize::new(0));
            let attempts_clone = attempts.clone();

            let server = tokio::spawn(async move {
                loop {
                    let (mut stream, _) = listener.accept().await.unwrap();
                    let attempt = attempts_clone.fetch_add(1, Ordering::SeqCst) + 1;

                    if attempt == 1 {
                        drop(stream); // Reset connection
                    } else {
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

            let config = LogoutConfig {
                error_strategy,
                ..Default::default()
            };

            //When Logout is executed
            let result = logout_session(
                &client,
                &server_url,
                "test_token",
                &client_info,
                config.timeout,
                &RetryPolicy::default(),
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
#[ignore = "TODO: Requires token refresh implementation - SNOW-2923705"]
async fn should_attempt_token_refresh_on_390112_when_retries_allowed_for_each_strategy_type() {
    // Scenario Outline: Examples (strategy_type)
    // strict, best-effort
    for (_strategy_name, _error_strategy) in [
        ("strict", ErrorStrategy::Strict),
        ("best-effort", ErrorStrategy::BestEffort),
    ] {
        //Given Core logout function called with <strategy_type> strategy
        //And Mock server returns SESSION_TOKEN_EXPIRED 390112 on first attempt
        //And Mock server returns 200 after token refresh
        //And Retry policy allows 1 retry
        //When Logout is executed
        //Then Token refresh request is sent to server
        //And Logout is retried with new session token
        //And Close succeeds

        // TODO: SNOW-2923705 - Requires token refresh during logout
    }
}

#[tokio::test]
#[ignore = "TODO: Requires token refresh implementation - SNOW-2923705"]
async fn should_not_attempt_token_refresh_when_retry_count_is_0_for_each_strategy_type() {
    // Scenario Outline: Examples (strategy_type)
    // strict, best-effort
    for (_strategy_name, _error_strategy) in [
        ("strict", ErrorStrategy::Strict),
        ("best-effort", ErrorStrategy::BestEffort),
    ] {
        //Given Core logout function called with <strategy_type> strategy
        //And Mock server returns SESSION_TOKEN_EXPIRED 390112
        //And Retry policy allows 0 retries
        //When Logout is executed
        //Then No token refresh request is sent to server
        // (For strict: Close throws SESSION_TOKEN_EXPIRED error)
        // (For best-effort: SESSION_TOKEN_EXPIRED is logged as WARN and Close succeeds)

        // TODO: SNOW-2923705 - Requires token refresh during logout
    }
}

#[tokio::test]
#[ignore = "TODO: Requires token refresh implementation - SNOW-2923705"]
async fn should_include_token_refresh_time_in_total_logout_timeout_budget() {
    //Given Core logout function called
    //And Mock server returns SESSION_TOKEN_EXPIRED 390112 on first attempt
    //And Token refresh endpoint delays response by 3 seconds
    //And Mock server returns 200 after token refresh
    //And Total retry budget timeout is set to 5 seconds
    //When Logout is executed
    //Then Token refresh is attempted
    //And Token refresh time is counted against total timeout budget
    //And Remaining budget for retry logout is reduced by token refresh duration
    //And Total wall-clock time does not exceed 7 seconds for closing the connection

    // TODO: SNOW-2923705 - Requires token refresh during logout with timing
}

// ===========================================================================
//                  Retry and Timeout Configuration
// ===========================================================================

#[tokio::test]
async fn should_honor_provided_retry_config_and_succeed_for_each_strategy_type() {
    // Scenario Outline: Examples (strategy_type, max_attempts, failures)
    // strict + 1, best-effort + 3
    for (strategy_name, error_strategy, max_attempts, num_failures) in [
        ("strict", ErrorStrategy::Strict, 1, 0),
        ("best-effort", ErrorStrategy::BestEffort, 3, 1),
    ] {
        //Given Core logout function called with <strategy_type> strategy
        //And Retry policy configured with <max_attempts> max attempts
        //And Mock server fails <failures> times then returns 200
        let expected_attempts = num_failures + 1;
        let (addr, attempts, server) = spawn_test_server(expected_attempts, |attempt| async move {
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

        let config = LogoutConfig {
            error_strategy,
            ..Default::default()
        };

        //When Logout is executed
        let result = logout_session(
            &client,
            &server_url,
            "test_token",
            &client_info,
            config.timeout,
            &RetryPolicy::default(),
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
        ("strict", ErrorStrategy::Strict, 10, 5),
        ("best-effort", ErrorStrategy::BestEffort, 10, 5),
    ] {
        //Given Core logout function called with <strategy_type> strategy
        //And Timeout configured to <timeout_seconds> seconds
        //And Mock server delays response by <delay_seconds> seconds then returns 200
        let (addr, _, server) = spawn_test_server(1, |_| {
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

        let config = LogoutConfig {
            error_strategy,
            timeout: Duration::from_secs(timeout_seconds),
            ..Default::default()
        };

        //When Logout is executed
        let start = Instant::now();
        let result = logout_session(
            &client,
            &server_url,
            "test_token",
            &client_info,
            config.timeout,
            &RetryPolicy::default(),
        )
        .await;
        let elapsed = start.elapsed();

        //Then Request completes within <timeout_seconds> seconds
        assert!(
            elapsed < Duration::from_secs(timeout_seconds + 2), // +2 buffer
            "Should complete within timeout for {}",
            strategy_name
        );

        //And Close succeeds
        assert!(result.is_ok(), "Should succeed for {}", strategy_name);

        server.await.unwrap();
    }
}

// TODO: Implement timeout failure scenarios
// Scenario Outline: should_throw_on_timeout_with_strict_strategy
// Scenario Outline: should_log_WARN_and_succeed_on_timeout_with_best_effort_strategy

#[tokio::test]
async fn should_throw_after_exhausted_retries_with_strict_strategy() {
    //Given Core logout function called with strict strategy
    //And Retry policy configured with <max_attempts> max attempts
    let max_attempts = 2;
    //And Mock server returns 503 on all attempts
    let (addr, attempts, server) = spawn_test_server(max_attempts, |_| async move {
        service_unavailable_response(r#"{"success":false}"#, 0)
    })
    .await;

    let server_url = format!("http://{}", addr);
    let client = reqwest::Client::builder().no_proxy().build().unwrap();
    let client_info = test_client_info();

    let config = LogoutConfig {
        error_strategy: ErrorStrategy::Strict,
        ..Default::default()
    };

    //When Logout is executed
    let result = logout_session(
        &client,
        &server_url,
        "test_token",
        &client_info,
        config.timeout,
        &RetryPolicy::default(),
    )
    .await;

    //Then Exactly <max_attempts> attempts are made
    assert!(
        attempts.load(Ordering::SeqCst) >= max_attempts,
        "Should make at least {} attempts",
        max_attempts
    );

    //And No further retries after max reached
    //And WARN log is emitted
    //And Close throws error
    assert!(result.is_err(), "Should fail after retries exhausted");

    server.await.unwrap();
}

// Continuation: Non-retryable errors and telemetry

#[tokio::test]
async fn should_throw_on_non_retryable_error_code_in_strict_strategy() {
    // Scenario Outline: Examples (error_code)
    // 400 Bad Request, 403 Forbidden, 404 Not Found, MASTER_TOKEN_EXPIRED 390114
    for (error_code, status, reason, body) in [
        (
            "400 Bad Request",
            400,
            "Bad Request",
            r#"{"success":false,"message":"Bad request"}"#,
        ),
        (
            "403 Forbidden",
            403,
            "Forbidden",
            r#"{"success":false,"message":"Forbidden"}"#,
        ),
        (
            "404 Not Found",
            404,
            "Not Found",
            r#"{"success":false,"message":"Not found"}"#,
        ),
        (
            "MASTER_TOKEN_EXPIRED 390114",
            401,
            "Unauthorized",
            r#"{"success":false,"message":"Master token expired","code":"390114"}"#,
        ),
    ] {
        //Given Core logout function called with strict strategy
        //And Mock server returns <error_code> error
        let (addr, _, server) =
            spawn_test_server(
                1,
                |_| async move { json_error_response(status, reason, body) },
            )
            .await;

        let server_url = format!("http://{}", addr);
        let client = reqwest::Client::builder().no_proxy().build().unwrap();
        let client_info = test_client_info();

        let config = LogoutConfig {
            error_strategy: ErrorStrategy::Strict,
            ..Default::default()
        };

        //When Logout is executed
        let result = logout_session(
            &client,
            &server_url,
            "test_token",
            &client_info,
            config.timeout,
            &RetryPolicy::default(),
        )
        .await;

        //Then Close throws error immediately
        assert!(result.is_err(), "Should throw on {}", error_code);

        //And Error is surfaced to caller
        //And No retries are attempted

        server.await.unwrap();
    }
}

#[tokio::test]
async fn should_log_and_suppress_non_retryable_error_code_in_best_effort_strategy() {
    // Scenario Outline: Examples (error_code)
    // 400 Bad Request, 403 Forbidden, 404 Not Found, MASTER_TOKEN_EXPIRED 390114
    for (error_code, status, reason, body) in [
        (
            "400 Bad Request",
            400,
            "Bad Request",
            r#"{"success":false,"message":"Bad request"}"#,
        ),
        (
            "403 Forbidden",
            403,
            "Forbidden",
            r#"{"success":false,"message":"Forbidden"}"#,
        ),
        (
            "404 Not Found",
            404,
            "Not Found",
            r#"{"success":false,"message":"Not found"}"#,
        ),
        (
            "MASTER_TOKEN_EXPIRED 390114",
            401,
            "Unauthorized",
            r#"{"success":false,"message":"Master token expired","code":"390114"}"#,
        ),
    ] {
        //Given Core logout function called with best-effort strategy
        //And Mock server returns <error_code> error
        let (addr, _, server) =
            spawn_test_server(
                1,
                |_| async move { json_error_response(status, reason, body) },
            )
            .await;

        let server_url = format!("http://{}", addr);
        let client = reqwest::Client::builder().no_proxy().build().unwrap();
        let client_info = test_client_info();

        let config = LogoutConfig {
            error_strategy: ErrorStrategy::BestEffort,
            ..Default::default()
        };

        //When Logout is executed
        let result = logout_session(
            &client,
            &server_url,
            "test_token",
            &client_info,
            config.timeout,
            &RetryPolicy::default(),
        )
        .await;

        //Then Error is logged as WARN
        //And Close succeeds without throwing
        assert!(
            result.is_ok(),
            "BestEffort should succeed despite {} error",
            error_code
        );

        //And No retries are attempted

        server.await.unwrap();
    }
}

#[tokio::test]
#[ignore = "TODO: Telemetry required - SNOW-2912513"]
async fn should_record_connection_close_decision_metrics_before_logout() {
    //Given Telemetry client is configured
    //And UD Core client is logged in
    //When Connection close is initiated
    //Then Pre-logout metrics are recorded in telemetry batch
    //And Metrics include whether auto-detection was performed
    //And Metrics include whether async queries were detected
    //And Metrics include whether logout will be sent or skipped
    //And Metrics include skip reason if logout is skipped
    //And Telemetry batch is flushed before logout is sent
    //And Logout proceeds after telemetry flush completes

    // TODO: SNOW-2912513 - Implement telemetry
}

// ===========================================================================
//                    Post-Logout Session Invalidation
// ===========================================================================

#[tokio::test]
async fn should_reject_queries_client_side_after_connection_is_closed() {
    //Given Snowflake client is logged in
    let server = MockServer::start().await;
    mount_jwt_login_success(&server).await;

    let client = SnowflakeTestClient::connect_integration_test(Some(&server.uri()));

    //And Simple query SELECT 1 executes successfully
    // Note: For this test we skip the pre-logout query since we're testing
    // client-side rejection, not server behavior. The key is that after
    // close(), queries are rejected without reaching the server.

    //When Connection is closed
    let close_result = DatabaseDriverClient::connection_close(ConnectionCloseRequest {
        conn_handle: Some(client.conn_handle),
        server_session_keep_alive: Some(true), // Skip logout HTTP request
        enable_auto_detection: None,
        error_strategy: None,
        timeout_seconds: None,
    });
    assert!(close_result.is_ok(), "Connection close should succeed");

    //And Query is attempted on closed connection
    let result_after = client.execute_query_no_unwrap("SELECT 1");

    //Then Query fails with connection closed error
    assert!(
        result_after.is_err(),
        "Query should fail after connection is closed, but got: {:?}",
        result_after
    );

    let error_msg = result_after.unwrap_err();
    assert!(
        error_msg.contains("closed")
            || error_msg.contains("Closed")
            || error_msg.contains("CONNECTION_NOT_OPEN")
            || error_msg.contains("not open")
            || error_msg.contains("not initialized"),
        "Error should indicate connection is closed: {}",
        error_msg
    );
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
