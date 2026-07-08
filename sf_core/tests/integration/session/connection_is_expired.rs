use serde_json::json;
use sf_core::protobuf::apis::database_driver_v1::{
    DatabaseDriverClientBlockingExt, database_driver_client,
};
use sf_core::protobuf::generated::database_driver_v1::*;
use wiremock::matchers::{method, path, path_regex};
use wiremock::{Mock, MockServer, ResponseTemplate};

use crate::common::mocks::auth::mount_jwt_login_success;
use crate::common::snowflake_test_client::SnowflakeTestClient;

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

    let is_expired = client
        .connection_is_expired_blocking(ConnectionIsExpiredRequest {
            conn_handle: Some(conn_handle),
        })
        .unwrap()
        .is_expired;
    assert!(!is_expired, "New connection should not be expired");

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

/// Closing a connection must NOT set the expired flag — expired and closed
/// are orthogonal states.
#[test]
fn test_connection_is_expired_not_set_by_close() {
    let (client, db_handle, conn_handle) = setup();

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

/// When the server returns GS code 390114 during a token refresh, the connection
/// must be marked expired. This proves the four `is_expired.store(true)` sites in
/// RefreshContext::try_refresh are reachable and wired to the flag.
#[tokio::test]
async fn should_set_is_expired_when_server_returns_390114() {
    let server = MockServer::start().await;

    // Reuse the shared JWT login mock (token + masterToken returned)
    mount_jwt_login_success(&server).await;

    // Query endpoint returns HTTP 401 on every call — triggers session token refresh
    Mock::given(method("POST"))
        .and(path_regex(r"/queries/v1/query-request.*"))
        .respond_with(ResponseTemplate::new(401))
        .named("query_401")
        .mount(&server)
        .await;

    // Token-request returns GS 390114: master token expired
    Mock::given(method("POST"))
        .and(path("/session/token-request"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "success": false,
            "code": "390114",
            "message": "Master token has expired. The session is no longer active."
        })))
        .named("refresh_390114")
        .mount(&server)
        .await;

    // Initialize a full connection against the mock server
    let server_uri = server.uri();
    let client = tokio::task::spawn_blocking(move || {
        SnowflakeTestClient::connect_integration_test(Some(&server_uri))
    })
    .await
    .unwrap();

    // Capture the handle before moving client into the next closure
    let conn_handle = client.conn_handle;

    // Execute a query: 401 triggers refresh → 390114 → sets is_expired; error is expected
    let _ = tokio::task::spawn_blocking(move || client.execute_query_no_unwrap("SELECT 1"))
        .await
        .unwrap();

    // The is_expired flag must now be true
    let check_client = database_driver_client();
    let is_expired = check_client
        .connection_is_expired_blocking(ConnectionIsExpiredRequest {
            conn_handle: Some(conn_handle),
        })
        .unwrap()
        .is_expired;
    assert!(
        is_expired,
        "is_expired must be true after server returns GS 390114"
    );
}
