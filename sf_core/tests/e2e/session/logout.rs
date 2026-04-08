//! E2E tests for session logout functionality.
//!
//! These tests implement scenarios from shared/session/logout.feature.
//! Token cleanup tests use real Snowflake; idempotent/concurrent close tests
//! use WireMock so "Only one logout request is sent" can be honestly verified
//! via HTTP request counting.

use crate::common::mocks::auth::mount_jwt_login_success;
use crate::common::snowflake_test_client::SnowflakeTestClient;
use serde_json::json;
use wiremock::matchers::{method, path, query_param};
use wiremock::{Mock, MockServer, ResponseTemplate};

// ===========================================================================
//                          Token Cleanup
// ===========================================================================

#[test]
fn should_cleanup_all_tokens_on_close_regardless_of_whether_logout_was_sent() {
    for keep_alive in [Some(true), Some(false), None] {
        //Given Snowflake client is logged in
        let client = SnowflakeTestClient::with_default_jwt_auth_params();

        //And server_session_keep_alive is set to <server_session_keep_alive>
        if let Some(value) = keep_alive {
            client.set_connection_option_bool("server_session_keep_alive", value);
        }

        // Connect (shared setup, not a Gherkin step)
        client.connect().expect("Connection should succeed");

        // Precondition: tokens are non-null before close
        let info_before = client
            .connection_get_info_blocking(true)
            .expect("get_info before close");
        assert!(
            !info_before.session_token.unwrap_or_default().is_empty(),
            "session_token must be non-null before close"
        );
        assert!(
            !info_before.master_token.unwrap_or_default().is_empty(),
            "master_token must be non-null before close"
        );

        //When Connection is closed
        let result = client.connection_close_blocking();
        assert!(
            result.is_ok(),
            "Close should succeed with server_session_keep_alive={:?}",
            keep_alive
        );

        //Then Session token in Connection.tokens is null
        let info = client
            .connection_get_info_blocking(true)
            .expect("get_info should work on closed connection handle");
        assert!(
            info.session_token.unwrap_or_default().is_empty(),
            "session_token must be null after close (keep_alive={:?})",
            keep_alive
        );

        //And Master token in Connection.tokens is null
        assert!(
            info.master_token.unwrap_or_default().is_empty(),
            "master_token must be null after close (keep_alive={:?})",
            keep_alive
        );
    }
}

#[tokio::test]
async fn should_be_idempotent_when_close_called_multiple_times() {
    //Given Snowflake client is logged in
    let server = MockServer::start().await;
    mount_jwt_login_success(&server).await;
    mount_logout_success(&server).await;

    let server_uri = server.uri();
    let client = tokio::task::spawn_blocking(move || {
        SnowflakeTestClient::connect_integration_test(Some(&server_uri))
    })
    .await
    .unwrap();

    //When Connection is closed
    let result1 = client.connection_close_blocking();

    //And Connection is closed again
    let result2 = client.connection_close_blocking();

    //And Connection is closed a third time
    let result3 = client.connection_close_blocking();

    //Then Only one logout request is sent
    let requests = server.received_requests().await.unwrap();
    let logout_count = requests.iter().filter(|r| is_logout_request(r)).count();
    assert_eq!(
        logout_count, 1,
        "Exactly one logout HTTP request should be sent"
    );

    //And No errors are thrown
    assert!(result1.is_ok(), "First close should succeed");
    assert!(result2.is_ok(), "Second close should succeed (idempotent)");
    assert!(result3.is_ok(), "Third close should succeed (idempotent)");
}

// ===========================================================================
//                        Concurrency
// ===========================================================================

#[tokio::test]
async fn should_handle_concurrent_close_calls_safely() {
    use std::sync::{Arc, Barrier};
    use std::thread;
    use std::time::Duration;

    //Given Snowflake client is logged in
    let server = MockServer::start().await;
    mount_jwt_login_success(&server).await;

    // Logout response with delay: thread 1 blocks on I/O while threads 2-5 race
    Mock::given(method("POST"))
        .and(path("/session"))
        .and(query_param("delete", "true"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_json(json!({ "success": true }))
                .insert_header("Content-Type", "application/json")
                .set_delay(Duration::from_millis(500)),
        )
        .mount(&server)
        .await;

    let server_uri = server.uri();
    let client = Arc::new(
        tokio::task::spawn_blocking(move || {
            SnowflakeTestClient::connect_integration_test(Some(&server_uri))
        })
        .await
        .unwrap(),
    );
    let barrier = Arc::new(Barrier::new(5));

    //When Connection is closed from multiple threads concurrently
    let handles: Vec<_> = (0..5)
        .map(|_| {
            let client_clone = Arc::clone(&client);
            let barrier_clone = Arc::clone(&barrier);
            thread::spawn(move || {
                barrier_clone.wait();
                client_clone.connection_close_blocking()
            })
        })
        .collect();

    let results: Vec<_> = handles
        .into_iter()
        .map(|h| h.join().expect("Thread should not panic"))
        .collect();

    //Then Only one logout request is sent
    let requests = server.received_requests().await.unwrap();
    let logout_count = requests.iter().filter(|r| is_logout_request(r)).count();
    assert_eq!(
        logout_count, 1,
        "Exactly one logout HTTP request despite 5 concurrent close() calls"
    );

    //And All close calls return successfully
    for result in results {
        assert!(
            result.is_ok(),
            "Concurrent close should succeed: {:?}",
            result.err()
        );
    }
}

// ===========================================================================
//                    Post-Logout Session Invalidation
// ===========================================================================

#[test]
fn should_reject_queries_client_side_after_connection_is_closed() {
    //Given Snowflake client is logged in
    let client = SnowflakeTestClient::connect_with_default_auth();

    //And Simple query SELECT 1 executes successfully
    let _result_before = client.execute_query("SELECT 1");

    //When Connection is closed
    let close_result = client.connection_close_blocking();
    assert!(close_result.is_ok(), "Close should succeed");

    //And Query is attempted on closed connection
    let result_after = client.execute_query_no_unwrap("SELECT 1");

    //Then The query fails with a connection-closed error
    assert!(
        result_after.is_err(),
        "Query should fail after close, but got: {:?}",
        result_after
    );

    let error_msg = result_after.unwrap_err();
    assert!(
        error_msg.contains("closed") || error_msg.contains("Closed"),
        "Error must mention connection is closed, got: {}",
        error_msg
    );
}

// ===========================================================================
//                          WireMock Helpers
// ===========================================================================

async fn mount_logout_success(server: &MockServer) {
    Mock::given(method("POST"))
        .and(path("/session"))
        .and(query_param("delete", "true"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_json(json!({ "success": true }))
                .insert_header("Content-Type", "application/json"),
        )
        .mount(server)
        .await;
}

fn is_logout_request(r: &wiremock::Request) -> bool {
    r.url.path() == "/session"
        && r.url
            .query()
            .map(|q| q.contains("delete=true"))
            .unwrap_or(false)
}
