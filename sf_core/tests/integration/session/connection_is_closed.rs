use sf_core::apis::database_driver_v1::{
    connection_close, connection_is_closed, connection_new, connection_release, database_init,
    database_new, database_release,
};
use sf_core::config::logout::LogoutConfig;
use std::time::Duration;

#[test]
fn test_connection_is_closed_initially_false() {
    // Create new connection
    let db_handle = database_new();
    database_init(db_handle).unwrap();
    let conn_handle = connection_new();

    // Initially, connection should not be closed
    let is_closed = connection_is_closed(conn_handle).unwrap();
    assert!(!is_closed, "New connection should not be closed");

    // Cleanup
    connection_release(conn_handle).unwrap();
    database_release(db_handle).unwrap();
}

#[test]
fn test_connection_is_closed_after_close() {
    // Create and initialize connection
    let db_handle = database_new();
    database_init(db_handle).unwrap();
    let conn_handle = connection_new();

    // Connection should not be closed initially
    assert!(!connection_is_closed(conn_handle).unwrap());

    // Close the connection
    let config = LogoutConfig {
        server_session_keep_alive: Some(true), // Skip logout for test
        enable_auto_detection: None,
        error_strategy: sf_core::config::logout::ErrorStrategy::BestEffort,
        timeout: Duration::from_secs(5),

        max_retry_attempts: None,
    };
    connection_close(conn_handle, config).unwrap();

    // Connection should now be closed
    let is_closed = connection_is_closed(conn_handle).unwrap();
    assert!(is_closed, "Connection should be closed after close()");

    // Cleanup
    connection_release(conn_handle).unwrap();
    database_release(db_handle).unwrap();

    let is_closed = connection_is_closed(conn_handle).unwrap();
    assert!(is_closed, "Connection should be closed after close()");
}

#[test]
fn test_connection_is_closed_idempotent() {
    // Create and initialize connection
    let db_handle = database_new();
    database_init(db_handle).unwrap();
    let conn_handle = connection_new();

    // Close the connection
    let config = LogoutConfig {
        server_session_keep_alive: Some(true),
        enable_auto_detection: None,
        error_strategy: sf_core::config::logout::ErrorStrategy::BestEffort,
        timeout: Duration::from_secs(5),

        max_retry_attempts: None,
    };
    connection_close(conn_handle, config.clone()).unwrap();

    // Verify closed
    assert!(connection_is_closed(conn_handle).unwrap());

    // Close again (should be idempotent)
    connection_close(conn_handle, config).unwrap();

    // Should still be closed
    assert!(connection_is_closed(conn_handle).unwrap());

    // Cleanup
    connection_release(conn_handle).unwrap();
    database_release(db_handle).unwrap();
}

#[test]
fn test_connection_is_closed_invalid_handle() {
    use sf_core::handle_manager::Handle;

    // Create invalid handle
    let invalid_handle = Handle {
        id: 99999,
        magic: 0,
    };

    // Should return error for invalid handle
    let result = connection_is_closed(invalid_handle);
    assert!(result.is_err(), "Should return error for invalid handle");
}
