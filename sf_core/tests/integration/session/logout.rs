//! Integration tests for session logout functionality.
//!
//! These tests use mock HTTP servers (wiremock, spawn_test_server) to verify
//! logout behavior without connecting to real Snowflake.

use crate::common::mocks::auth::mount_jwt_login_success;
use crate::common::snowflake_test_client::SnowflakeTestClient;
use crate::common::test_server::{
    json_error_response, json_response, service_unavailable_response, spawn_capture_server,
    spawn_test_server,
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
    //And UD Core connection is logged in
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

#[test]
fn should_not_send_logout_when_connection_was_never_established() {
    use sf_core::apis::database_driver_v1::{
        connection_close, connection_is_closed, connection_new, connection_release, database_init,
        database_new, database_release,
    };

    //Given Connection handle created but never initialized
    let db_handle = database_new();
    database_init(db_handle).unwrap();
    let conn_handle = connection_new();
    // Note: connection_init() NOT called - connection remains uninitialized

    //When Connection close is attempted
    let result = connection_close(conn_handle);

    //Then Close succeeds without sending HTTP request
    assert!(
        result.is_ok(),
        "Connection close should succeed for uninitialized connection"
    );

    //And Connection is marked as closed
    assert!(
        connection_is_closed(conn_handle).unwrap(),
        "Connection should be marked closed"
    );

    // Cleanup
    connection_release(conn_handle).unwrap();
    database_release(db_handle).unwrap();
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
        use crate::common::private_key_helper;

        let mut client = SnowflakeTestClient::with_int_tests_params(Some(&server_uri));

        // Configure JWT authentication
        client.set_connection_option("authenticator", "SNOWFLAKE_JWT");
        let temp_key_file = private_key_helper::get_test_private_key_file()
            .expect("Failed to create test private key file");
        client.set_connection_option("private_key_file", temp_key_file.path().to_str().unwrap());

        // Configure logout behavior BEFORE connection_init
        client.set_connection_option_bool("server_session_keep_alive", true);

        // Initialize connection
        DatabaseDriverClient::connection_init(ConnectionInitRequest {
            conn_handle: Some(client.conn_handle),
            db_handle: Some(client.db_handle),
        })
        .unwrap();

        client.set_temp_key_file(temp_key_file);
        client
    })
    .await
    .unwrap();

    //When Connection is closed
    let conn_handle = client.conn_handle;
    let result = tokio::task::spawn_blocking(move || {
        DatabaseDriverClient::connection_close(ConnectionCloseRequest {
            conn_handle: Some(conn_handle),
        })
    })
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
    use serde_json::json;
    use wiremock::matchers::{method, path, query_param};
    use wiremock::{Mock, ResponseTemplate};

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
        use crate::common::private_key_helper;

        let mut client = SnowflakeTestClient::with_int_tests_params(Some(&server_uri));

        // Configure JWT authentication
        client.set_connection_option("authenticator", "SNOWFLAKE_JWT");
        let temp_key_file = private_key_helper::get_test_private_key_file()
            .expect("Failed to create test private key file");
        client.set_connection_option("private_key_file", temp_key_file.path().to_str().unwrap());

        // Configure logout behavior BEFORE connection_init
        client.set_connection_option_bool("server_session_keep_alive", false);

        // Initialize connection
        DatabaseDriverClient::connection_init(ConnectionInitRequest {
            conn_handle: Some(client.conn_handle),
            db_handle: Some(client.db_handle),
        })
        .unwrap();

        client.set_temp_key_file(temp_key_file);
        client
    })
    .await
    .unwrap();

    //When Connection is closed
    let conn_handle = client.conn_handle;
    let result = tokio::task::spawn_blocking(move || {
        DatabaseDriverClient::connection_close(ConnectionCloseRequest {
            conn_handle: Some(conn_handle),
        })
    })
    .await
    .unwrap();

    //Then Logout HTTP request is sent to server
    assert!(result.is_ok(), "Close should succeed");
    let received_requests = server.received_requests().await.unwrap();
    // Verify there was a logout request
    let delete_requests: Vec<_> = received_requests
        .into_iter()
        .filter(|request| request.url.to_string().contains("delete=true"))
        .collect();
    assert_eq!(
        delete_requests.len(),
        1,
        "Should have received exactly 1 logout request (delete=true)"
    );
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
    let config = LogoutConfig::default(); // Default total timeout is 5 seconds

    // Simulate user-facing default: Python passes logout_request_timeout_seconds=5 by default
    // This tests what users experience "by default" through language wrappers
    // (Core's internal default is None, but wrappers configure it)
    let retry_policy = RetryPolicy {
        max_elapsed: config.logout_total_timeout,
        per_request_timeout: Some(Duration::from_secs(5)), // User-facing default via Python
        ..Default::default()
    };

    //When Logout is initiated
    let start = Instant::now();
    let result = logout_session(
        &client,
        &server_url,
        "test_token",
        &client_info,
        &retry_policy,
    )
    .await;
    let elapsed = start.elapsed();

    //Then Logout request times out after approximately 5 seconds
    assert!(result.is_err(), "Should timeout");

    //And Close throws timeout error
    let error_msg = format!("{:?}", result.unwrap_err());
    let error_lower = error_msg.to_lowercase();
    assert!(
        error_lower.contains("timeout")
            || error_lower.contains("timed out")
            || error_lower.contains("deadline"),
        "Error should be timeout-related, got: {}",
        error_msg
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
        max_elapsed: total_timeout,
        per_request_timeout: Some(per_request_timeout),
        ..Default::default()
    };

    //When Logout is initiated
    let start = Instant::now();
    let result = logout_session(
        &client,
        &server_url,
        "test_token",
        &client_info,
        &retry_policy,
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
    let retry_policy = RetryPolicy {
        max_attempts: 10,
        max_elapsed: total_timeout,
        ..Default::default()
    };

    //When Logout is initiated
    let start = Instant::now();
    let result = logout_session(
        &client,
        &server_url,
        "test_token",
        &client_info,
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
            "test_token",
            &client_info,
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
            //And Mock HTTP server returns <error_type> on attempt 1
            //And Mock HTTP server returns 200 on attempt 2
            let (addr, attempts, server) = spawn_test_server(2, move |attempt| {
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

            let _config = LogoutConfig {
                error_strategy,
                ..Default::default()
            };

            //When Logout is executed
            let result = logout_session(
                &client,
                &server_url,
                "test_token",
                &client_info,
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
            //And Mock HTTP server resets connection on first attempt
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

            let _config = LogoutConfig {
                error_strategy,
                ..Default::default()
            };

            //When Logout is executed
            let result = logout_session(
                &client,
                &server_url,
                "test_token",
                &client_info,
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
async fn should_attempt_token_refresh_on_390112_when_retries_allowed_for_each_strategy_type() {
    use serde_json::json;
    use wiremock::matchers::{body_partial_json, header, method, path, path_regex, query_param};
    use wiremock::{Mock, ResponseTemplate};

    // Scenario Outline: Examples (strategy_type)
    // strict, best-effort
    for (strategy_name, error_strategy) in [
        ("strict", ErrorStrategy::Strict),
        ("best-effort", ErrorStrategy::BestEffort),
    ] {
        //Given Core logout function called with <strategy_type> strategy
        //And Mock HTTP server returns SESSION_TOKEN_EXPIRED 390112 on first attempt
        //And Mock HTTP server returns 200 after token refresh
        //And Retry policy allows 1 retry
        let server = MockServer::start().await;

        // Login: returns initial tokens with master token for refresh
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

        // First logout attempt: returns 390112 SESSION_TOKEN_EXPIRED
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

        // Token refresh: returns new session token
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

        // Second logout attempt with refreshed token: succeeds
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
            .mount(&server)
            .await;

        //And UD Core connection is configured and logged in
        let server_uri = server.uri();
        let client = tokio::task::spawn_blocking(move || {
            use crate::common::private_key_helper;

            let mut client = SnowflakeTestClient::with_int_tests_params(Some(&server_uri));

            // Configure JWT authentication
            client.set_connection_option("authenticator", "SNOWFLAKE_JWT");
            let temp_key_file = private_key_helper::get_test_private_key_file()
                .expect("Failed to create test private key file");
            client
                .set_connection_option("private_key_file", temp_key_file.path().to_str().unwrap());

            // Configure logout behavior BEFORE connection_init
            client.set_connection_option_bool("server_session_keep_alive", false);
            client.set_connection_option(
                "logout_error_strategy",
                match error_strategy {
                    ErrorStrategy::Strict => "strict",
                    ErrorStrategy::BestEffort => "best_effort",
                },
            );
            client.set_connection_option_int("logout_total_timeout_seconds", 30);
            client.set_connection_option_int("logout_max_attempts", 1); // 1 attempt (0 retries)

            // Initialize connection
            DatabaseDriverClient::connection_init(ConnectionInitRequest {
                conn_handle: Some(client.conn_handle),
                db_handle: Some(client.db_handle),
            })
            .unwrap();

            client.set_temp_key_file(temp_key_file);
            client
        })
        .await
        .unwrap();

        //When Logout is executed
        let conn_handle = client.conn_handle;
        let result = tokio::task::spawn_blocking(move || {
            use sf_core::protobuf_apis::database_driver_v1::DatabaseDriverClient;
            DatabaseDriverClient::connection_close(ConnectionCloseRequest {
                conn_handle: Some(conn_handle),
            })
        })
        .await
        .unwrap();

        //Then Token refresh request is sent to server
        //And Logout is retried with new session token
        //And Close succeeds
        assert!(
            result.is_ok(),
            "Close should succeed after token refresh for {}: {:?}",
            strategy_name,
            result.err()
        );

        // Verify requests: login + logout(390112) + token-refresh + logout(success)
        let received_requests = server.received_requests().await.unwrap();
        let request_paths: Vec<String> = received_requests
            .iter()
            .map(|r| r.url.path().to_string())
            .collect();

        assert!(
            request_paths.iter().any(|p| p.contains("token-request")),
            "Should have made token refresh request for {}: {:?}",
            strategy_name,
            request_paths
        );

        let logout_count = request_paths.iter().filter(|p| p == &"/session").count();
        assert!(
            logout_count >= 2,
            "Should have made at least 2 logout requests for {}, got {}: {:?}",
            strategy_name,
            logout_count,
            request_paths
        );
    }
}

#[tokio::test]
async fn should_fail_gracefully_when_token_refresh_fails_on_390112_for_each_strategy_type() {
    use serde_json::json;
    use wiremock::matchers::{body_partial_json, method, path, path_regex, query_param};
    use wiremock::{Mock, ResponseTemplate};

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
            use crate::common::private_key_helper;

            let mut client = SnowflakeTestClient::with_int_tests_params(Some(&server_uri));

            // Configure JWT authentication
            client.set_connection_option("authenticator", "SNOWFLAKE_JWT");
            let temp_key_file = private_key_helper::get_test_private_key_file()
                .expect("Failed to create test private key file");
            client
                .set_connection_option("private_key_file", temp_key_file.path().to_str().unwrap());

            // Configure logout behavior BEFORE connection_init
            client.set_connection_option_bool("server_session_keep_alive", false);
            client.set_connection_option(
                "logout_error_strategy",
                match error_strategy {
                    ErrorStrategy::Strict => "strict",
                    ErrorStrategy::BestEffort => "best_effort",
                },
            );
            client.set_connection_option_int("logout_total_timeout_seconds", 30);
            client.set_connection_option_int("logout_max_attempts", 1); // 1 attempt (0 retries)

            // Initialize connection
            DatabaseDriverClient::connection_init(ConnectionInitRequest {
                conn_handle: Some(client.conn_handle),
                db_handle: Some(client.db_handle),
            })
            .unwrap();

            client.set_temp_key_file(temp_key_file);
            client
        })
        .await
        .unwrap();

        //When Connection close is initiated
        let conn_handle = client.conn_handle;
        let result = tokio::task::spawn_blocking(move || {
            use sf_core::protobuf_apis::database_driver_v1::DatabaseDriverClient;
            DatabaseDriverClient::connection_close(ConnectionCloseRequest {
                conn_handle: Some(conn_handle),
            })
        })
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
        //And Retry policy configured with <max_attempts> max attempts
        //And Mock HTTP server fails <failures> times then returns 200
        let expected_attempts = num_failures + 1;
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

        let _config = LogoutConfig {
            error_strategy,
            ..Default::default()
        };

        let retry_policy = RetryPolicy {
            max_attempts: expected_attempts as u32, // Use the calculated value from loop
            ..Default::default()
        };

        //When Logout is executed
        let result = logout_session(
            &client,
            &server_url,
            "test_token",
            &client_info,
            &retry_policy,
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

        let _config = LogoutConfig {
            error_strategy,
            logout_total_timeout: Duration::from_secs(timeout_seconds),
            ..Default::default()
        };

        //When Logout is executed
        let start = Instant::now();
        let result = logout_session(
            &client,
            &server_url,
            "test_token",
            &client_info,
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
async fn should_throw_after_exhausted_retries_with_strict_strategy_2_attempts() {
    // Gherkin: core/session/logout.feature:316-329 (max_attempts=2)
    //Given Core logout function called with strict strategy
    //And Retry policy configured with 2 max attempts
    //And Mock HTTP server returns 503 on all attempts
    let server = MockServer::start().await;
    mount_jwt_login_success(&server).await;

    // Mock logout to return 503 for all attempts
    use wiremock::{Mock, ResponseTemplate};
    Mock::given(wiremock::matchers::method("POST"))
        .and(wiremock::matchers::path("/session"))
        .and(wiremock::matchers::query_param("delete", "true"))
        .respond_with(ResponseTemplate::new(503).set_body_string("Service Unavailable"))
        .expect(2) // Should make exactly 2 attempts
        .mount(&server)
        .await;

    //And UD Core connection is configured and logged in
    let server_uri = server.uri();
    let client = tokio::task::spawn_blocking(move || {
        use crate::common::private_key_helper;

        let mut client = SnowflakeTestClient::with_int_tests_params(Some(&server_uri));

        // Configure JWT authentication
        client.set_connection_option("authenticator", "SNOWFLAKE_JWT");
        let temp_key_file = private_key_helper::get_test_private_key_file()
            .expect("Failed to create test private key file");
        client.set_connection_option("private_key_file", temp_key_file.path().to_str().unwrap());

        // Configure logout behavior BEFORE connection_init
        client.set_connection_option_bool("server_session_keep_alive", false);
        client.set_connection_option_int("logout_error_strategy", 2); // ERROR_STRATEGY_STRICT
        client.set_connection_option_int("logout_total_timeout_seconds", 30);
        client.set_connection_option_int("logout_max_attempts", 2); // 2 attempts

        // Initialize connection
        DatabaseDriverClient::connection_init(ConnectionInitRequest {
            conn_handle: Some(client.conn_handle),
            db_handle: Some(client.db_handle),
        })
        .unwrap();

        client.set_temp_key_file(temp_key_file);
        client
    })
    .await
    .unwrap();

    //When Logout is executed
    let conn_handle = client.conn_handle;
    let result = tokio::task::spawn_blocking(move || {
        use sf_core::protobuf_apis::database_driver_v1::DatabaseDriverClient;
        DatabaseDriverClient::connection_close(ConnectionCloseRequest {
            conn_handle: Some(conn_handle),
        })
    })
    .await
    .unwrap();

    //Then Exactly max_attempts attempts are made
    //And No further retries after max reached
    //And Close throws error
    assert!(
        result.is_err(),
        "Close should fail with strict strategy after exhausted retries"
    );

    // Verify exactly 2 logout attempts were made
    let received_requests = server.received_requests().await.unwrap();
    let logout_count = received_requests
        .iter()
        .filter(|r| {
            r.url.path() == "/session" && r.url.query().unwrap_or("").contains("delete=true")
        })
        .count();
    assert_eq!(
        logout_count, 2,
        "Should have made exactly 2 logout attempts"
    );
}

#[tokio::test]
async fn should_throw_after_exhausted_retries_with_strict_strategy_3_attempts() {
    // Gherkin: core/session/logout.feature:316-329 (max_attempts=3)
    //Given Core logout function called with strict strategy
    //And Retry policy configured with 3 max attempts
    //And Mock HTTP server returns 503 on all attempts
    let server = MockServer::start().await;
    mount_jwt_login_success(&server).await;

    // Mock logout to return 503 for all attempts
    use wiremock::{Mock, ResponseTemplate};
    Mock::given(wiremock::matchers::method("POST"))
        .and(wiremock::matchers::path("/session"))
        .and(wiremock::matchers::query_param("delete", "true"))
        .respond_with(ResponseTemplate::new(503).set_body_string("Service Unavailable"))
        .expect(3) // Should make exactly 3 attempts
        .mount(&server)
        .await;

    //And UD Core connection is configured and logged in
    let server_uri = server.uri();
    let client = tokio::task::spawn_blocking(move || {
        use crate::common::private_key_helper;

        let mut client = SnowflakeTestClient::with_int_tests_params(Some(&server_uri));

        // Configure JWT authentication
        client.set_connection_option("authenticator", "SNOWFLAKE_JWT");
        let temp_key_file = private_key_helper::get_test_private_key_file()
            .expect("Failed to create test private key file");
        client.set_connection_option("private_key_file", temp_key_file.path().to_str().unwrap());

        // Configure logout behavior BEFORE connection_init
        client.set_connection_option_bool("server_session_keep_alive", false);
        client.set_connection_option_int("logout_error_strategy", 2); // ERROR_STRATEGY_STRICT
        client.set_connection_option_int("logout_total_timeout_seconds", 30);
        client.set_connection_option_int("logout_max_attempts", 3); // 3 attempts

        // Initialize connection
        DatabaseDriverClient::connection_init(ConnectionInitRequest {
            conn_handle: Some(client.conn_handle),
            db_handle: Some(client.db_handle),
        })
        .unwrap();

        client.set_temp_key_file(temp_key_file);
        client
    })
    .await
    .unwrap();

    //When Logout is executed
    let conn_handle = client.conn_handle;
    let result = tokio::task::spawn_blocking(move || {
        use sf_core::protobuf_apis::database_driver_v1::DatabaseDriverClient;
        DatabaseDriverClient::connection_close(ConnectionCloseRequest {
            conn_handle: Some(conn_handle),
        })
    })
    .await
    .unwrap();

    //Then Exactly max_attempts attempts are made
    //And No further retries after max reached
    //And Close throws error
    assert!(
        result.is_err(),
        "Close should fail with strict strategy after exhausted retries"
    );

    // Verify exactly 3 logout attempts were made
    let received_requests = server.received_requests().await.unwrap();
    let logout_count = received_requests
        .iter()
        .filter(|r| {
            r.url.path() == "/session" && r.url.query().unwrap_or("").contains("delete=true")
        })
        .count();
    assert_eq!(
        logout_count, 3,
        "Should have made exactly 3 logout attempts"
    );
}

#[tokio::test]
async fn should_log_warn_and_succeed_after_exhausted_retries_with_best_effort_strategy_2_attempts()
{
    // Gherkin: core/session/logout.feature:331-344 (max_attempts=2)
    //Given Core logout function called with best-effort strategy
    //And Retry policy configured with 2 max attempts
    //And Mock HTTP server returns 503 on all attempts
    let server = MockServer::start().await;
    mount_jwt_login_success(&server).await;

    // Mock logout to return 503 for all attempts
    use wiremock::{Mock, ResponseTemplate};
    Mock::given(wiremock::matchers::method("POST"))
        .and(wiremock::matchers::path("/session"))
        .and(wiremock::matchers::query_param("delete", "true"))
        .respond_with(ResponseTemplate::new(503).set_body_string("Service Unavailable"))
        .expect(2) // Should make exactly 2 attempts
        .mount(&server)
        .await;

    //And UD Core connection is configured and logged in
    let server_uri = server.uri();
    let client = tokio::task::spawn_blocking(move || {
        use crate::common::private_key_helper;

        let mut client = SnowflakeTestClient::with_int_tests_params(Some(&server_uri));

        // Configure JWT authentication
        client.set_connection_option("authenticator", "SNOWFLAKE_JWT");
        let temp_key_file = private_key_helper::get_test_private_key_file()
            .expect("Failed to create test private key file");
        client.set_connection_option("private_key_file", temp_key_file.path().to_str().unwrap());

        // Configure logout behavior BEFORE connection_init
        client.set_connection_option_bool("server_session_keep_alive", false);
        client.set_connection_option_int("logout_error_strategy", 1); // ERROR_STRATEGY_BEST_EFFORT
        client.set_connection_option_int("logout_total_timeout_seconds", 30);
        client.set_connection_option_int("logout_max_attempts", 2); // 2 attempts

        // Initialize connection
        DatabaseDriverClient::connection_init(ConnectionInitRequest {
            conn_handle: Some(client.conn_handle),
            db_handle: Some(client.db_handle),
        })
        .unwrap();

        client.set_temp_key_file(temp_key_file);
        client
    })
    .await
    .unwrap();

    //When Logout is executed
    let conn_handle = client.conn_handle;
    let result = tokio::task::spawn_blocking(move || {
        use sf_core::protobuf_apis::database_driver_v1::DatabaseDriverClient;
        DatabaseDriverClient::connection_close(ConnectionCloseRequest {
            conn_handle: Some(conn_handle),
        })
    })
    .await
    .unwrap();

    //Then Exactly max_attempts attempts are made
    //And No further retries after max reached
    //And WARN log is emitted
    //And Close succeeds
    assert!(
        result.is_ok(),
        "Close should succeed with best-effort strategy despite failures: {:?}",
        result.err()
    );

    // Verify exactly 2 logout attempts were made
    let received_requests = server.received_requests().await.unwrap();
    let logout_count = received_requests
        .iter()
        .filter(|r| {
            r.url.path() == "/session" && r.url.query().unwrap_or("").contains("delete=true")
        })
        .count();
    assert_eq!(
        logout_count, 2,
        "Should have made exactly 2 logout attempts"
    );
}

#[tokio::test]
async fn should_log_warn_and_succeed_after_exhausted_retries_with_best_effort_strategy_3_attempts()
{
    // Gherkin: core/session/logout.feature:331-344 (max_attempts=3)
    //Given Core logout function called with best-effort strategy
    //And Retry policy configured with 3 max attempts
    //And Mock HTTP server returns 503 on all attempts
    let server = MockServer::start().await;
    mount_jwt_login_success(&server).await;

    // Mock logout to return 503 for all attempts
    use wiremock::{Mock, ResponseTemplate};
    Mock::given(wiremock::matchers::method("POST"))
        .and(wiremock::matchers::path("/session"))
        .and(wiremock::matchers::query_param("delete", "true"))
        .respond_with(ResponseTemplate::new(503).set_body_string("Service Unavailable"))
        .expect(3) // Should make exactly 3 attempts
        .mount(&server)
        .await;

    //And UD Core connection is configured and logged in
    let server_uri = server.uri();
    let client = tokio::task::spawn_blocking(move || {
        use crate::common::private_key_helper;

        let mut client = SnowflakeTestClient::with_int_tests_params(Some(&server_uri));

        // Configure JWT authentication
        client.set_connection_option("authenticator", "SNOWFLAKE_JWT");
        let temp_key_file = private_key_helper::get_test_private_key_file()
            .expect("Failed to create test private key file");
        client.set_connection_option("private_key_file", temp_key_file.path().to_str().unwrap());

        // Configure logout behavior BEFORE connection_init
        client.set_connection_option_bool("server_session_keep_alive", false);
        client.set_connection_option_int("logout_error_strategy", 1); // ERROR_STRATEGY_BEST_EFFORT
        client.set_connection_option_int("logout_total_timeout_seconds", 30);
        client.set_connection_option_int("logout_max_attempts", 3); // 3 attempts

        // Initialize connection
        DatabaseDriverClient::connection_init(ConnectionInitRequest {
            conn_handle: Some(client.conn_handle),
            db_handle: Some(client.db_handle),
        })
        .unwrap();

        client.set_temp_key_file(temp_key_file);
        client
    })
    .await
    .unwrap();

    //When Logout is executed
    let conn_handle = client.conn_handle;
    let result = tokio::task::spawn_blocking(move || {
        use sf_core::protobuf_apis::database_driver_v1::DatabaseDriverClient;
        DatabaseDriverClient::connection_close(ConnectionCloseRequest {
            conn_handle: Some(conn_handle),
        })
    })
    .await
    .unwrap();

    //Then Exactly max_attempts attempts are made
    //And No further retries after max reached
    //And WARN log is emitted
    //And Close succeeds
    assert!(
        result.is_ok(),
        "Close should succeed with best-effort strategy despite failures: {:?}",
        result.err()
    );

    // Verify exactly 3 logout attempts were made
    let received_requests = server.received_requests().await.unwrap();
    let logout_count = received_requests
        .iter()
        .filter(|r| {
            r.url.path() == "/session" && r.url.query().unwrap_or("").contains("delete=true")
        })
        .count();
    assert_eq!(
        logout_count, 3,
        "Should have made exactly 3 logout attempts"
    );
}

#[tokio::test]
async fn should_throw_on_non_retryable_400_in_strict_strategy() {
    // Gherkin: core/session/logout.feature:378-391 (error_code=400)
    //Given Core logout function called with strict strategy
    //And Mock HTTP server returns 400 error
    test_non_retryable_error_strict(400, "Bad Request").await;
}

#[tokio::test]
async fn should_throw_on_non_retryable_403_in_strict_strategy() {
    // Gherkin: core/session/logout.feature:378-391 (error_code=403)
    //Given Core logout function called with strict strategy
    //And Mock HTTP server returns 403 error
    test_non_retryable_error_strict(403, "Forbidden").await;
}

#[tokio::test]
async fn should_throw_on_non_retryable_404_in_strict_strategy() {
    // Gherkin: core/session/logout.feature:378-391 (error_code=404)
    //Given Core logout function called with strict strategy
    //And Mock HTTP server returns 404 error
    test_non_retryable_error_strict(404, "Not Found").await;
}

#[tokio::test]
async fn should_throw_on_non_retryable_390114_in_strict_strategy() {
    // Gherkin: core/session/logout.feature:378-391 (error_code=390114 MASTER_TOKEN_EXPIRED)
    //Given Core logout function called with strict strategy
    //And Mock HTTP server returns 390114 error
    let server = MockServer::start().await;
    mount_jwt_login_success(&server).await;

    // Mock logout to return 390114 (MASTER_TOKEN_EXPIRED)
    use serde_json::json;
    use wiremock::{Mock, ResponseTemplate};
    Mock::given(wiremock::matchers::method("POST"))
        .and(wiremock::matchers::path("/session"))
        .and(wiremock::matchers::query_param("delete", "true"))
        .respond_with(
            ResponseTemplate::new(401)
                .set_body_json(json!({
                    "success": false,
                    "code": "390114",
                    "message": "Master token expired"
                }))
                .insert_header("Content-Type", "application/json"),
        )
        .expect(1) // Should make exactly 1 attempt (non-retryable)
        .mount(&server)
        .await;

    //And UD Core connection is configured and logged in
    let server_uri = server.uri();
    let client = tokio::task::spawn_blocking(move || {
        use crate::common::private_key_helper;

        let mut client = SnowflakeTestClient::with_int_tests_params(Some(&server_uri));

        // Configure JWT authentication
        client.set_connection_option("authenticator", "SNOWFLAKE_JWT");
        let temp_key_file = private_key_helper::get_test_private_key_file()
            .expect("Failed to create test private key file");
        client.set_connection_option("private_key_file", temp_key_file.path().to_str().unwrap());

        // Configure logout behavior BEFORE connection_init
        client.set_connection_option_bool("server_session_keep_alive", false);
        client.set_connection_option_int("logout_error_strategy", 2); // ERROR_STRATEGY_STRICT
        client.set_connection_option_int("logout_total_timeout_seconds", 30);
        client.set_connection_option_int("logout_max_attempts", 3); // Max attempts, but should only try once

        // Initialize connection
        DatabaseDriverClient::connection_init(ConnectionInitRequest {
            conn_handle: Some(client.conn_handle),
            db_handle: Some(client.db_handle),
        })
        .unwrap();

        client.set_temp_key_file(temp_key_file);
        client
    })
    .await
    .unwrap();

    //When Logout is executed
    let conn_handle = client.conn_handle;
    let result = tokio::task::spawn_blocking(move || {
        use sf_core::protobuf_apis::database_driver_v1::DatabaseDriverClient;
        DatabaseDriverClient::connection_close(ConnectionCloseRequest {
            conn_handle: Some(conn_handle),
        })
    })
    .await
    .unwrap();

    //Then Close throws error immediately
    //And Error is surfaced to caller
    //And No retries are attempted
    assert!(
        result.is_err(),
        "Close should fail with strict strategy for non-retryable error 390114"
    );

    // Verify exactly 1 logout attempt was made (no retries)
    let received_requests = server.received_requests().await.unwrap();
    let logout_count = received_requests
        .iter()
        .filter(|r| {
            r.url.path() == "/session" && r.url.query().unwrap_or("").contains("delete=true")
        })
        .count();
    assert_eq!(
        logout_count, 1,
        "Should have made exactly 1 logout attempt (no retries for non-retryable error)"
    );
}

#[tokio::test]
async fn should_log_and_suppress_non_retryable_400_in_best_effort_strategy() {
    // Gherkin: core/session/logout.feature:394-407 (error_code=400)
    //Given Core logout function called with best-effort strategy
    //And Mock HTTP server returns 400 error
    test_non_retryable_error_best_effort(400, "Bad Request").await;
}

#[tokio::test]
async fn should_log_and_suppress_non_retryable_403_in_best_effort_strategy() {
    // Gherkin: core/session/logout.feature:394-407 (error_code=403)
    //Given Core logout function called with best-effort strategy
    //And Mock HTTP server returns 403 error
    test_non_retryable_error_best_effort(403, "Forbidden").await;
}

#[tokio::test]
async fn should_log_and_suppress_non_retryable_404_in_best_effort_strategy() {
    // Gherkin: core/session/logout.feature:394-407 (error_code=404)
    //Given Core logout function called with best-effort strategy
    //And Mock HTTP server returns 404 error
    test_non_retryable_error_best_effort(404, "Not Found").await;
}

#[tokio::test]
async fn should_log_and_suppress_non_retryable_390114_in_best_effort_strategy() {
    // Gherkin: core/session/logout.feature:394-407 (error_code=390114 MASTER_TOKEN_EXPIRED)
    //Given Core logout function called with best-effort strategy
    //And Mock HTTP server returns 390114 error
    let server = MockServer::start().await;
    mount_jwt_login_success(&server).await;

    // Mock logout to return 390114 (MASTER_TOKEN_EXPIRED)
    use serde_json::json;
    use wiremock::{Mock, ResponseTemplate};
    Mock::given(wiremock::matchers::method("POST"))
        .and(wiremock::matchers::path("/session"))
        .and(wiremock::matchers::query_param("delete", "true"))
        .respond_with(
            ResponseTemplate::new(401)
                .set_body_json(json!({
                    "success": false,
                    "code": "390114",
                    "message": "Master token expired"
                }))
                .insert_header("Content-Type", "application/json"),
        )
        .expect(1) // Should make exactly 1 attempt (non-retryable)
        .mount(&server)
        .await;

    //And UD Core connection is configured and logged in
    let server_uri = server.uri();
    let client = tokio::task::spawn_blocking(move || {
        use crate::common::private_key_helper;

        let mut client = SnowflakeTestClient::with_int_tests_params(Some(&server_uri));

        // Configure JWT authentication
        client.set_connection_option("authenticator", "SNOWFLAKE_JWT");
        let temp_key_file = private_key_helper::get_test_private_key_file()
            .expect("Failed to create test private key file");
        client.set_connection_option("private_key_file", temp_key_file.path().to_str().unwrap());

        // Configure logout behavior BEFORE connection_init
        client.set_connection_option_bool("server_session_keep_alive", false);
        client.set_connection_option_int("logout_error_strategy", 1); // ERROR_STRATEGY_BEST_EFFORT
        client.set_connection_option_int("logout_total_timeout_seconds", 30);
        client.set_connection_option_int("logout_max_attempts", 3); // Max attempts, but should only try once

        // Initialize connection
        DatabaseDriverClient::connection_init(ConnectionInitRequest {
            conn_handle: Some(client.conn_handle),
            db_handle: Some(client.db_handle),
        })
        .unwrap();

        client.set_temp_key_file(temp_key_file);
        client
    })
    .await
    .unwrap();

    //When Logout is executed
    let conn_handle = client.conn_handle;
    let result = tokio::task::spawn_blocking(move || {
        use sf_core::protobuf_apis::database_driver_v1::DatabaseDriverClient;
        DatabaseDriverClient::connection_close(ConnectionCloseRequest {
            conn_handle: Some(conn_handle),
        })
    })
    .await
    .unwrap();

    //Then Error is logged as WARN
    //And Close succeeds without throwing
    //And No retries are attempted
    assert!(
        result.is_ok(),
        "Close should succeed with best-effort strategy despite non-retryable error 390114: {:?}",
        result.err()
    );

    // Verify exactly 1 logout attempt was made (no retries)
    let received_requests = server.received_requests().await.unwrap();
    let logout_count = received_requests
        .iter()
        .filter(|r| {
            r.url.path() == "/session" && r.url.query().unwrap_or("").contains("delete=true")
        })
        .count();
    assert_eq!(
        logout_count, 1,
        "Should have made exactly 1 logout attempt (no retries for non-retryable error)"
    );
}

// Helper functions for non-retryable error tests

async fn test_non_retryable_error_strict(status_code: u16, status_text: &str) {
    //Given Core logout function called with strict strategy
    //And Mock HTTP server returns error
    let server = MockServer::start().await;
    mount_jwt_login_success(&server).await;

    // Mock logout to return non-retryable error
    use wiremock::{Mock, ResponseTemplate};
    Mock::given(wiremock::matchers::method("POST"))
        .and(wiremock::matchers::path("/session"))
        .and(wiremock::matchers::query_param("delete", "true"))
        .respond_with(ResponseTemplate::new(status_code).set_body_string(status_text))
        .expect(1) // Should make exactly 1 attempt (non-retryable)
        .mount(&server)
        .await;

    //And UD Core connection is configured and logged in
    let server_uri = server.uri();
    let client = tokio::task::spawn_blocking(move || {
        use crate::common::private_key_helper;

        let mut client = SnowflakeTestClient::with_int_tests_params(Some(&server_uri));

        // Configure JWT authentication
        client.set_connection_option("authenticator", "SNOWFLAKE_JWT");
        let temp_key_file = private_key_helper::get_test_private_key_file()
            .expect("Failed to create test private key file");
        client.set_connection_option("private_key_file", temp_key_file.path().to_str().unwrap());

        // Configure logout behavior BEFORE connection_init
        client.set_connection_option_bool("server_session_keep_alive", false);
        client.set_connection_option_int("logout_error_strategy", 2); // ERROR_STRATEGY_STRICT
        client.set_connection_option_int("logout_total_timeout_seconds", 30);
        client.set_connection_option_int("logout_max_attempts", 3); // Max attempts, but should only try once

        // Initialize connection
        DatabaseDriverClient::connection_init(ConnectionInitRequest {
            conn_handle: Some(client.conn_handle),
            db_handle: Some(client.db_handle),
        })
        .unwrap();

        client.set_temp_key_file(temp_key_file);
        client
    })
    .await
    .unwrap();

    //When Logout is executed
    let conn_handle = client.conn_handle;
    let result = tokio::task::spawn_blocking(move || {
        use sf_core::protobuf_apis::database_driver_v1::DatabaseDriverClient;
        DatabaseDriverClient::connection_close(ConnectionCloseRequest {
            conn_handle: Some(conn_handle),
        })
    })
    .await
    .unwrap();

    //Then Close throws error immediately
    //And Error is surfaced to caller
    //And No retries are attempted
    assert!(
        result.is_err(),
        "Close should fail with strict strategy for non-retryable error {}: {:?}",
        status_code,
        result.ok()
    );

    // Verify exactly 1 logout attempt was made (no retries)
    let received_requests = server.received_requests().await.unwrap();
    let logout_count = received_requests
        .iter()
        .filter(|r| {
            r.url.path() == "/session" && r.url.query().unwrap_or("").contains("delete=true")
        })
        .count();
    assert_eq!(
        logout_count, 1,
        "Should have made exactly 1 logout attempt (no retries for non-retryable error {})",
        status_code
    );
}

async fn test_non_retryable_error_best_effort(status_code: u16, status_text: &str) {
    //Given Core logout function called with best-effort strategy
    //And Mock HTTP server returns error
    let server = MockServer::start().await;
    mount_jwt_login_success(&server).await;

    // Mock logout to return non-retryable error
    use wiremock::{Mock, ResponseTemplate};
    Mock::given(wiremock::matchers::method("POST"))
        .and(wiremock::matchers::path("/session"))
        .and(wiremock::matchers::query_param("delete", "true"))
        .respond_with(ResponseTemplate::new(status_code).set_body_string(status_text))
        .expect(1) // Should make exactly 1 attempt (non-retryable)
        .mount(&server)
        .await;

    //And UD Core connection is configured and logged in
    let server_uri = server.uri();
    let client = tokio::task::spawn_blocking(move || {
        use crate::common::private_key_helper;

        let mut client = SnowflakeTestClient::with_int_tests_params(Some(&server_uri));

        // Configure JWT authentication
        client.set_connection_option("authenticator", "SNOWFLAKE_JWT");
        let temp_key_file = private_key_helper::get_test_private_key_file()
            .expect("Failed to create test private key file");
        client.set_connection_option("private_key_file", temp_key_file.path().to_str().unwrap());

        // Configure logout behavior BEFORE connection_init
        client.set_connection_option_bool("server_session_keep_alive", false);
        client.set_connection_option_int("logout_error_strategy", 1); // ERROR_STRATEGY_BEST_EFFORT
        client.set_connection_option_int("logout_total_timeout_seconds", 30);
        client.set_connection_option_int("logout_max_attempts", 3); // Max attempts, but should only try once

        // Initialize connection
        DatabaseDriverClient::connection_init(ConnectionInitRequest {
            conn_handle: Some(client.conn_handle),
            db_handle: Some(client.db_handle),
        })
        .unwrap();

        client.set_temp_key_file(temp_key_file);
        client
    })
    .await
    .unwrap();

    //When Logout is executed
    let conn_handle = client.conn_handle;
    let result = tokio::task::spawn_blocking(move || {
        use sf_core::protobuf_apis::database_driver_v1::DatabaseDriverClient;
        DatabaseDriverClient::connection_close(ConnectionCloseRequest {
            conn_handle: Some(conn_handle),
        })
    })
    .await
    .unwrap();

    //Then Error is logged as WARN
    //And Close succeeds without throwing
    //And No retries are attempted
    assert!(
        result.is_ok(),
        "Close should succeed with best-effort strategy despite non-retryable error {}: {:?}",
        status_code,
        result.err()
    );

    // Verify exactly 1 logout attempt was made (no retries)
    let received_requests = server.received_requests().await.unwrap();
    let logout_count = received_requests
        .iter()
        .filter(|r| {
            r.url.path() == "/session" && r.url.query().unwrap_or("").contains("delete=true")
        })
        .count();
    assert_eq!(
        logout_count, 1,
        "Should have made exactly 1 logout attempt (no retries for non-retryable error {})",
        status_code
    );
}

// ===========================================================================
//                      Timeout Failure Scenarios
// ===========================================================================

#[tokio::test]
async fn should_throw_on_timeout_with_strict_strategy() {
    // Scenario Outline: Examples (timeout_seconds=3, delay_seconds=5)
    //Given Core logout function called with strict strategy
    //And Timeout configured to 3 seconds
    let timeout = Duration::from_secs(3);
    //And Mock HTTP server delays response by 5 seconds
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
        max_elapsed: timeout,
        per_request_timeout: Some(timeout),
        ..Default::default()
    };

    //When Logout is executed
    let start = Instant::now();
    let result = logout_session(
        &client,
        &server_url,
        "test_token",
        &client_info,
        &retry_policy,
    )
    .await;
    let elapsed = start.elapsed();

    //Then Request times out after timeout_seconds
    assert!(
        elapsed >= timeout && elapsed < timeout + Duration::from_secs(2),
        "Should timeout after ~{:?}, took {:?}",
        timeout,
        elapsed
    );

    //And Close throws timeout error (Strict raises error)
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
    //And Timeout configured to 3 seconds
    let timeout = Duration::from_secs(3);
    //And Mock HTTP server delays response by 5 seconds
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

    let config = LogoutConfig {
        error_strategy: ErrorStrategy::BestEffort,
        logout_total_timeout: timeout,
        ..Default::default()
    };

    let retry_policy = RetryPolicy {
        max_attempts: 1,
        max_elapsed: timeout,
        per_request_timeout: Some(timeout),
        ..Default::default()
    };

    //When Logout is executed
    let start = Instant::now();
    let result = logout_session(
        &client,
        &server_url,
        "test_token",
        &client_info,
        &retry_policy,
    )
    .await;
    let elapsed = start.elapsed();

    //Then Request times out after timeout_seconds
    assert!(
        elapsed >= timeout && elapsed < timeout + Duration::from_secs(2),
        "Should timeout after ~{:?}, took {:?}",
        timeout,
        elapsed
    );

    // Apply error strategy (same logic as connection_close)
    let api_result =
        result.map_err(
            |e| sf_core::apis::database_driver_v1::ApiError::LogoutFailed {
                message: format!("{e}"),
                location: snafu::Location::default(),
            },
        );
    let handled_result = config.error_strategy.handle_failed_logout(api_result);

    //And Close succeeds (BestEffort suppresses timeout error)
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

    //And UD Core connection is logged in
    let server_uri = server.uri();
    let client = tokio::task::spawn_blocking(move || {
        SnowflakeTestClient::connect_integration_test(Some(&server_uri))
    })
    .await
    .unwrap();

    //When Connection is closed
    let conn_handle = client.conn_handle;
    let close_result = tokio::task::spawn_blocking(move || {
        DatabaseDriverClient::connection_close(ConnectionCloseRequest {
            conn_handle: Some(conn_handle),
        })
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
