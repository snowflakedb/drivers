use scopeguard::defer;
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

/// A freshly-created connection must not be expired.
#[test]
fn test_connection_is_expired_initially_false() {
    let (client, db_handle, conn_handle) = setup();
    defer! {
        let _ = client.connection_release_blocking(ConnectionReleaseRequest {
            conn_handle: Some(conn_handle),
        });
        let _ = client.database_release_blocking(DatabaseReleaseRequest {
            db_handle: Some(db_handle),
        });
    }

    let is_expired = client
        .connection_is_expired_blocking(ConnectionIsExpiredRequest {
            conn_handle: Some(conn_handle),
        })
        .unwrap()
        .is_expired;
    assert!(!is_expired, "New connection should not be expired");
}

/// Closing a connection must NOT set the expired flag — expired and closed
/// are orthogonal states.
#[test]
fn test_connection_is_expired_not_set_by_close() {
    let (client, db_handle, conn_handle) = setup();
    defer! {
        let _ = client.connection_release_blocking(ConnectionReleaseRequest {
            conn_handle: Some(conn_handle),
        });
        let _ = client.database_release_blocking(DatabaseReleaseRequest {
            db_handle: Some(db_handle),
        });
    }

    client
        .connection_close_blocking(ConnectionCloseRequest {
            conn_handle: Some(conn_handle),
        })
        .unwrap();

    let is_expired = client
        .connection_is_expired_blocking(ConnectionIsExpiredRequest {
            conn_handle: Some(conn_handle),
        })
        .unwrap()
        .is_expired;
    assert!(
        !is_expired,
        "Closing a connection must not set the expired flag"
    );
}

/// Querying expired state for an invalid handle must return an error.
#[test]
fn test_connection_is_expired_invalid_handle() {
    let client = database_driver_client();

    let invalid_handle = ConnectionHandle {
        id: 99999,
        magic: 0,
    };

    let result = client.connection_is_expired_blocking(ConnectionIsExpiredRequest {
        conn_handle: Some(invalid_handle),
    });
    assert!(result.is_err(), "Should return error for invalid handle");
}
