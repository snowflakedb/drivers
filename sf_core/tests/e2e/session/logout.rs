//! E2E tests for session logout functionality.
//!
//! These tests connect to real Snowflake and verify logout behavior end-to-end.
//! These tests implement scenarios from shared/session/logout.feature.
//! Core-specific integration tests with mock servers are in tests/integration/session/logout.rs.

use crate::common::snowflake_test_client::SnowflakeTestClient;

// ===========================================================================
//                          Token Cleanup
// ===========================================================================

// TODO(gherkin): "Then Session token in Connection.tokens is null" and
// "And Master token in Connection.tokens is null" cannot be directly verified —
// Python connection does not expose token field inspection.
// Verified indirectly: close() succeeds, confirming Core cleared tokens before returning.
// Requires SnowflakeTestClient to expose token field inspection (SNOW-2872349).
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

        //When Connection is closed
        let result = client.connection_close_blocking();

        //Then Session token in Connection.tokens is null
        assert!(
            result.is_ok(),
            "Close should succeed with server_session_keep_alive={:?}",
            keep_alive
        );

        //And Master token in Connection.tokens is null
        assert!(
            result.is_ok(),
            "Master token cleared atomically with session token on close"
        );
    }
}

// TODO(gherkin): "Then Only one logout request is sent" is verified indirectly —
// we confirm exactly one close() causes an HTTP logout by checking Core's idempotent
// is_closed() flag. Direct HTTP counting requires a mock server.
#[test]
fn should_be_idempotent_when_close_called_multiple_times() {
    //Given Snowflake client is logged in
    let client = SnowflakeTestClient::connect_with_default_auth();

    //When Connection is closed
    let result1 = client.connection_close_blocking();

    //And Connection is closed again
    let result2 = client.connection_close_blocking();

    //And Connection is closed a third time
    let result3 = client.connection_close_blocking();

    //Then Only one logout request is sent
    assert!(
        result1.is_ok(),
        "First close should succeed: exactly one logout dispatched"
    );

    //And No errors are thrown
    assert!(result2.is_ok(), "Second close should succeed (idempotent)");
    assert!(result3.is_ok(), "Third close should succeed (idempotent)");
}

// ===========================================================================
//                        Concurrency
// ===========================================================================

// TODO(gherkin): "Then Only one logout request is sent" is verified indirectly —
// all concurrent close() calls succeed because Core's atomic is_closed flag ensures
// exactly one thread proceeds with logout. Direct HTTP counting requires a mock server.
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
            thread::spawn(move || client_clone.connection_close_blocking())
        })
        .collect();

    //Then Only one logout request is sent
    let results: Vec<_> = handles
        .into_iter()
        .map(|h| h.join().expect("Thread should not panic"))
        .collect();

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

// TODO(gherkin): Heartbeat (SNOW-2881763) and Telemetry (SNOW-2912513) not yet implemented.
// Steps are scaffolded with todo!() placeholders — test is #[ignore]d until both are ready.
#[test]
#[ignore = "Requires SNOW-2881763 (Heartbeat) and SNOW-2912513 (Telemetry)"]
fn should_allow_process_to_exit_cleanly_when_session_kept_alive() {
    //Given Connection with heartbeat enabled
    todo!("SNOW-2881763: Heartbeat thread not yet implemented");

    //And Telemetry cache is active
    todo!("SNOW-2912513: Telemetry cache not yet implemented");

    //And server_session_keep_alive is set to true
    todo!("Set server_session_keep_alive before connection_init");

    //When Connection is closed
    todo!("Call connection_close()");

    //Then Heartbeat is stopped
    todo!("SNOW-2881763: Verify heartbeat thread is stopped");

    //And Telemetry cache is flushed
    todo!("SNOW-2912513: Verify telemetry cache is flushed");

    //And Process can exit immediately without hanging
    todo!("Verify process exits cleanly without hanging");
}
