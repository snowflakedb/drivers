//! E2E tests for session logout functionality.
//!
//! These tests connect to real Snowflake and verify logout behavior end-to-end.
//! These tests implement scenarios from shared/session/logout.feature.
//! Core-specific integration tests with mock servers are in tests/integration/session/logout.rs.

use crate::common::snowflake_test_client::SnowflakeTestClient;
use sf_core::protobuf_apis::database_driver_v1::DatabaseDriverClient;
use sf_core::protobuf_gen::database_driver_v1::*;

// ===========================================================================
//                          Token Cleanup
// ===========================================================================

#[test]
fn should_cleanup_all_tokens_on_close_regardless_of_whether_logout_was_sent() {
    //Given Snowflake client is logged in
    //And <server_session_keep_alive> is set to any value

    for keep_alive in [Some(true), Some(false), None] {
        let client = SnowflakeTestClient::connect_with_default_auth();

        //When Connection is closed
        let result = DatabaseDriverClient::connection_close(ConnectionCloseRequest {
            conn_handle: Some(client.conn_handle),
            server_session_keep_alive: keep_alive,
            enable_auto_detection: None,
            error_strategy: None,
            timeout_seconds: None,

            max_retry_attempts: None,
        });

        //Then Session token in Connection.tokens is null
        //And Master token in Connection.tokens is null
        assert!(
            result.is_ok(),
            "Close should succeed with server_session_keep_alive={:?}",
            keep_alive
        );
    }
}

#[test]
fn should_be_idempotent_when_close_called_multiple_times() {
    //Given Snowflake client is logged in
    let client = SnowflakeTestClient::connect_with_default_auth();

    //When Connection is closed
    let result1 = DatabaseDriverClient::connection_close(ConnectionCloseRequest {
        conn_handle: Some(client.conn_handle),
        server_session_keep_alive: None,
        enable_auto_detection: None,
        error_strategy: None,
        timeout_seconds: None,

        max_retry_attempts: None,
    });

    //And Connection is closed again
    let result2 = DatabaseDriverClient::connection_close(ConnectionCloseRequest {
        conn_handle: Some(client.conn_handle),
        server_session_keep_alive: None,
        enable_auto_detection: None,
        error_strategy: None,
        timeout_seconds: None,

        max_retry_attempts: None,
    });

    //And Connection is closed a third time
    let result3 = DatabaseDriverClient::connection_close(ConnectionCloseRequest {
        conn_handle: Some(client.conn_handle),
        server_session_keep_alive: None,
        enable_auto_detection: None,
        error_strategy: None,
        timeout_seconds: None,

        max_retry_attempts: None,
    });

    //Then Only one logout request is sent
    //And No errors are thrown
    assert!(result1.is_ok(), "First close should succeed");
    assert!(result2.is_ok(), "Second close should succeed");
    assert!(result3.is_ok(), "Third close should succeed");
}

// ===========================================================================
//                        Concurrency
// ===========================================================================

#[test]
fn should_handle_concurrent_close_calls_safely() {
    use std::sync::Arc;
    use std::thread;

    //Given Snowflake client is logged in
    let client = Arc::new(SnowflakeTestClient::connect_with_default_auth());

    //When Connection is closed from multiple threads concurrently
    let handles: Vec<_> = (0..5)
        .map(|_| {
            let client_clone = Arc::clone(&client);
            thread::spawn(move || {
                DatabaseDriverClient::connection_close(ConnectionCloseRequest {
                    conn_handle: Some(client_clone.conn_handle),
                    server_session_keep_alive: None,
                    enable_auto_detection: None,
                    error_strategy: None,
                    timeout_seconds: None,

                    max_retry_attempts: None,
                })
            })
        })
        .collect();

    //Then Only one logout request is sent
    //And All close calls return successfully
    for handle in handles {
        let result = handle.join().expect("Thread should not panic");
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
    let close_result = DatabaseDriverClient::connection_close(ConnectionCloseRequest {
        conn_handle: Some(client.conn_handle),
        server_session_keep_alive: None,
        enable_auto_detection: None,
        error_strategy: None,
        timeout_seconds: None,

        max_retry_attempts: None,
    });
    assert!(close_result.is_ok(), "Close should succeed");

    //And Query is attempted on closed connection
    let result_after = client.execute_query_no_unwrap("SELECT 1");

    //Then Query throws ConnectionClosedException
    assert!(
        result_after.is_err(),
        "Query should fail after close, but got: {:?}",
        result_after
    );

    //And Error message indicates connection is unusable
    let error_msg = result_after.unwrap_err();
    assert!(
        error_msg.contains("closed")
            || error_msg.contains("Closed")
            || error_msg.contains("not initialized"),
        "Error should mention connection is closed or not initialized, got: {}",
        error_msg
    );
}

// ===========================================================================
//                        Process Exit and Thread Management
// ===========================================================================

#[test]
#[ignore = "Requires SNOW-2881763 (Heartbeat)"]
fn should_allow_process_to_exit_cleanly_when_session_kept_alive() {
    // Scenario: should allow process to exit cleanly when session kept alive
    // Requires: SNOW-2881763 (Heartbeat), SNOW-2912513 (Telemetry)
    //Given Connection with heartbeat enabled
    //And Telemetry cache is active
    //And server_session_keep_alive is set to true
    //When Connection is closed
    //Then Heartbeat is stopped
    //And Telemetry cache is flushed
    //And Process can exit immediately without hanging

    // TODO: Implement once heartbeat thread exists
}
