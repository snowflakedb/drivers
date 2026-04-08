use sf_core::protobuf::apis::database_driver_v1::{
    DatabaseDriverClientBlockingExt, database_driver_client,
};
use sf_core::protobuf::generated::database_driver_v1::*;

fn setup() -> (
    impl DatabaseDriverClientBlockingExt,
    DatabaseHandle,
    ConnectionHandle,
) {
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
    (client, db_handle, conn_handle)
}

#[test]
fn test_connection_is_closed_initially_false() {
    let (client, db_handle, conn_handle) = setup();

    // Initially, connection should not be closed
    let is_closed = client
        .connection_is_closed_blocking(ConnectionIsClosedRequest {
            conn_handle: Some(conn_handle),
        })
        .unwrap()
        .is_closed;
    assert!(!is_closed, "New connection should not be closed");

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

#[test]
fn test_connection_is_closed_after_close() {
    let (client, db_handle, conn_handle) = setup();

    // Connection should not be closed initially
    assert!(
        !client
            .connection_is_closed_blocking(ConnectionIsClosedRequest {
                conn_handle: Some(conn_handle),
            })
            .unwrap()
            .is_closed
    );

    // Close the connection
    client
        .connection_close_blocking(ConnectionCloseRequest {
            conn_handle: Some(conn_handle),
        })
        .unwrap();

    // Connection should now be closed
    let is_closed = client
        .connection_is_closed_blocking(ConnectionIsClosedRequest {
            conn_handle: Some(conn_handle),
        })
        .unwrap()
        .is_closed;
    assert!(is_closed, "Connection should be closed after close()");

    // Verify idempotency - querying closed state multiple times works
    let is_closed_again = client
        .connection_is_closed_blocking(ConnectionIsClosedRequest {
            conn_handle: Some(conn_handle),
        })
        .unwrap()
        .is_closed;
    assert!(is_closed_again, "Connection should remain closed");

    // Cleanup - release handle (makes it invalid for further operations)
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

#[test]
fn test_connection_is_closed_idempotent() {
    let (client, db_handle, conn_handle) = setup();

    // Close the connection
    client
        .connection_close_blocking(ConnectionCloseRequest {
            conn_handle: Some(conn_handle),
        })
        .unwrap();

    // Verify closed
    assert!(
        client
            .connection_is_closed_blocking(ConnectionIsClosedRequest {
                conn_handle: Some(conn_handle),
            })
            .unwrap()
            .is_closed
    );

    // Close again (should be idempotent)
    client
        .connection_close_blocking(ConnectionCloseRequest {
            conn_handle: Some(conn_handle),
        })
        .unwrap();

    // Should still be closed
    assert!(
        client
            .connection_is_closed_blocking(ConnectionIsClosedRequest {
                conn_handle: Some(conn_handle),
            })
            .unwrap()
            .is_closed
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

#[test]
fn test_connection_is_closed_invalid_handle() {
    let client = database_driver_client();

    // Create invalid handle (ConnectionHandle with unknown id/magic)
    let invalid_handle = ConnectionHandle {
        id: 99999,
        magic: 0,
    };

    // Should return error for invalid handle
    let result = client.connection_is_closed_blocking(ConnectionIsClosedRequest {
        conn_handle: Some(invalid_handle),
    });
    assert!(result.is_err(), "Should return error for invalid handle");
}
